// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

extern crate core;

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_types::{
    account_address::AccountAddress,
    transaction::{ExecutionStatus, TransactionStatus},
};
use move_core_types::value::MoveValue;
use serde::{Deserialize, Serialize};
use rstest::rstest;

/// Mimics `0xcafe::test::ModuleData`
#[derive(Serialize, Deserialize)]
struct ModuleData {
    state: Vec<u8>,
}

#[rstest(stateless_account,
    case(true),
    case(false),
)]
fn error_map(stateless_account: bool) {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x915").unwrap(), if stateless_account { None } else { Some(0) });
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
