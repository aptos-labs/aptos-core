// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! A tool for interacting with Move
//!
//! TODO: Examples
//!

use crate::{
    common::{
        types::{EncodingOptions, MovePackageDir, NodeOptions, PrivateKeyInputOptions},
        utils::to_common_result,
    },
    CliResult, Error,
    Error::CommandArgumentError,
};
use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey};
use aptos_rest_client::{Client, Transaction};
use aptos_sdk::{transaction_builder::TransactionFactory, types::LocalAccount};
use aptos_types::{
    chain_id::ChainId,
    transaction::{
        authenticator::AuthenticationKey, ModuleBundle, ScriptFunction, TransactionPayload,
    },
};
use aptos_vm::natives::aptos_natives;
use clap::{Parser, Subcommand};
use move_cli::package::cli::{run_move_unit_tests, UnitTestResult};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};
use move_package::{compilation::compiled_package::CompiledPackage, BuildConfig};
use move_unit_test::UnitTestingConfig;
use reqwest::Url;
use std::{path::Path, str::FromStr};

/// CLI tool for performing Move tasks
///
#[derive(Subcommand)]
pub enum MoveTool {
    Compile(CompilePackage),
    Publish(PublishPackage),
    Run(RunFunction),
    Test(TestPackage),
}

impl MoveTool {
    pub async fn execute(self) -> CliResult {
        match self {
            MoveTool::Compile(tool) => to_common_result(tool.execute().await),
            MoveTool::Publish(tool) => to_common_result(tool.execute().await),
            MoveTool::Run(tool) => to_common_result(tool.execute().await),
            MoveTool::Test(tool) => to_common_result(tool.execute().await),
        }
    }
}

/// Compiles a package and returns the [`ModuleId`]s
#[derive(Parser)]
pub struct CompilePackage {
    #[clap(flatten)]
    move_options: MovePackageDir,
}

impl CompilePackage {
    pub async fn execute(&self) -> Result<Vec<String>, Error> {
        let build_config = BuildConfig {
            generate_docs: true,
            install_dir: self.move_options.output_dir.clone(),
            ..Default::default()
        };
        let compiled_package = compile_move(build_config, self.move_options.package_dir.as_path())?;
        let mut ids = Vec::new();
        compiled_package
            .compiled_modules()
            .iter_modules()
            .iter()
            .for_each(|module| ids.push(module.self_id().to_string()));
        Ok(ids)
    }
}

/// Run Move unit tests against a package path
#[derive(Parser)]
pub struct TestPackage {
    #[clap(flatten)]
    move_options: MovePackageDir,
}

impl TestPackage {
    pub async fn execute(&self) -> Result<&'static str, Error> {
        let config = BuildConfig {
            test_mode: true,
            install_dir: self.move_options.output_dir.clone(),
            ..Default::default()
        };
        let result = run_move_unit_tests(
            self.move_options.package_dir.as_path(),
            config,
            UnitTestingConfig::default_with_bound(Some(100_000)),
            aptos_natives(),
            false,
        )
        .map_err(|err| Error::UnexpectedError(err.to_string()))?;

        // TODO: commit back up to the move repo
        match result {
            UnitTestResult::Success => Ok("Success"),
            UnitTestResult::Failure => Ok("Failure"),
        }
    }
}

/// Compiles a Move package dir, and returns the compiled modules.
fn compile_move(build_config: BuildConfig, package_dir: &Path) -> Result<CompiledPackage, Error> {
    // TODO: Add caching
    build_config
        .compile_package(package_dir, &mut Vec::new())
        .map_err(|err| Error::MoveCompiliationError(err.to_string()))
}

/// Publishes the modules in a Move package
#[derive(Parser)]
pub struct PublishPackage {
    #[clap(flatten)]
    encoding_options: EncodingOptions,
    #[clap(flatten)]
    private_key_options: PrivateKeyInputOptions,
    #[clap(flatten)]
    move_options: MovePackageDir,
    #[clap(flatten)]
    node_options: NodeOptions,
    #[clap(long)]
    chain_id: ChainId,
    #[clap(long, default_value_t = 1000)]
    max_gas: u64,
}

