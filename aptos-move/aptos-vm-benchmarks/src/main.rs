// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod helper;
use aptos::move_tool::MemberId;
use aptos_framework::BuildOptions;
use aptos_framework::BuiltPackage;
use aptos_language_e2e_tests::executor::FakeExecutor;
use clap::Parser;
use move_binary_format::CompiledModule;
use std::fs::read_dir;
use std::path::PathBuf;

//// CLI options
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

    //// get all paths for Move projects
    let dir_paths = helper::get_dir_paths(dirs);

    //// Go over all Move projects
    for dir_path in dir_paths {
        // configure and build Move package
        let build_options = BuildOptions {
            with_srcs: true,
            with_abis: true,
            with_source_maps: true,
            with_error_map: true,
            ..BuildOptions::default()
        };
        let package =
            BuiltPackage::build(dir_path, build_options).expect("build package must succeed");

        // iterate over all Move package code
        let codes = package.extract_code();
        for code in codes {
            // avoid SEQUENCE_NUMBER_TOO_NEW error
            let mut sequence_num_counter = 0;

            let compiled_module = CompiledModule::deserialize(&code).unwrap();
            let module_id = compiled_module.self_id();
            let identifier = module_id.name().to_string();

            //// get module address
            let address = module_id.address();

            //// get all benchmark tagged functions
            let func_identifiers = helper::get_functional_identifiers(
                compiled_module,
                identifier.clone(),
                *address,
                pattern.clone(),
            );

            //// publish test-package under module address
            let creator = executor.new_account_at(*address);

            //// iterate over all the functions that satisfied the requirements above
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
                helper::sign_user_txn(&mut executor, &module_id, func_identifier.as_str());
            }
        }
    }
}
