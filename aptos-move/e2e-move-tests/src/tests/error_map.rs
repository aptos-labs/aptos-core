// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

extern crate core;

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_types::{
    account_address::AccountAddress,
    transaction::{ExecutionStatus, TransactionStatus},
};
use move_core_types::value::MoveValue;

#[test]
fn error_map() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("error_map.data/pack"),
        BuildOptions {
            with_error_map: true,
            ..BuildOptions::default()
        }
    ));

    // Now send transactions which abort with one of two errors, depending on the boolean parameter.
    let result = h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::entry").unwrap(),
        vec![],
        vec![MoveValue::Bool(true).simple_serialize().unwrap()],
    );
    check_error(
        result,
        "ESOME_ERROR",
        "This error is raised because it wants to.",
    );

    let result = h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::entry").unwrap(),
        vec![],
        vec![MoveValue::Bool(false).simple_serialize().unwrap()],
    );
    check_error(
        result,
        "ESOME_OTHER_ERROR",
        "This error is often raised as well.",
    );
}

/// Tests that a single-argument `assert!` — which aborts with `UNSPECIFIED_ABORT_CODE` — does not
/// incorrectly return abort info for a user-defined error constant that happens to have code 0.
#[test]
fn error_map_unspecified_abort_code() {
    let mut h = MoveHarness::new();

    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("error_map.data/pack_assert"),
        BuildOptions {
            with_error_map: true,
            ..BuildOptions::default()
        }
    ));

    // assert!(false) uses UNSPECIFIED_ABORT_CODE. Even though the module has E_SOME_ERROR = 0,
    // no abort info should be returned.
    let result = h.run_entry_function(
        &acc,
        str::parse("0xcafe::test_assert::entry_assert").unwrap(),
        vec![],
        vec![],
    );
    match result {
        TransactionStatus::Keep(ExecutionStatus::MoveAbort { info, .. }) => {
            assert!(
                info.is_none(),
                "expected no AbortInfo for UNSPECIFIED_ABORT_CODE"
            );
        },
        _ => panic!("expected MoveAbort"),
    }

    // Sanity check: aborting with E_SOME_ERROR directly should still return abort info.
    let result = h.run_entry_function(
        &acc,
        str::parse("0xcafe::test_assert::entry_error").unwrap(),
        vec![],
        vec![],
    );
    check_error(
        result,
        "E_SOME_ERROR",
        "Some error that happens to have code 0.",
    );

    // Sanity check: aborting with a canonical std::error code whose reason is 0 should also
    // return abort info, via the reason fallback.
    let result = h.run_entry_function(
        &acc,
        str::parse("0xcafe::test_assert::entry_canonical_error").unwrap(),
        vec![],
        vec![],
    );
    check_error(
        result,
        "E_SOME_ERROR",
        "Some error that happens to have code 0.",
    );
}

fn check_error(status: TransactionStatus, reason_name: &str, description: &str) {
    match status {
        TransactionStatus::Keep(ExecutionStatus::MoveAbort { info, .. }) => {
            if let Some(i) = info {
                assert_eq!(i.reason_name, reason_name);
                assert_eq!(i.description, description);
            } else {
                panic!("expected AbortInfo populated")
            }
        },
        _ => panic!("expected MoveAbort"),
    }
}
