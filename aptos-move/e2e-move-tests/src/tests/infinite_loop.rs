// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{ExecutionStatus, TransactionStatus},
};
use move_core_types::vm_status::StatusCode::EXECUTION_LIMIT_REACHED;
use std::time::Instant;

/// Run with `cargo test <test_name> -- --nocapture` to see output.

#[test]
fn empty_while_loop_test_with_stateful_sender() {
    let mut h = MoveHarness::new();
    let stateless_acc = h.new_account_at(AccountAddress::from_hex_literal("0x915").unwrap(), Some(0));
    empty_while_loop(&mut h, acc);
}

#[test]
fn empty_while_loop_test_with_stateless_sender() {
    let mut h = MoveHarness::new();
    let stateless_acc = h.new_account_at(AccountAddress::from_hex_literal("0x915").unwrap(), None);
    empty_while_loop(&mut h, acc);
}

fn empty_while_loop(h: &mut MoveHarness, acc: Account) {
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
