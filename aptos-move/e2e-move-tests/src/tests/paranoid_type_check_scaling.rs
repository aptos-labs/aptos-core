// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Checks that the cost of each instruction family below does not scale with the size of the
//! runtime type: function-typed locals, comparisons of distinct locals, return and native-call
//! type checks, the layout and resource caches, and `type_of`. Each family runs a deep type and
//! a small control with the same bytecode, and fails if the deep one is materially slower.

use crate::{assert_success, tests::common, MoveHarness};
use aptos_transaction_simulation::Account;
use aptos_types::{
    account_address::AccountAddress,
    transaction::{ExecutionStatus, TransactionStatus},
};
use std::time::{Duration, Instant};

const ITERATIONS: u64 = 300;

fn run(h: &mut MoveHarness, acc: &Account, f: &str, mut args: Vec<Vec<u8>>) -> Duration {
    args.push(bcs::to_bytes(&ITERATIONS).unwrap());
    let start = Instant::now();
    let status = h.run_entry_function(
        acc,
        str::parse(&format!("0xcafe::scaling::{f}")).unwrap(),
        vec![],
        args,
    );
    let elapsed = start.elapsed();
    assert!(
        matches!(status, TransactionStatus::Keep(ExecutionStatus::Success)),
        "{f}: {status:?}"
    );
    elapsed
}

#[test]
fn paranoid_type_check_cost_does_not_scale_with_type_size() {
    // Production nodes run with layout caches on; the harness default is off.
    aptos_vm_environment::prod_configs::set_layout_caches(true);
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("paranoid_type_check_scaling.data/pack"),
    ));

    // `object_exists_*` creates a named object, so each call needs a fresh seed.
    let mut seed = 0u64;
    let mut args = |f: &str| -> Vec<Vec<u8>> {
        if f.starts_with("object_exists") {
            seed += 1;
            vec![bcs::to_bytes(&seed.to_le_bytes().to_vec()).unwrap()]
        } else {
            vec![]
        }
    };

    let families = [
        "copy_pop_fn",
        "distinct_locals",
        "fn_stloc",
        "ret_compare",
        "native_arg",
        "bcs_const",
        "type_of",
        "object_exists",
    ];

    // Warm up module and type caches so that only the instruction loops are compared.
    for family in families {
        for f in [format!("{family}_small"), format!("{family}_big")] {
            let a = args(&f);
            run(&mut h, &acc, &f, a);
        }
    }

    for family in families {
        let (small, big) = (format!("{family}_small"), format!("{family}_big"));
        // Take the best of a few runs to reduce scheduler noise.
        let mut best = |f: &str| {
            (0..3)
                .map(|_| {
                    let a = args(f);
                    run(&mut h, &acc, f, a)
                })
                .min()
                .unwrap()
        };
        let small_time = best(&small);
        let big_time = best(&big);
        let ratio = big_time.as_secs_f64() / small_time.as_secs_f64();
        println!("{family}: {small_time:?} vs {big_time:?}, ratio: {ratio:.2}");
        // Loose threshold: catches O(type size) per-instruction work, not small regressions.
        assert!(
            ratio < 3.0,
            "{big} is {ratio:.2}x slower than {small}: cost scales with type size"
        );
    }
}
