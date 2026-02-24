// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Transactional tests for macros like `assert!`, `assert_eq!`, and `assert_ne!`
//! introduced in Move language version 2.4 and onwards.

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_types::{account_address::AccountAddress, transaction::{ExecutionStatus, TransactionStatus}};

#[test]
fn test_macros() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x99").unwrap());
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("macros.data/pack"),
        BuildOptions::move_2().set_latest_language()
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::assert::test_assert_returns").unwrap(),
        vec![],
        vec![],
    ));

    assert_abort_code_and_message(
        h.run_entry_function(
            &acc,
            str::parse("0x99::assert::test_assert_aborts").unwrap(),
            vec![],
            vec![],
        ),
        14566554180833181696,
        "",
    );

    assert_abort_code_and_message(
        h.run_entry_function(
            &acc,
            str::parse("0x99::assert::test_assert_aborts_with_code").unwrap(),
            vec![],
            vec![],
        ),
        42,
        "",
    );

    assert_abort_code_and_message(
        h.run_entry_function(
            &acc,
            str::parse("0x99::assert::test_assert_aborts_with_message").unwrap(),
            vec![],
            vec![],
        ),
        14566554180833181696,
        "custom error message",
    );

    assert_abort_code_and_message(
        h.run_entry_function(
            &acc,
            str::parse("0x99::assert::test_assert_aborts_with_formatted_message").unwrap(),
            vec![],
            vec![],
        ),
        14566554180833181696,
        "custom error message with arg: 42",
    );

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::assert_eq::test_assert_eq_returns").unwrap(),
        vec![],
        vec![],
    ));

    assert_abort_code_and_message(
        h.run_entry_function(
            &acc,
            str::parse("0x99::assert_eq::test_assert_eq_aborts").unwrap(),
            vec![],
            vec![],
        ),
        14566554180833181696,
        "assertion `left == right` failed\n  left: 1\n right: 2",
    );

    assert_abort_code_and_message(
        h.run_entry_function(
            &acc,
            str::parse("0x99::assert_eq::test_assert_eq_aborts_with_message").unwrap(),
            vec![],
            vec![],
        ),
        14566554180833181696,
        "assertion `left == right` failed: custom error message\n  left: 1\n right: 2",
    );

    assert_abort_code_and_message(
        h.run_entry_function(
            &acc,
            str::parse("0x99::assert_eq::test_assert_eq_aborts_with_formatted_message").unwrap(),
            vec![],
            vec![],
        ),
        14566554180833181696,
        "assertion `left == right` failed: custom error message with arg: 42\n  left: 1\n right: 2",
    );

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::assert_ne::test_assert_ne_returns").unwrap(),
        vec![],
        vec![],
    ));

    assert_abort_code_and_message(
        h.run_entry_function(
            &acc,
            str::parse("0x99::assert_ne::test_assert_ne_aborts").unwrap(),
            vec![],
            vec![],
        ),
        14566554180833181696,
        "assertion `left != right` failed\n  left: 1\n right: 1",
    );

    assert_abort_code_and_message(
        h.run_entry_function(
            &acc,
            str::parse("0x99::assert_ne::test_assert_ne_aborts_with_message").unwrap(),
            vec![],
            vec![],
        ),
        14566554180833181696,
        "assertion `left != right` failed: custom error message\n  left: 1\n right: 1",
    );

    assert_abort_code_and_message(
        h.run_entry_function(
            &acc,
            str::parse("0x99::assert_ne::test_assert_ne_aborts_with_formatted_message").unwrap(),
            vec![],
            vec![],
        ),
        14566554180833181696,
        "assertion `left != right` failed: custom error message with arg: 42\n  left: 1\n right: 1",
    );
}

fn assert_abort_code_and_message(
    status: TransactionStatus,
    expected_code: u64,
    expected_message: &str,
) {
    match status {
        TransactionStatus::Keep(ExecutionStatus::MoveAbort { code, info, .. }) => {
            assert_eq!(code, expected_code, "Abort codes do not match");

            let message = info.map(|info| info.description).unwrap_or_default();
            assert_eq!(message, expected_message, "Abort messages do not match");
        },
        _ => {
            panic!("Expected transaction to abort, but got: {:?}", status);
        }
    }
}
