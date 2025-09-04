// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::measurements::GasMeasurements;
use anyhow::{anyhow, Result};
use velor_cached_packages::velor_stdlib;
use velor_framework::BuiltPackage;
use velor_language_e2e_tests::{
    account::Account,
    executor::{ExecFuncTimerDynamicArgs, FakeExecutor, GasMeterType},
};
use velor_types::transaction::TransactionPayload;
use move_binary_format::{file_format::SignatureToken, CompiledModule};
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::ModuleId,
    value::{serialize_values, MoveValue},
};
use std::{fs::ReadDir, path::PathBuf, string::String, time::Instant};

// CONSTANTS
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
    velor_stdlib::code_publish_package_txn(
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

/// execute user transaction and the time only records the body of the transaction
///
/// ### Arguments
///
/// * `executor` - Runs transactions
/// * `module_name` - Name of module
/// * `function_name` - Name of function in the module
pub fn execute_user_txn(
    executor: &mut FakeExecutor,
    module_name: &ModuleId,
    function_name: &str,
    iterations: u64,
    args: Vec<Vec<u8>>,
) -> u128 {
    let elapsed = executor
        .exec_func_record_running_time(
            module_name,
            function_name,
            vec![],
            args,
            iterations,
            ExecFuncTimerDynamicArgs::NoArgs,
            GasMeterType::UnmeteredGasMeter,
        )
        .elapsed_micros();
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
pub fn record_gas_usage(
    package: &BuiltPackage,
    executor: &mut FakeExecutor,
    func_identifiers: Vec<(String, Vec<Vec<u8>>)>,
    address: AccountAddress,
    identifier: &String,
    iterations: u64,
) -> GasMeasurements {
    // publish test-package under module address
    let creator = executor.new_account_at(address);

    let mut gas_measurement = GasMeasurements {
        regular_meter: Vec::new(),
        abstract_meter: Vec::new(),
        equation_names: Vec::new(),
    };

    // iterate over all the functions that satisfied the requirements above
    for (sequence_num_counter, func_identifier) in func_identifiers.into_iter().enumerate() {
        println!(
            "Executing {}::{}::{}",
            address,
            identifier,
            func_identifier.0.clone(),
        );
        gas_measurement
            .equation_names
            .push(format!("{}::{}", &identifier, &func_identifier.0));

        // publish package similar to create_publish_package in harness.rs
        println!("Signing txn for module... ");
        let module_payload = generate_module_payload(package);
        let counter = sequence_num_counter.try_into().unwrap();
        execute_module_txn(executor, &creator, module_payload, counter);

        // send a txn that invokes the entry function 0x{address}::{name}::benchmark
        println!("Signing and running user txn for Regular Meter... ");
        let module_name = get_module_name(address, identifier, &func_identifier.0);
        let duration = execute_user_txn(
            executor,
            &module_name,
            &func_identifier.0,
            iterations,
            func_identifier.1.clone(),
        );
        gas_measurement.regular_meter.push(duration);

        println!("Signing and running user txn for Abstract Meter... ");
        let gas_formula = executor.exec_abstract_usage(
            &module_name,
            &func_identifier.0,
            vec![],
            func_identifier.1,
        );
        gas_measurement.abstract_meter.push(gas_formula);
    }

    gas_measurement
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
    identifier: &str,
    _func_identifier: &str,
) -> ModuleId {
    ModuleId::new(address, Identifier::new(identifier).unwrap())
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
pub fn list_entrypoints(
    cm: &CompiledModule,
    pattern: String,
) -> Result<Vec<(String, Vec<Vec<u8>>)>> {
    // find non-entry functions and ignore them
    // keep entry function names in func_identifiers vector
    let funcs = &cm.function_defs;
    let func_handles = &cm.function_handles;
    let func_identifier_pool = &cm.identifiers;

    let module_id = cm.self_id();
    let identifier = module_id.name().to_string();
    let address = module_id.address();

    // find # of params in each func if it is entry function
    let signature_pool = &cm.signatures;

    let mut func_identifiers: Vec<(String, Vec<Vec<u8>>)> = Vec::new();
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
            let first_arg_signature_token = &func_params.0.to_vec()[0];
            match first_arg_signature_token {
                SignatureToken::Signer => {
                    let args = serialize_values(&vec![MoveValue::Signer(*address)]);
                    func_identifiers.push((func_name, args));
                },
                _ => {
                    return Err(anyhow!(
                        "Failed: only supports 1 parameter that is a signer type at the moment."
                    ));
                },
            }
        } else {
            func_identifiers.push((func_name, vec![]));
        }
    }

    Ok(func_identifiers)
}
