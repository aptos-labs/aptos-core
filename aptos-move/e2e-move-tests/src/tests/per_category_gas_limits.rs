// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_gas_algebra::{Fee, Gas, GasQuantity};
use aptos_types::{account_address::AccountAddress, vm_status::StatusCode};
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
    assert_success!(h.publish_package(
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
fn io_limit_reached() {
    let mut h = MoveHarness::new();

    // Publish the test module.
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("execution_limit.data/test"),));

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
fn storage_limit_reached() {
    let mut h = MoveHarness::new();

    // Publish the test module.
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("execution_limit.data/test"),));

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
