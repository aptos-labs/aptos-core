// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_cached_packages::aptos_stdlib;
use aptos_framework::BuiltPackage;
use aptos_language_e2e_tests::{
    account::Account,
    executor::{ExecFuncTimerDynamicArgs, FakeExecutor, GasMeterType},
};
use aptos_types::{move_utils::MemberId, transaction::TransactionPayload};
use move_binary_format::CompiledModule;
use move_core_types::{account_address::AccountAddress, language_storage::ModuleId};
use std::{fs::ReadDir, path::PathBuf, string::String, time::Instant};

// CONSTANTS
const PREFIX: &str = "benchmark";

// generate a TransactionPayload for modules
pub fn generate_module_payload(package: &BuiltPackage) -> TransactionPayload {
    // extract package data
    let code = package.extract_code();
    let metadata = package
        .extract_metadata()
        .expect("extracting package metadata must succeed");

    // publish package similar to create_publish_package in harness.rs
    aptos_stdlib::code_publish_package_txn(
        bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
        code,
    )
}

// sign transaction to create a Module and return transaction status
pub fn execute_module_txn(
    executor: &mut FakeExecutor,
    account: &Account,
    payload: TransactionPayload,
    sequence_number: u64,
) {
    // build and sign transaction
    let sign_tx = account
        .transaction()
        .sequence_number(sequence_number)
        .max_gas_amount(2_000_000)
        .gas_unit_price(200)
        .payload(payload)
        .sign();

    // Restart timer and sequence counter for each new package
    // only count running time of entry function
    let start = Instant::now();
    let txn_output = executor.execute_transaction(sign_tx);

    // apply write set to avoid LINKER_ERROR
    executor.apply_write_set(txn_output.write_set());
    let elapsed = start.elapsed();
    println!("running time (microseconds): {}", elapsed.as_micros());

    // validate successful transaction
    let txn_status = txn_output.status().to_owned();
    assert!(txn_output.status().status().unwrap().is_success());
    println!("txn status: {:?}", txn_status);
}

// sign user transaction and only records the body of the transaction
pub fn execute_user_txn(executor: &mut FakeExecutor, module_name: &ModuleId, function_name: &str) {
    let elapsed = executor
        .exec_func_record_running_time(
            module_name,
            function_name,
            vec![],
            vec![],
            10,
            ExecFuncTimerDynamicArgs::NoArgs,
            GasMeterType::UnmeteredGasMeter,
        )
        .elapsed_micros();
    println!("running time (microseconds): {}", elapsed);
}

// publish module under user and sign user transaction
pub fn publish(
    package: &BuiltPackage,
    executor: &mut FakeExecutor,
    func_identifiers: Vec<String>,
    address: AccountAddress,
    identifier: &String,
) {
    //// publish test-package under module address
    // TODO[Orderless]: Check if this needs modification to accommodate stateless accounts
    let creator = executor.new_account_at(address, Some(0));

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
        execute_module_txn(executor, &creator, module_payload, counter);

        //// send a txn that invokes the entry function 0x{address}::{name}::benchmark
        print!("Signing user txn... ");
        let module_name = get_module_name(address, identifier, &func_identifier);
        execute_user_txn(executor, &module_name, &func_identifier);
    }
}

/*
 *
 * GETTER FUNCTIONS
 *
 */
// get module name
pub fn get_module_name(
    address: AccountAddress,
    identifier: &String,
    func_identifier: &String,
) -> ModuleId {
    let MemberId {
        module_id,
        member_id: _function_id,
    } = str::parse(&format!(
        "0x{}::{}::{}",
        address.to_hex(),
        identifier,
        func_identifier,
    ))
    .unwrap();

    module_id
}

// get all directories of Move projects
pub fn get_dir_paths(dirs: ReadDir) -> Vec<PathBuf> {
    let mut dir_paths = Vec::new();
    for dir in dirs {
        // validate path is directory
        let entry = dir.unwrap();
        if !entry.path().is_dir() {
            continue;
        }
        dir_paths.push(entry.path());
    }
    dir_paths
}

// get functional identifiers
pub fn get_functional_identifiers(
    cm: CompiledModule,
    identifier: &String,
    address: AccountAddress,
    pattern: String,
) -> Vec<String> {
    // find non-entry functions and ignore them
    // keep entry function names in func_identifiers vector
    let funcs = cm.function_defs;
    let func_handles = cm.function_handles;
    let func_identifier_pool = cm.identifiers;

    // find # of params in each func if it is entry function
    let signature_pool = cm.signatures;

    let mut func_identifiers: Vec<String> = Vec::new();
    for func in funcs {
        // check if function is marked as entry, if not skip it
        let is_entry = func.is_entry;
        if !is_entry {
            continue;
        }

        // extract some info from the function
        let func_idx: usize = func.function.0.into();
        let handle = &func_handles[func_idx];
        let func_identifier_idx: usize = handle.name.0.into();
        let func_identifier = &func_identifier_pool[func_identifier_idx];

        // check if it doesn't start with "benchmark", if not skip it
        let func_name = func_identifier.to_string();
        if !func_name.starts_with(PREFIX) {
            continue;
        }

        // check if it doesn't match pattern, if not skip it
        let fully_qualified_path = format!("{}::{}::{}", address, identifier, func_name);
        if !fully_qualified_path.contains(&pattern) && !pattern.is_empty() {
            continue;
        }

        // if it does, ensure no params in benchmark function
        let signature_idx: usize = handle.parameters.0.into();
        let func_params = &signature_pool[signature_idx];
        if !func_params.is_empty() {
            eprintln!(
                "\n[WARNING] benchmark function should not have parameters: {}\n",
                func_name,
            );
            // TODO: should we exit instead of continuing with the benchmark
            continue;
        }

        // save function to later run benchmark for it
        func_identifiers.push(func_name);
    }

    func_identifiers
}
