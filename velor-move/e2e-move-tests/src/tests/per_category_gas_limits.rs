// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    assert_out_of_gas, assert_success, assert_vm_status, enable_golden, tests::common, MoveHarness,
};
use velor_gas_algebra::{Gas, GasQuantity};
use velor_language_e2e_tests::account::Account;
use velor_types::{
    account_address::AccountAddress,
    state_store::{state_key::StateKey, table::TableHandle},
    transaction::TransactionStatus,
    vm_status::StatusCode,
};
use move_core_types::gas_algebra::{InternalGas, NumBytes};
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

    h.modify_gas_schedule(|gas_params: &mut velor_gas_schedule::VelorGasParameters| {
        assert!(
            gas_params.vm.txn.max_execution_gas
                < gas_params.vm.instr.add * GasQuantity::from(10_000_000)
        )
    });
}

#[test]
fn io_limit_reached_by_load_resource() {
    let (mut h, acc) = setup();

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
#[ignore = "test needs redesign after 1.9 charging scheme change."]
fn io_limit_reached_by_new_bytes() {
    let (mut h, acc) = setup();
    enable_golden!(h);

    // Modify the gas schedule.
    h.modify_gas_schedule(|gas_params| {
        // Make other aspects of the gas schedule irrelevant by setting per state byte write gas super high.
        gas_params.vm.txn.storage_io_per_state_byte_write = 10_000_000.into();
        // Allow 10 value bytes charged at most.
        gas_params.vm.txn.max_io_gas = 110_000_000.into();
        // Make the key bytes free, only play around value sizes.
        gas_params.vm.txn.legacy_free_write_bytes_quota = state_key_size();
    });

    test_create_multiple_items(&mut h, &acc, |status| {
        assert_vm_status!(status, StatusCode::IO_LIMIT_REACHED);
    });
}

#[test]
#[ignore = "test needs redesign after 1.9 charging scheme change."]
fn storage_limit_reached_by_new_bytes() {
    let (mut h, acc) = setup();
    enable_golden!(h);

    // Modify the gas schedule.
    h.modify_gas_schedule(|gas_params| {
        // Make other aspects of the gas schedule irrelevant by setting byte fee super high.
        gas_params.vm.txn.legacy_storage_fee_per_excess_state_byte = 1_000_000.into();
        gas_params.vm.txn.storage_io_per_state_byte_write = 1.into();
        // Allow 10 value bytes charged at most.
        gas_params.vm.txn.max_storage_fee = 11_000_000.into();
        // Make the key bytes free, only play around value sizes.
        gas_params.vm.txn.legacy_free_write_bytes_quota = state_key_size();
    });

    test_create_multiple_items(&mut h, &acc, |status| {
        assert_vm_status!(status, StatusCode::STORAGE_LIMIT_REACHED);
    });
}

#[test]
#[ignore = "test needs redesign after 1.9 charging scheme change."]
fn out_of_gas_while_charging_write_gas() {
    let (mut h, acc) = setup();
    enable_golden!(h);

    // Modify the gas schedule.
    h.modify_gas_schedule(|gas_params| {
        // Make other aspects of the gas schedule irrelevant by setting per state byte write gas super high.
        gas_params.vm.txn.storage_io_per_state_byte_write = 10_000_000_000.into();
        // Make sure we don't hit io gas limit
        gas_params.vm.txn.max_io_gas = 1_000_000_000_000.into();
        // Bump max gas allowed
        gas_params.vm.txn.maximum_number_of_gas_units = 1_000_000_000.into();
        // Make the key bytes free, only play around value sizes.
        gas_params.vm.txn.legacy_free_write_bytes_quota = state_key_size();
    });
    // Allow 10 value bytes charged at most. Notice this is in external units.
    h.set_max_gas_per_txn(110_000);

    test_create_multiple_items(&mut h, &acc, |status| assert_out_of_gas!(status));
}

#[test]
#[ignore = "test needs redesign after 1.9 charging scheme change."]
fn out_of_gas_while_charging_storage_fee() {
    let (mut h, acc) = setup();
    enable_golden!(h);

    // Modify the gas schedule.
    h.modify_gas_schedule(|gas_params| {
        // Make other aspects of the gas schedule irrelevant by setting per state byte storage fee super high.
        gas_params.vm.txn.legacy_storage_fee_per_excess_state_byte = 1_000_000.into();
        // Make sure we don't hit storage fee limit
        gas_params.vm.txn.max_storage_fee = 100_000_000.into();
        // Bump max gas allowed
        gas_params.vm.txn.maximum_number_of_gas_units = 1_000_000_000.into();
        // Make the key bytes free, only play around value sizes.
        gas_params.vm.txn.legacy_free_write_bytes_quota = state_key_size();
    });
    // Allow 10 value bytes charged at most. Notice this is in external units,
    //   which is 1/100x octas or 1Mx internal units.
    h.set_max_gas_per_txn(110_000);

    test_create_multiple_items(&mut h, &acc, |status| assert_out_of_gas!(status));
}

fn state_key_size() -> NumBytes {
    let key_size = StateKey::table_item(&TableHandle(AccountAddress::ONE), &[0u8]).size();
    (key_size as u64).into()
}

fn setup() -> (MoveHarness, Account) {
    let mut h = MoveHarness::new();
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
    (h, acc)
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

fn test_create_multiple_items<P>(h: &mut MoveHarness, acc: &Account, assert_failure: P)
where
    P: Fn(TransactionStatus),
{
    // Create 1 + 2 + 3 + 4 = 10 bytes, should pass.
    assert_success!(create_multiple_items(h, acc, 10, 14, 1));
    // Try to create 3 + 4 + 5 = 12 bytes, should fail
    assert_failure(create_multiple_items(h, acc, 0, 3, 3));
    // Try to create 5 + 6 + 7 = 18 bytes, should fail, actually any two of these would exceed the
    // limit already, so it'll expose any problem wrt different orders of the write ops being fed to
    // the charging logic.
    assert_failure(create_multiple_items(h, acc, 4, 7, 5))
}
