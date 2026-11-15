// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Paranoid type checks must not cost O(type size) per instruction, since gas only covers the
//! value. Copy/compare and borrow loops run over 123-node struct and function locals and over
//! a `u64` control; a big variant materially slower than the control fails the test.

use crate::{assert_success, tests::common, MoveHarness};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{ExecutionStatus, TransactionStatus},
};
use std::time::{Duration, Instant};

const ITERATIONS: u64 = 300;

fn run(h: &mut MoveHarness, acc: &aptos_transaction_simulation::Account, f: &str) -> Duration {
    let start = Instant::now();
    let status = h.run_entry_function(
        acc,
        str::parse(&format!("0xcafe::cost::{f}")).unwrap(),
        vec![],
        vec![bcs::to_bytes(&ITERATIONS).unwrap()],
    );
    let elapsed = start.elapsed();
    assert!(
        matches!(status, TransactionStatus::Keep(ExecutionStatus::Success)),
        "{f}: {status:?}"
    );
    elapsed
}

#[test]
fn paranoid_type_check_cost_is_independent_of_type_size() {
    // Paranoid type checks are enabled by default in the harness (as on mainnet).
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("paranoid_type_check_cost.data/pack"),
    ));

    // Warm up module and type caches so that the measurements below only compare the
    // instruction loops.
    for f in [
        "copy_neq_small",
        "copy_neq_big",
        "copy_fn_big",
        "borrow_small",
        "borrow_big",
        "borrow_fn_big",
    ] {
        run(&mut h, &acc, f);
    }

    for (small, big) in [
        ("copy_neq_small", "copy_neq_big"),
        ("copy_neq_small", "copy_fn_big"),
        ("borrow_small", "borrow_big"),
        ("borrow_small", "borrow_fn_big"),
    ] {
        // Take the best of a few runs to reduce scheduler noise.
        let small_time = (0..3).map(|_| run(&mut h, &acc, small)).min().unwrap();
        let big_time = (0..3).map(|_| run(&mut h, &acc, big)).min().unwrap();
        let ratio = big_time.as_secs_f64() / small_time.as_secs_f64();
        println!("{small}: {small_time:?}, {big}: {big_time:?}, ratio: {ratio:.2}");

        // Loose threshold: catches O(type size) per-instruction work, not small regressions.
        assert!(
            ratio < 3.0,
            "{big} is {ratio:.2}x slower than {small}: paranoid type checks scale with type size"
        );
    }
}