impl PublishPackage {
    pub async fn execute(&self) -> Result<aptos_rest_client::Transaction, Error> {
        let build_config = BuildConfig {
            generate_abis: false,
            generate_docs: true,
            install_dir: self.move_options.output_dir.clone(),
            ..Default::default()
        };
        let package = compile_move(build_config, self.move_options.package_dir.as_path())?;
        let compiled_units: Vec<Vec<u8>> = package
            .compiled_units
            .iter()
            .map(|unit_with_source| unit_with_source.unit.serialize())
            .collect();
        let compiled_payload = TransactionPayload::ModuleBundle(ModuleBundle::new(compiled_units));

        // Now that it's compiled, lets send it
        let sender_key = self
            .private_key_options
            .extract_private_key(self.encoding_options.encoding)?;

        submit_transaction(
            self.node_options.url.clone(),
            self.chain_id,
            sender_key,
            compiled_payload,
            self.max_gas,
        )
        .await
    }
}

/// Run a Move function
#[derive(Parser)]
pub struct RunFunction {
    #[clap(flatten)]
    encoding_options: EncodingOptions,
    #[clap(flatten)]
    private_key_options: PrivateKeyInputOptions,
    #[clap(flatten)]
    node_options: NodeOptions,
    /// Chain id of network being used
    #[clap(long)]
    chain_id: ChainId,
    /// Maximum gas willing to be spent on this transaction.
    #[clap(long, default_value_t = 1000)]
    max_gas: u64,
    /// Function name as `<ADDRESS>::<MODULE_ID>::<FUNCTION_NAME>`
    ///
    /// Example: `0x842ed41fad9640a2ad08fdd7d3e4f7f505319aac7d67e1c0dd6a7cce8732c7e3::Message::set_message`
    #[clap(long)]
    function_id: String,
    /// Hex encoded arguments separated by spaces.
    ///
    /// Example: `0x01 0x02 0x03`
    args: Vec<String>,
}

impl RunFunction {
    pub async fn execute(&self) -> Result<aptos_rest_client::Transaction, Error> {
        let ids: Vec<&str> = self.function_id.split_terminator("::").collect();
        if ids.len() != 3 {
            return Err(CommandArgumentError(
                "--function-id is not well formed.  Must be of <address>::<module>::<function>"
                    .to_string(),
            ));
        }

        let address = AccountAddress::from_hex_literal(ids.get(0).unwrap()).unwrap();
        let module = Identifier::from_str(ids.get(1).unwrap()).unwrap();
        let function = Identifier::from_str(ids.get(2).unwrap()).unwrap();
        let module_id = ModuleId::new(address, module);

        // TODO: Support type args
        // TODO: Take in arguments from a file instead?
        let args: Vec<Vec<u8>> = self
            .args
            .iter()
            .map(|str| {
                if let Some(stripped_hex) = str.strip_prefix("0x") {
                    hex::decode(stripped_hex).unwrap()
                } else {
                    hex::decode(str).unwrap()
                }
            })
            .collect();

        let script_function = ScriptFunction::new(module_id, function, Vec::new(), args);
        println!("Encoded: {}", hex::encode("Hi Greg"));
        println!("{:?}", serde_json::to_string(&script_function));
        // Now that it's compiled, lets send it
        let sender_key = self
            .private_key_options
            .extract_private_key(self.encoding_options.encoding)?;
        submit_transaction(
            self.node_options.url.clone(),
            self.chain_id,
            sender_key,
            TransactionPayload::ScriptFunction(script_function),
            self.max_gas,
        )
        .await
    }
}

/// Submits a [`TransactionPayload`] as signed by the `sender_key`
async fn submit_transaction(
    url: Url,
    chain_id: ChainId,
    sender_key: Ed25519PrivateKey,
    payload: TransactionPayload,
    max_gas: u64,
) -> Result<Transaction, Error> {
    let client = Client::new(url);

    // Get sender address
    let sender_address = AuthenticationKey::ed25519(&sender_key.public_key()).derived_address();
    let sender_address = AccountAddress::new(*sender_address);

    // Get account to get the sequence number
    let account_response = client
        .get_account(sender_address)
        .await
        .map_err(|err| Error::UnexpectedError(err.to_string()))?;
    let account = account_response.inner();
    let sequence_number = account.sequence_number;

    let transaction_factory = TransactionFactory::new(chain_id)
        .with_gas_unit_price(1)
        .with_max_gas_amount(max_gas);
    let sender_account = &mut LocalAccount::new(sender_address, sender_key, sequence_number);
    let transaction =
        sender_account.sign_with_transaction_builder(transaction_factory.payload(payload));
    let response = client
        .submit_and_wait(&transaction)
        .await
        .map_err(|err| Error::UnexpectedError(err.to_string()))?;

    Ok(response.inner().clone())
}
