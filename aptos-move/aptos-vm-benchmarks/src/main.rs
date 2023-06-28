// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod helper;
use aptos::move_tool::MemberId;
use aptos_framework::BuildOptions;
use aptos_framework::BuiltPackage;
use aptos_language_e2e_tests::{account::Account, executor::FakeExecutor};
use aptos_types::transaction::{EntryFunction, TransactionPayload};
use clap::Parser;
use move_binary_format::CompiledModule;
use std::fs::read_dir;
use std::path::PathBuf;

const PREFIX: &str = "benchmark";

#[derive(Parser, Debug)]
struct Cli {
    #[clap(default_value = "")]
    pattern: String,
}

fn main() {
    //// Implement CLI
    let args = Cli::parse();
    let pattern = &args.pattern;

    //// Discover all top-level packages in samples directory
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("samples");
    let dirs = read_dir(path.as_path()).unwrap();

    //// Setting up local execution environment once
    // disable parallel execution
    let executor = FakeExecutor::from_head_genesis();
    let mut executor = executor.set_not_parallel();

    //// Go over all Move projects
    for dir in dirs {
        let entry = dir.unwrap();
        if !entry.path().is_dir() {
            continue;
        }
        let dir_path = entry.path();

        let build_options = BuildOptions {
            with_srcs: true,
            with_abis: true,
            with_source_maps: true,
            with_error_map: true,
            ..BuildOptions::default()
        };
        let package =
            BuiltPackage::build(dir_path, build_options).expect("build package must succeed");

        let codes = package.extract_code();
        for code in codes {
            // avoid SEQUENCE_NUMBER_TOO_NEW error
            let mut sequence_num_counter = 0;

            let compiled_module = CompiledModule::deserialize(&code).unwrap();

            // get module address
            let mut module_bytes = vec![];
            compiled_module.serialize(&mut module_bytes).unwrap();
            let module_id = compiled_module.self_id();
            let address = &module_id.address();
            let identifier = &module_id.name().as_str();

            // find non-entry functions and ignore them
            // keep entry function names in func_identifiers vector
            let funcs = compiled_module.function_defs;
            let func_handles = compiled_module.function_handles;
            let func_identifier_pool = compiled_module.identifiers;

            // find # of params in each func if it is entry function
            let signature_pool = compiled_module.signatures;

            let mut func_identifiers = Vec::new();
            for func in funcs {
                let is_entry = func.is_entry;
                if !is_entry {
                    continue;
                }

                let func_idx: usize = func.function.0.into();
                let handle = &func_handles[func_idx];

                let func_identifier_idx: usize = handle.name.0.into();
                let func_identifier = &func_identifier_pool[func_identifier_idx];

                // skip if it doesn't start with "benchmark"
                let func_name = func_identifier.as_str();
                if !func_name.starts_with(PREFIX) {
                    continue;
                }

                // skip if it doesn't match pattern
                let fully_qualified_path = format!("{}::{}::{}", address, identifier, func_name);
                if !fully_qualified_path.contains(pattern) && !pattern.is_empty() {
                    continue;
                }

                // if it does, ensure no params in benchmark function
                let signature_idx: usize = handle.parameters.0.into();
                let func_params = &signature_pool[signature_idx];
                if func_params.len() != 0 {
                    eprintln!(
                        "\n[WARNING] benchmark function should not have parameters: {}\n",
                        func_name,
                    );
                    // TODO: should we exit instead of continuing with the benchmark
                    continue;
                }

                func_identifiers.push(func_name);
            }

            //// publish test-package under module address
            let creator = executor.new_account_at(**address);

            for func_identifier in func_identifiers {
                println!(
                    "Executing {}::{}::{}",
                    address.to_string(),
                    identifier,
                    func_identifier,
                );

                // publish package similar to create_publish_package in harness.rs
                let module_payload = helper::generate_module_payload(&package);
                print!("Signing txn for module... ");
                helper::sign_module_txn(
                    &mut executor,
                    &creator,
                    module_payload,
                    sequence_num_counter,
                );
                sequence_num_counter = sequence_num_counter + 1;

                //// send a txn that invokes the entry function 0x{address}::{name}::benchmark
                print!("Signing user txn... ");
                let MemberId {
                    module_id,
                    member_id: _function_id,
                } = str::parse(&format!(
                    "0x{}::{}::{}",
                    address.to_string(),
                    identifier,
                    func_identifier,
                ))
                .unwrap();
                helper::sign_user_txn(&mut executor, &module_id, func_identifier);
            }
        }
    }
}
