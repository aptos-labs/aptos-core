// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{
    assert_out_of_gas, assert_success, assert_vm_status, enable_golden, tests::common, MoveHarness,
};
use aptos_framework::BuildOptions;
use aptos_gas_algebra::{Gas, GasQuantity};
use aptos_language_e2e_tests::account::Account;
use aptos_types::{
    account_address::AccountAddress,
    state_store::{state_key::StateKey, table::TableHandle},
    transaction::TransactionStatus,
    vm_status::StatusCode,
};
use move_core_types::gas_algebra::{InternalGas, NumBytes};
use rstest::rstest;
use serde::{Deserialize, Serialize};

/// Mimics `0xcafe::test::ModuleData`
#[derive(Serialize, Deserialize)]
struct ModuleData {
    state: Vec<u8>,
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn execution_limit_reached(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    // Publish the infinite loop module.
    let acc = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    let mut build_options = BuildOptions::default();
    build_options
        .named_addresses
        .insert("publisher".to_string(), *acc.address());
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("infinite_loop.data/empty_loop"),
        build_options,
    ));

    // Lower the max execution gas to 1000 units.
    h.modify_gas_schedule(|gas_params| {
        gas_params.vm.txn.max_execution_gas =
            Gas::new(1000) * gas_params.vm.txn.gas_unit_scaling_factor
    });

    // Execute the loop. It should hit the execution limit before running out of gas.
    let res = h.run_entry_function(
        &acc,
        str::parse(format!("{}::test::run", acc.address()).as_str()).unwrap(),
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

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn io_limit_reached_by_load_resource(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let (mut h, acc) = setup(
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );

    // Lower the max io gas to lower than a single load_resource
    h.modify_gas_schedule(|gas_params| {
        gas_params.vm.txn.max_io_gas = InternalGas::new(300_000 - 1)
    });

    // Execute the test function. The function will attempt to check if a resource exists and shall immediately hit the IO gas limit.
    let res = h.run_entry_function(
        &acc,
        str::parse(format!("{}::test::run_exists", acc.address()).as_str()).unwrap(),
        vec![],
        vec![],
    );
    assert_vm_status!(res, StatusCode::IO_LIMIT_REACHED);
}

#[rstest(stateless_account, case(true), case(false))]
#[ignore = "test needs redesign after 1.9 charging scheme change."]
fn io_limit_reached_by_new_bytes(stateless_account: bool) {
    let (mut h, acc) = setup(stateless_account, true, true);
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

#[rstest(stateless_account, case(true), case(false))]
#[ignore = "test needs redesign after 1.9 charging scheme change."]
fn storage_limit_reached_by_new_bytes(stateless_account: bool) {
    let (mut h, acc) = setup(stateless_account, true, true);
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

#[rstest(stateless_account, case(true), case(false))]
#[ignore = "test needs redesign after 1.9 charging scheme change."]
fn out_of_gas_while_charging_write_gas(stateless_account: bool) {
    let (mut h, acc) = setup(stateless_account, true, true);
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

#[rstest(stateless_account, case(true), case(false))]
#[ignore = "test needs redesign after 1.9 charging scheme change."]
fn out_of_gas_while_charging_storage_fee(stateless_account: bool) {
    let (mut h, acc) = setup(stateless_account, true, true);
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

fn setup(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) -> (MoveHarness, Account) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    // Publish the test module.
    let acc = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    let mut build_options = BuildOptions::default();
    build_options
        .named_addresses
        .insert("publisher".to_string(), *acc.address());
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("per_category_gas_limits.data/test"),
        build_options
    ));

    // Initialize.
    assert_success!(h.run_entry_function(
        &acc,
        str::parse(format!("{}::test::init_table_of_bytes", acc.address()).as_str()).unwrap(),
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
        str::parse(format!("{}::test::create_multiple", acc.address()).as_str()).unwrap(),
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
