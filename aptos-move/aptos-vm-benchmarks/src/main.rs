// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos::move_tool::MemberId;
use aptos_cached_packages::aptos_stdlib;
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_language_e2e_tests::executor::FakeExecutor;
use aptos_types::{
    account_address::AccountAddress,
    transaction::{EntryFunction, TransactionPayload},
};
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    //// Compile test-package
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("samples")
        .join("test-package");

    let build_options = BuildOptions::default();
    let package = BuiltPackage::build(path, build_options).expect("build package must succeed");

    //// Setting up local execution environment
    let start = Instant::now();
    // disable parallel execution
    let executor = FakeExecutor::from_head_genesis();
    let mut executor = executor.set_not_parallel();
    let mut sequence_num_counter = 0;

    //// publish test-package under 0xbeef
    let creator = executor.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());

    // publish package similar to create_publish_package in harness.rs
    let code = package.extract_code();
    let metadata = package
        .extract_metadata()
        .expect("extracting package metadata must succeed");
    let module_payload = aptos_stdlib::code_publish_package_txn(
        bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
        code,
    );
    let module_signed_txn = creator
        .transaction()
        .sequence_number(sequence_num_counter)
        .max_gas_amount(2_000_000)
        .gas_unit_price(200)
        .payload(module_payload)
        .sign();
    let module_txn_output = executor.execute_transaction(module_signed_txn);
    let module_txn_status = module_txn_output.status().to_owned();
    println!("module publish status: {:?}", module_txn_status);
    // apply write set to avoid LINKER_ERROR
    executor.apply_write_set(module_txn_output.write_set());
    sequence_num_counter = sequence_num_counter + 1;

    //// send a txn that invokes the entry function 0xbeef::test::benchmark
    let MemberId {
        module_id,
        member_id: function_id,
    } = str::parse("0xbeef::test::benchmark").unwrap();
    let entry_fun_payload = TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        function_id,
        vec![],
        vec![],
    ));
    let entry_fun_signed_txn = creator
        .transaction()
        .sequence_number(sequence_num_counter)
        .max_gas_amount(2_000_000)
        .gas_unit_price(200)
        .payload(entry_fun_payload)
        .sign();

    let entry_fun_txn_output = executor.execute_transaction(entry_fun_signed_txn);
    let entry_fun_txn_status = entry_fun_txn_output.status().to_owned();
    println!("call entry function status: {:?}", entry_fun_txn_status);
    executor.apply_write_set(entry_fun_txn_output.write_set());

    println!("running time (ms): {}", start.elapsed().as_millis());
}
