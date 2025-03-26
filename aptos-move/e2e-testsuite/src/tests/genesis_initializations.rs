// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_language_e2e_tests::executor::FakeExecutor;
use aptos_types::account_config::CORE_CODE_ADDRESS;
use move_core_types::{
    account_address::AccountAddress,
    value::{serialize_values, MoveValue},
    vm_status::StatusCode,
};

#[test]
fn test_timestamp_time_has_started() {
    let mut executor = FakeExecutor::stdlib_only_genesis();
    let account_address = AccountAddress::random();

    // Invalid address used to call `Timestamp::set_time_has_started`
    let output = executor.try_exec(
        "timestamp",
        "set_time_has_started",
        vec![],
        serialize_values(&vec![MoveValue::Signer(account_address), MoveValue::U64(12)]),
    );
    println!("{:?}", output);
    assert_eq!(output.unwrap_err().move_abort_code(), Some(327683));

    executor.exec(
        "timestamp",
        "set_time_has_started",
        vec![],
        serialize_values(&vec![MoveValue::Signer(CORE_CODE_ADDRESS), MoveValue::U64(12)]),
    );
}

#[test]
fn test_block_double_init() {
    let mut executor = FakeExecutor::stdlib_only_genesis();

    executor.exec(
        "account",
        "create_account_unchecked",
        vec![],
        serialize_values(&vec![MoveValue::Address(CORE_CODE_ADDRESS)]),
    );

    executor.exec(
        "block",
        "initialize",
        vec![],
        serialize_values(&vec![
            MoveValue::Signer(CORE_CODE_ADDRESS),
            MoveValue::U64(1),
        ]),
    );

    let output = executor.try_exec(
        "block",
        "initialize",
        vec![],
        serialize_values(&vec![
            MoveValue::Signer(CORE_CODE_ADDRESS),
            MoveValue::U64(1),
        ]),
    );

    assert_eq!(
        output.unwrap_err().status_code(),
        StatusCode::RESOURCE_ALREADY_EXISTS
    );
}
