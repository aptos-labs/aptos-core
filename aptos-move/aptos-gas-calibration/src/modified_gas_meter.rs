// Copyright © Aptos Foundation

use crate::benchmark_helpers::{
    generate_module_payload, get_dir_paths, get_functional_identifiers, get_module_name,
    sign_module_txn,
};
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_gas_algebra::Expression;
use aptos_language_e2e_tests::executor::FakeExecutor;
use move_binary_format::CompiledModule;
use move_core_types::account_address::AccountAddress;
use std::{
    fs::read_dir,
    path::PathBuf,
    //sync::{Arc, Mutex},
};

/*
 * @notice: Executes Calibration Function to get Abstract Gas Usage
 * @param package: Compiled module code
 * @param executor: Modified FakeExecutor with a SafeNativeBuilder
 * @param func_identifiers: All the function names
 * @param address: Module address to publish under
 * @param identifier: Name of module
 */
pub fn record_abstract_gas_usage(
    package: &BuiltPackage,
    executor: &mut FakeExecutor,
    func_identifiers: Vec<String>,
    address: AccountAddress,
    identifier: &String,
) -> Vec<Vec<Expression>> {
    let mut formulae: Vec<Vec<Expression>> = Vec::new();

    //// publish test-package under module address
    let creator = executor.new_account_at(address);

    //// iterate over all the functions that satisfied the requirements above
    for (sequence_num_counter, func_identifier) in func_identifiers.into_iter().enumerate() {
        println!(
            "Executing {}::{}::{}",
            address,
            identifier,
            func_identifier.clone(),
        );

        // publish package similar to create_publish_package in harness.rs
        print!("Signing txn for module... ");
        let module_payload = generate_module_payload(package);
        let counter = sequence_num_counter.try_into().unwrap();
        sign_module_txn(executor, &creator, module_payload, counter);

        println!("recording abstract gas usage...");
        let module_name = get_module_name(address, identifier, &func_identifier);
        let gas_formula = executor.exec_abstract_usage(&module_name, &func_identifier, vec![], vec![]);
        formulae.push(gas_formula);
    }

    formulae
}

pub fn get_abstract_gas_usage() -> Vec<Vec<Expression>> {
    //// Discover all top-level packages in samples directory
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("samples");
    let dirs = read_dir(path.as_path()).unwrap();

    let executor = FakeExecutor::from_head_genesis();
    let mut executor = executor.set_not_parallel();

    //// get all paths for Move projects
    let dir_paths = get_dir_paths(dirs);

    let mut abstract_formulae: Vec<Vec<Expression>> = Vec::new();

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
            let compiled_module = CompiledModule::deserialize(&code).unwrap();
            let module_id = compiled_module.self_id();
            let identifier = &module_id.name().to_string();

            //// get module address
            let address = module_id.address();

            //// get all benchmark tagged functions
            let func_identifiers =
                get_functional_identifiers(compiled_module, identifier, *address, String::new());

            //// publish module and sign user transaction
            //// the benchmark happens when in signing user txn
            let gas_formulae = record_abstract_gas_usage(
                &package,
                &mut executor,
                func_identifiers,
                *address,
                identifier,
            );
            abstract_formulae.extend(gas_formulae);
        }
    }
    abstract_formulae
}
