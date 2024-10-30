// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
extern crate core;

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_types::transaction::{ExecutionStatus, TransactionStatus};
use move_core_types::value::MoveValue;
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
fn error_map(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let acc = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("error_map.data/pack"),
        BuildOptions {
            with_error_map: true,
            named_addresses: vec![("publisher".to_string(), *acc.address())]
                .into_iter()
                .collect(),
            ..BuildOptions::default()
        }
    ));

    // Now send transactions which abort with one of two errors, depending on the boolean parameter.
    let result = h.run_entry_function(
        &acc,
        str::parse(format!("{}::test::entry", acc.address()).as_str()).unwrap(),
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
        str::parse(format!("{}::test::entry", acc.address()).as_str()).unwrap(),
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
