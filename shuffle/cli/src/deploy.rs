// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::utils::send;
use anyhow::{anyhow, bail, format_err, Context, Result};
use diem_config::config::NodeConfig;
use diem_crypto::PrivateKey;
use diem_sdk::{
    client::BlockingClient,
    transaction_builder::TransactionFactory,
    types::{
        account_address::AccountAddress,
        transaction::{Module, TransactionPayload},
        LocalAccount,
    },
};
use diem_temppath::TempPath;
use diem_types::{
    account_state::AccountState, account_state_blob::AccountStateBlob, chain_id::ChainId,
    transaction::authenticator::AuthenticationKey,
};
use generate_key::load_key;
use move_binary_format::{file_format::CompiledModule, normalized};
use move_command_line_common::files::MOVE_EXTENSION;
use std::{
    convert::TryFrom,
    fs, io,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

pub fn handle(project_dir: PathBuf, account_key_path: PathBuf) -> Result<()> {
    let config_path = project_dir.join("nodeconfig/0").join("node.yaml");
    let config = NodeConfig::load(&config_path)
        .with_context(|| format!("Failed to load NodeConfig from file: {:?}", config_path))?;
    let new_account_key = load_key(account_key_path);
    let json_rpc_url = format!("http://0.0.0.0:{}", config.json_rpc.address.port());
    let factory = TransactionFactory::new(ChainId::test());
    println!("Connecting to {}...", json_rpc_url);

    let client = BlockingClient::new(json_rpc_url);

    let mut new_account = LocalAccount::new(
        AuthenticationKey::ed25519(&new_account_key.public_key()).derived_address(),
        new_account_key,
        0,
    );
    // ================= Send a module transaction ========================
    print!("Add a module to user account...");

    // Get the path to the Move stdlib sources
    let move_stdlib_dir = move_stdlib::move_stdlib_modules_full_path();
    let diem_framework_dir = diem_framework::diem_core_modules_full_path();
    let module_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(project_dir.clone())
        .join(project_dir)
        .join("sources/SampleUserModule.move")
        .canonicalize()?;
    let copied_module_path =
        copy_file_with_sender_address(&module_path, new_account.address()).unwrap();
    let unwrapped_module_path = copied_module_path.to_str().unwrap();
    let compiled_module = compile_program(
        unwrapped_module_path,
        &[move_stdlib_dir.as_str(), diem_framework_dir.as_str()],
    )?;

    let publish_txn = new_account.sign_with_transaction_builder(
        factory.payload(TransactionPayload::Module(Module::new(compiled_module))),
    );

    send(&client, publish_txn)?;
    println!("Success!");

    // ================= Get modules in the account  ========================

    let account_state_blob: AccountStateBlob = {
        let blob = client
            .get_account_state_with_proof(new_account.address(), None, None)
            .unwrap()
            .into_inner()
            .blob
            .ok_or_else(|| anyhow::anyhow!("missing account state blob"))?;
        bcs::from_bytes(&blob)?
    };
    let account_state = AccountState::try_from(&account_state_blob)?;
    let mut modules = vec![];
    for module_bytes in account_state.get_modules() {
        modules.push(normalized::Module::new(
            &CompiledModule::deserialize(module_bytes)
                .map_err(|e| anyhow!("Failure deserializing module: {:?}", e))?,
        ));
    }
    println!("move modules length: {}", modules.len());
    println!("move modules name: {}", modules[0].name);

    Ok(())
}

/// Compile Move program
pub fn compile_program(file_path: &str, dependency_paths: &[&str]) -> Result<Vec<u8>> {
    let tmp_output_dir = TempPath::new();
    tmp_output_dir
        .create_as_dir()
        .expect("error creating temporary output directory");
    let tmp_output_path = tmp_output_dir.as_ref().display().to_string();

    let mut command = Command::new("cargo");
    command
        .args(&["run", "-p", "move-lang", "--bin", "move-build", "--"])
        .arg(file_path)
        .args(&["-o", &tmp_output_path]);

    for dep in dependency_paths {
        command.args(&["-d", dep]);
    }
    for (name, addr) in diem_framework::diem_framework_named_addresses() {
        command.args(&["-a", &format!("{}=0x{:#X}", name, addr)]);
    }

    let output = command.output()?;
    if !output.status.success() {
        return Err(format_err!("compilation failed"));
    }

    let mut output_files = walkdir::WalkDir::new(tmp_output_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            e.file_type().is_file()
                && path
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|ext| ext == "mv")
                    .unwrap_or(false)
        })
        .filter_map(|e| e.path().to_str().map(|s| s.to_string()))
        .collect::<Vec<_>>();
    if output_files.is_empty() {
        bail!("compiler failed to produce an output file")
    }

    let compiled_program = if output_files.len() != 1 {
        bail!("compiler output has more than one file")
    } else {
        fs::read(output_files.pop().unwrap())?
    };

    Ok(compiled_program)
}

fn copy_file_with_sender_address(file_path: &Path, sender: AccountAddress) -> io::Result<PathBuf> {
    let tmp_source_path = TempPath::new().as_ref().with_extension(MOVE_EXTENSION);
    let mut tmp_source_file = std::fs::File::create(tmp_source_path.clone())?;
    let mut code = fs::read_to_string(file_path)?;
    code = code.replace("{{sender}}", &format!("0x{}", sender));
    writeln!(tmp_source_file, "{}", code)?;
    Ok(tmp_source_path)
}
