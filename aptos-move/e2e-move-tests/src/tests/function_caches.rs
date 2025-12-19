// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests that interpreter caches frees all allocated data structures for recursively called
//! functions.

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_package_builder::PackageBuilder;
use aptos_transaction_simulation::Account;
use aptos_types::transaction::TransactionStatus;
use move_core_types::account_address::AccountAddress;

#[test]
fn test_function_caches_for_recursive_functions_do_not_leak_memory() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x99").unwrap());

    let status = publish(
        &mut h,
        &acc,
        r#"
        module 0x99::m {
            public entry fun factorial(n: u64) {
                factorial_impl(n);
            }

            fun factorial_impl(n: u64): u64 {
                if (n <= 1) {
                    1
                } else {
                    n * factorial_impl(n - 1)
                }
            }
        }
        "#,
    );
    assert_success!(status);

    // Warmup.
    run_factorial(&mut h, &acc);

    // Memory growth later in time should be smaller than at the beginning.
    let a = run_factorial_measure_memory_growth(&mut h, &acc);
    let b = run_factorial_measure_memory_growth(&mut h, &acc);
    assert!(b <= a);
}

fn run_factorial(h: &mut MoveHarness, acc: &Account) {
    for _ in 0..300 {
        let status = h.run_entry_function(
            acc,
            str::parse("0x99::m::factorial").unwrap(),
            vec![],
            vec![bcs::to_bytes(&4_u64).unwrap()],
        );
        assert_success!(status);
    }
}

fn run_factorial_measure_memory_growth(h: &mut MoveHarness, acc: &Account) -> usize {
    let start = memory_stats::memory_stats().unwrap().virtual_mem;
    run_factorial(h, acc);
    memory_stats::memory_stats()
        .unwrap()
        .virtual_mem
        .saturating_sub(start)
}

fn publish(h: &mut MoveHarness, account: &Account, source: &str) -> TransactionStatus {
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);
    builder.add_local_dep(
        "MoveStdlib",
        &common::framework_dir_path("move-stdlib").to_string_lossy(),
    );
    let path = builder.write_to_temp().unwrap();
    h.publish_package_with_options(
        account,
        path.path(),
        BuildOptions::move_2().set_latest_language(),
    )
}
