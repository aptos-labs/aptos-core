// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::account_config;
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
        "Timestamp",
        "set_time_has_started",
        vec![],
        serialize_values(&vec![MoveValue::Signer(account_address)]),
    );
    assert_eq!(output.unwrap_err().move_abort_code(), Some(2));

    executor.exec(
        "Timestamp",
        "set_time_has_started",
        vec![],
        serialize_values(&vec![MoveValue::Signer(
            account_config::aptos_root_address(),
        )]),
    );

    let output = executor.try_exec(
        "Timestamp",
        "set_time_has_started",
        vec![],
        serialize_values(&vec![MoveValue::Signer(
            account_config::aptos_root_address(),
        )]),
    );

    assert_eq!(output.unwrap_err().move_abort_code(), Some(1));
}

#[test]
fn test_block_double_init() {
    let mut executor = FakeExecutor::stdlib_only_genesis();

    executor.exec(
        "Block",
        "initialize_block_metadata",
        vec![],
        serialize_values(&vec![
            MoveValue::Signer(account_config::aptos_root_address()),
            MoveValue::U64(0),
        ]),
    );

    let output = executor.try_exec(
        "Block",
        "initialize_block_metadata",
        vec![],
        serialize_values(&vec![
            MoveValue::Signer(account_config::aptos_root_address()),
            MoveValue::U64(0),
        ]),
    );

    assert_eq!(output.unwrap_err().move_abort_code(), Some(6));
}
