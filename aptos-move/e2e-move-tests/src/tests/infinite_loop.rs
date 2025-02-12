// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{ExecutionStatus, TransactionStatus},
};
use move_core_types::vm_status::StatusCode::EXECUTION_LIMIT_REACHED;
use std::time::Instant;
use rstest::rstest;

/// Run with `cargo test <test_name> -- --nocapture` to see output.
#[rstest(stateless_account,
    case(true),
    case(false),
)]
fn empty_while_loop(stateless_account: bool) {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x915").unwrap(), if stateless_account { None } else { Some(0)});

    assert_success!(h.publish_package_cache_building(
        &acc,
        &common::test_dir_path("infinite_loop.data/empty_loop"),
    ));

    let t0 = Instant::now();
    let result = h.run_entry_function(
        &acc,
        str::parse("0xbeef::test::run").unwrap(),
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
