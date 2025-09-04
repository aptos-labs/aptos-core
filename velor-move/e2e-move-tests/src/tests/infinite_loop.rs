// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use velor_types::{
    account_address::AccountAddress,
    transaction::{ExecutionStatus, TransactionStatus},
};
use move_core_types::vm_status::StatusCode::EXECUTION_LIMIT_REACHED;
use std::time::Instant;

/// Run with `cargo test <test_name> -- --nocapture` to see output.

#[test]
fn empty_while_loop() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
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
