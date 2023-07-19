// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::gas_meter::GasMeters;
use aptos::move_tool::MemberId;
use aptos_cached_packages::aptos_stdlib;
use aptos_framework::BuiltPackage;
use aptos_gas_algebra::Expression;
use aptos_language_e2e_tests::{account::Account, executor::FakeExecutor};
use aptos_types::transaction::TransactionPayload;
use move_binary_format::CompiledModule;
use move_core_types::{account_address::AccountAddress, language_storage::ModuleId};
use std::{fs::ReadDir, path::PathBuf, string::String, time::Instant};

//// CONSTANTS
const PREFIX: &str = "calibrate_";

/// Generate a TransactionPayload for modules
///
/// ### Arguments
///
/// * `package` - Built Move package
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

/// Sign transaction to create a Module and return transaction status
///
/// ### Arguments
///
/// * `executor` - Runs transactions
/// * `account` - Account to publish module under
/// * `payload` - Info relating to the module
/// * `sequence_number` - Nonce
pub fn sign_module_txn(
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

/// sign user transaction and only records the body of the transaction
///
/// ### Arguments
///
/// * `executor` - Runs transactions
/// * `module_name` - Name of module
/// * `function_name` - Name of function in the module
pub fn sign_user_txn(
    executor: &mut FakeExecutor,
    module_name: &ModuleId,
    function_name: &str,
) -> u128 {
    let elapsed = executor.exec_module(module_name, function_name, vec![], vec![]);
    println!("running time (microseconds): {}", elapsed);
    elapsed
}

/// Publish module under user, sign and run user transaction.
/// This runs for both Gas Meters (regular and abstract).
///
/// ### Arguments
///
/// * `package` - Built Move package
/// * `executor` - Runs transactions
/// * `func_identifiers` - All function names
/// * `address` - Address associated to an account
/// * `identifier` - Name of module
pub fn record_gas_meter(
    package: &BuiltPackage,
    executor: &mut FakeExecutor,
    func_identifiers: Vec<String>,
    address: AccountAddress,
    identifier: &String,
) -> GasMeters {
    // publish test-package under module address
    let creator = executor.new_account_at(address);

    let mut gas_meter = GasMeters {
        regular_meter: Vec::new(),
        abstract_meter: Vec::new(),
    };

    // iterate over all the functions that satisfied the requirements above
    for (sequence_num_counter, func_identifier) in func_identifiers.into_iter().enumerate() {
        println!(
            "Executing {}::{}::{}",
            address,
            identifier,
            func_identifier.clone(),
        );

        // publish package similar to create_publish_package in harness.rs
        println!("Signing txn for module... ");
        let module_payload = generate_module_payload(package);
        let counter = sequence_num_counter.try_into().unwrap();
        sign_module_txn(executor, &creator, module_payload, counter);

        // send a txn that invokes the entry function 0x{address}::{name}::benchmark
        println!("Signing and running user txn for Regular Meter... ");
        let module_name = get_module_name(address, identifier, &func_identifier);
        let duration = sign_user_txn(executor, &module_name, &func_identifier);
        gas_meter.regular_meter.push(duration);

        println!("Signing and running user txn for Abstract Meter... ");
        let gas_formula =
            executor.exec_abstract_usage(&module_name, &func_identifier, vec![], vec![]);
        gas_meter.abstract_meter.push(gas_formula);
    }

    gas_meter
}

/*
 *
 * GETTER FUNCTIONS
 *
 */
/// get module name
///
/// ### Arguments
///
/// * `address` - Address associated to an account
/// * `identifier` - Name of module
/// * `func_identifier` - Name of function in module
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
        address, identifier, func_identifier,
    ))
    .unwrap();

    module_id
}

/// get all directories of Move projects
///
/// ### Arguments
///
/// * `dirs` - directory
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

/// get functional identifiers
///
/// ### Arguments
///
/// * `cm` - Compiled module
/// * `identifier` - Name of module
/// * `address` - Account address for the module
/// * `pattern` - Certain functions to run based on pattern
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
