// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, assert_vm_status, enable_golden, tests::common, MoveHarness};
use aptos_gas_algebra::{Fee, Gas, GasQuantity};
use aptos_language_e2e_tests::account::Account;
use aptos_types::{
    account_address::AccountAddress,
    state_store::{state_key::StateKey, table::TableHandle},
    transaction::TransactionStatus,
    vm_status::StatusCode,
};
use move_core_types::gas_algebra::InternalGas;
use serde::{Deserialize, Serialize};

/// Mimics `0xcafe::test::ModuleData`
#[derive(Serialize, Deserialize)]
struct ModuleData {
    state: Vec<u8>,
}

#[test]
fn execution_limit_reached() {
    let mut h = MoveHarness::new();

    // Publish the infinite loop module.
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    assert_success!(h.publish_package_cache_building(
        &acc,
        &common::test_dir_path("infinite_loop.data/empty_loop"),
    ));

    // Lower the max execution gas to 1000 units.
    h.modify_gas_schedule(|gas_params| {
        gas_params.vm.txn.max_execution_gas =
            Gas::new(1000) * gas_params.vm.txn.gas_unit_scaling_factor
    });

    // Execute the loop. It should hit the execution limit before running out of gas.
    let res = h.run_entry_function(
        &acc,
        str::parse("0xbeef::test::run").unwrap(),
        vec![],
        vec![],
    );
    assert_vm_status!(res, StatusCode::EXECUTION_LIMIT_REACHED);
}

#[test]
fn bounded_execution_time() {
    let mut h = MoveHarness::new();

    h.modify_gas_schedule(|gas_params: &mut aptos_gas_schedule::AptosGasParameters| {
        assert!(
            gas_params.vm.txn.max_execution_gas
                < gas_params.vm.instr.add * GasQuantity::from(10_000_000)
        )
    });
}

#[test]
fn io_limit_reached_by_load_resource() {
    let mut h = MoveHarness::new();

    // Publish the test module.
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    assert_success!(h.publish_package_cache_building(
        &acc,
        &common::test_dir_path("per_category_gas_limits.data/test"),
    ));

    // Lower the max io gas to lower than a single load_resource
    h.modify_gas_schedule(|gas_params| {
        gas_params.vm.txn.max_io_gas = InternalGas::new(300_000 - 1)
    });

    // Execute the test function. The function will attempt to check if a resource exists and shall immediately hit the IO gas limit.
    let res = h.run_entry_function(
        &acc,
        str::parse("0xbeef::test::run_exists").unwrap(),
        vec![],
        vec![],
    );
    assert_vm_status!(res, StatusCode::IO_LIMIT_REACHED);
}

#[test]
fn io_limit_reached_by_new_bytes() {
    let mut h = MoveHarness::new();
    enable_golden!(h);

    // Publish the test module.
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    assert_success!(h.publish_package_cache_building(
        &acc,
        &common::test_dir_path("per_category_gas_limits.data/test"),
    ));

    // Initialize.
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xbeef::test::init_table_of_bytes").unwrap(),
        vec![],
        vec![],
    ));

    let key_size = StateKey::table_item(TableHandle(AccountAddress::ONE), vec![0u8]).size();
    // Modify the gas schedule.
    h.modify_gas_schedule(|gas_params| {
        // Make other aspects of the gas schedule irrelevant by setting per state byte write gas super high.
        gas_params.vm.txn.storage_io_per_state_byte_write = 10_000_000.into();
        // Allow 10 value bytes charged at most.
        gas_params.vm.txn.max_io_gas = 110_000_000.into();
        // Make the key bytes free, only play around value sizes.
        gas_params.vm.txn.free_write_bytes_quota = (key_size as u64).into();
    });

    // Create 1 + 2 + 3 + 4 = 10 bytes, should pass.
    assert_success!(create_multiple_items(&mut h, &acc, 10, 14, 1));
    // Try to create 3 + 4 + 5 = 12 bytes, should fail
    assert_vm_status!(
        create_multiple_items(&mut h, &acc, 0, 3, 3),
        StatusCode::IO_LIMIT_REACHED,
    );
    // Try to create 5 + 6 + 7 = 18 bytes, should fail, actually any two of these would exceed the
    // limit already, so it'll expose any problem wrt different orders of the write ops being fed to
    // the charging logic.
    assert_vm_status!(
        create_multiple_items(&mut h, &acc, 4, 7, 5),
        StatusCode::IO_LIMIT_REACHED,
    );
}

#[test]
fn storage_limit_reached_by_new_bytes() {
    let mut h = MoveHarness::new();
    enable_golden!(h);

    // Publish the test module.
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    assert_success!(h.publish_package_cache_building(
        &acc,
        &common::test_dir_path("per_category_gas_limits.data/test"),
    ));

    // Initialize.
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xbeef::test::init_table_of_bytes").unwrap(),
        vec![],
        vec![],
    ));

    let key_size = StateKey::table_item(TableHandle(AccountAddress::ONE), vec![0u8]).size();
    // Modify the gas schedule.
    h.modify_gas_schedule(|gas_params| {
        // Make other aspects of the gas schedule irrelevant by setting byte fee super high.
        gas_params.vm.txn.storage_fee_per_excess_state_byte = 1_000_000.into();
        gas_params.vm.txn.storage_io_per_state_byte_write = 1.into();
        // Allow 10 value bytes charged at most.
        gas_params.vm.txn.max_storage_fee = 11_000_000.into();
        // Make the key bytes free, only play around value sizes.
        gas_params.vm.txn.free_write_bytes_quota = (key_size as u64).into();
    });

    // Create 1 + 2 + 3 + 4 = 10 bytes, should pass.
    assert_success!(create_multiple_items(&mut h, &acc, 10, 14, 1));
    // Try to create 3 + 4 + 5 = 12 bytes, should fail
    assert_vm_status!(
        create_multiple_items(&mut h, &acc, 0, 3, 3),
        StatusCode::STORAGE_LIMIT_REACHED,
    );
    // Try to create 5 + 6 + 7 = 18 bytes, should fail, actually any two of these would exceed the
    // limit already, so it'll expose any problem wrt different orders of the write ops being fed to
    // the charging logic.
    assert_vm_status!(
        create_multiple_items(&mut h, &acc, 4, 7, 5),
        StatusCode::STORAGE_LIMIT_REACHED,
    );
}

#[test]
fn storage_limit_reached() {
    let mut h = MoveHarness::new();

    // Publish the test module.
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    assert_success!(h.publish_package_cache_building(
        &acc,
        &common::test_dir_path("per_category_gas_limits.data/test"),
    ));

    // Lower the max storage fee to 10 Octa.
    h.modify_gas_schedule(|gas_params| gas_params.vm.txn.max_storage_fee = Fee::new(10));

    // Execute the test function. The function will attempt to publish a resource and shall immediately hit the storage fee limit.
    let res = h.run_entry_function(
        &acc,
        str::parse("0xbeef::test::run_move_to").unwrap(),
        vec![],
        vec![],
    );
    assert_vm_status!(res, StatusCode::STORAGE_LIMIT_REACHED);
}

fn ser<T: Serialize>(t: &T) -> Vec<u8> {
    bcs::to_bytes(t).unwrap()
}

fn create_multiple_items(
    h: &mut MoveHarness,
    acc: &Account,
    begin: u8,
    end: u8,
    base_size: u64,
) -> TransactionStatus {
    h.run_entry_function(
        acc,
        str::parse("0xbeef::test::create_multiple").unwrap(),
        vec![],
        vec![ser(&begin), ser(&end), ser(&base_size)],
    )
}
