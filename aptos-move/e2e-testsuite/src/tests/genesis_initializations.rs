// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::account_config::CORE_CODE_ADDRESS;
use language_e2e_tests::executor::FakeExecutor;
use move_deps::move_core_types::{
    account_address::AccountAddress,
    value::{serialize_values, MoveValue},
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
        serialize_values(&vec![MoveValue::Signer(account_address)]),
    );
    assert_eq!(output.unwrap_err().move_abort_code(), Some(514));

    executor.exec(
        "timestamp",
        "set_time_has_started",
        vec![],
        serialize_values(&vec![MoveValue::Signer(CORE_CODE_ADDRESS)]),
    );

    let output = executor.try_exec(
        "timestamp",
        "set_time_has_started",
        vec![],
        serialize_values(&vec![MoveValue::Signer(CORE_CODE_ADDRESS)]),
    );

    assert_eq!(output.unwrap_err().move_abort_code(), Some(1));
}

#[test]
fn test_block_double_init() {
    let mut executor = FakeExecutor::stdlib_only_genesis();

    executor.exec(
        "block",
        "initialize_block_metadata",
        vec![],
        serialize_values(&vec![
            MoveValue::Signer(CORE_CODE_ADDRESS),
            MoveValue::U64(0),
        ]),
    );

    let output = executor.try_exec(
        "block",
        "initialize_block_metadata",
        vec![],
        serialize_values(&vec![
            MoveValue::Signer(CORE_CODE_ADDRESS),
            MoveValue::U64(0),
        ]),
    );

    assert_eq!(output.unwrap_err().move_abort_code(), Some(6));
}
