// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos::move_tool::MemberId;
use aptos_cached_packages::aptos_stdlib;
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_language_e2e_tests::{account::Account, executor::FakeExecutor};
use aptos_types::transaction::{EntryFunction, TransactionPayload};
use move_binary_format::CompiledModule;
use std::fs::read_dir;
use std::path::PathBuf;
use std::time::Instant;

//// generate a TransactionPayload for modules
fn generate_module_payload(package: &BuiltPackage) -> TransactionPayload {
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

//// generate a TransactionPayload for entry functions
fn generate_entry_fun_payloads(account: &Account, package_name: &str) -> TransactionPayload {
    let MemberId {
        module_id,
        member_id: function_id,
    } = str::parse(&format!(
        "0x{}::{}::benchmark",
        *account.address(),
        package_name,
    ))
    .unwrap();
    TransactionPayload::EntryFunction(EntryFunction::new(module_id, function_id, vec![], vec![]))
}

//// sign transaction and return transaction status
fn sign_txn(
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
    let txn_output = executor.execute_transaction(sign_tx);
    let txn_status = txn_output.status().to_owned();
    assert!(txn_output.status().status().unwrap().is_success());
    println!("txn status: {:?}", txn_status);
    // apply write set to avoid LINKER_ERROR
    executor.apply_write_set(txn_output.write_set());
}

fn main() {
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

        //// Restart timer and sequence counter for each new package
        let start = Instant::now();

        let codes = package.extract_code();
        for code in codes {
            let compiled_module = CompiledModule::deserialize(&code).unwrap();
            let mut module_bytes = vec![];
            compiled_module.serialize(&mut module_bytes).unwrap();
            let module_id = compiled_module.self_id();
            let address = &module_id.address();
            let identifier = &module_id.name().as_str();

            //// publish test-package under module address
            let creator = executor.new_account_at(**address);

            println!(
                "Executing {}::{}::benchmark",
                address.to_string(),
                identifier
            );

            // publish package similar to create_publish_package in harness.rs
            let module_payload = generate_module_payload(&package);
            sign_txn(
                &mut executor,
                &creator,
                module_payload,
                sequence_num_counter,
            );
            sequence_num_counter = sequence_num_counter + 1;

            // avoid SEQUENCE_NUMBER_TOO_NEW error
            // only count running time of entry function
            let mut sequence_num_counter = 0;

            //// send a txn that invokes the entry function 0x{address}::{name}::benchmark
            let entry_fun_payload = generate_entry_fun_payloads(&creator, *identifier);
            sign_txn(
                &mut executor,
                &creator,
                entry_fun_payload,
                sequence_num_counter,
            );
        }

        println!("running time (ms): {}", start.elapsed().as_millis());
    }
}
