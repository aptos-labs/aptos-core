// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

extern crate core;

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::FeatureFlag,
    transaction::{ExecutionStatus, TransactionStatus},
};
use move_core_types::value::MoveValue;
use serde::{Deserialize, Serialize};

/// Mimics `0xcafe::test::ModuleData`
#[derive(Serialize, Deserialize)]
struct ModuleData {
    state: Vec<u8>,
}

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

/// Exercises the fix for compiler-generated abort codes (e.g. from a single-argument
/// `assert!`) being incorrectly resolved to a user-defined error constant whose lower
/// 12 bits happen to be zero.
#[test]
fn unspecified_abort_does_not_match_user_zero() {
    let mut h = MoveHarness::new();
    h.enable_features(
        vec![FeatureFlag::EXTRACT_ABORT_INFO_EXACT_MATCH],
        vec![],
    );

    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("error_map.data/pack_unspecified_abort"),
        BuildOptions {
            with_error_map: true,
            ..BuildOptions::default()
        }
    ));

    // 1. `assert!(false)` aborts with UNSPECIFIED_ABORT_CODE; even though E_ZERO = 0
    //    exists in the module, no AbortInfo should be returned.
    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::test_unspecified_abort::unspecified_abort").unwrap(),
        vec![],
        vec![],
    );
    match status {
        TransactionStatus::Keep(ExecutionStatus::MoveAbort { info, .. }) => {
            assert!(
                info.is_none(),
                "UNSPECIFIED_ABORT_CODE must not resolve to user error code 0",
            );
        },
        other => panic!("expected MoveAbort, got {:?}", other),
    }

    // 2. Aborting directly with E_ZERO must still produce AbortInfo.
    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::test_unspecified_abort::abort_with_user_zero").unwrap(),
        vec![],
        vec![],
    );
    check_error(status, "E_ZERO", "User-defined error that happens to use code 0.");

    // 3. A canonical std::error code with reason 0 still resolves via the reason
    //    fallback (upper 5 bytes are zero).
    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::test_unspecified_abort::abort_with_canonical_zero").unwrap(),
        vec![],
        vec![],
    );
    check_error(status, "E_ZERO", "User-defined error that happens to use code 0.");

    // 4. Direct abort with a non-zero user error constant must resolve via the
    //    exact-match path.
    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::test_unspecified_abort::abort_with_user_nonzero").unwrap(),
        vec![],
        vec![],
    );
    check_error(status, "E_NONZERO", "User-defined error with a non-zero code.");

    // 5. A canonical std::error code with a non-zero reason resolves via the
    //    reason fallback (upper 5 bytes are zero).
    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::test_unspecified_abort::abort_with_canonical_nonzero").unwrap(),
        vec![],
        vec![],
    );
    check_error(status, "E_NONZERO", "User-defined error with a non-zero code.");
}

/// Without the feature flag we keep the legacy (buggy) lookup, so that historical
/// transactions replay with their original outputs.
#[test]
fn unspecified_abort_legacy_still_matches_user_zero() {
    let mut h = MoveHarness::new();
    h.enable_features(
        vec![],
        vec![FeatureFlag::EXTRACT_ABORT_INFO_EXACT_MATCH],
    );

    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("error_map.data/pack_unspecified_abort"),
        BuildOptions {
            with_error_map: true,
            ..BuildOptions::default()
        }
    ));

    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::test_unspecified_abort::unspecified_abort").unwrap(),
        vec![],
        vec![],
    );
    // Legacy behaviour: spurious match against E_ZERO.
    check_error(status, "E_ZERO", "User-defined error that happens to use code 0.");
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
