// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_language_e2e_tests::{executor::FakeExecutor, feature_flags_for_orderless};
use aptos_types::account_config::CORE_CODE_ADDRESS;
use move_core_types::{
    account_address::AccountAddress,
    value::{serialize_values, MoveValue},
    vm_status::StatusCode,
};
use rstest::rstest;

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn test_timestamp_time_has_started(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut executor = FakeExecutor::stdlib_only_genesis();
    // TODO[Orderless]: Giving code deserialization error when enabling feature flags here. check why
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    let account_address = AccountAddress::random();

    // Invalid address used to call `Timestamp::set_time_has_started`
    let output = executor.try_exec(
        "timestamp",
        "set_time_has_started",
        vec![],
        serialize_values(&vec![MoveValue::Signer(account_address)]),
    );
    assert_eq!(output.unwrap_err().move_abort_code(), Some(327683));

    executor.exec(
        "timestamp",
        "set_time_has_started",
        vec![],
        serialize_values(&vec![MoveValue::Signer(CORE_CODE_ADDRESS)]),
    );
}

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn test_block_double_init(use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
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
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
}
