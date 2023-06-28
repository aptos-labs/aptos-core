// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_cached_packages::aptos_stdlib;
use aptos_framework::BuiltPackage;
use aptos_types::transaction::TransactionPayload;
use aptos_language_e2e_tests::{account::Account, executor::FakeExecutor};
use move_core_types::language_storage::ModuleId;
use std::time::Instant;

//// generate a TransactionPayload for modules
pub fn generate_module_payload(package: &BuiltPackage) -> TransactionPayload {
    // publish package similar to create_publish_package in harness.rs
    let code = package.extract_code();
    let metadata = package
        .extract_metadata()
        .expect("extracting package metadata must succeed");
    aptos_stdlib::code_publish_package_txn(
        bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
        code,
    )
}

//// sign transaction and return transaction status
pub fn sign_module_txn(
    executor: &mut FakeExecutor,
    account: &Account,
    payload: TransactionPayload,
    sequence_number: u64,
) {
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

    let txn_status = txn_output.status().to_owned();
    assert!(txn_output.status().status().unwrap().is_success());
    println!("txn status: {:?}", txn_status);
}

pub fn sign_user_txn(executor: &mut FakeExecutor, module_name: &ModuleId, function_name: &str) {
    let start = Instant::now();
    executor.exec_module(module_name, function_name, vec![], vec![]);
    let elapsed = start.elapsed();
    println!("running time (microseconds): {}", elapsed.as_micros());
}
