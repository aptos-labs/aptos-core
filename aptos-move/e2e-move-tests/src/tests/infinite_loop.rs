// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_types::transaction::{ExecutionStatus, TransactionStatus};
use move_core_types::vm_status::StatusCode::EXECUTION_LIMIT_REACHED;
use rstest::rstest;
use std::time::Instant;

/// Run with `cargo test <test_name> -- --nocapture` to see output.
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
fn empty_while_loop(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let acc = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    let mut build_options = BuildOptions::default();
    build_options
        .named_addresses
        .insert("publisher".to_string(), *acc.address());
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("infinite_loop.data/empty_loop"),
        build_options,
    ));

    let t0 = Instant::now();
    let result = h.run_entry_function(
        &acc,
        str::parse(format!("{}::test::run", acc.address()).as_str()).unwrap(),
        vec![],
        vec![],
    );
    let t1 = Instant::now();

    println!("{:?}", t1 - t0);

    assert!(matches!(
        result,
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            EXECUTION_LIMIT_REACHED
        )))
    ));
}
