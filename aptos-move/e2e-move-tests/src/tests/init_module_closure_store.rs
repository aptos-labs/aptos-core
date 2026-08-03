// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Regression test for module upgrade when `init_module` stores a closure.
//!
//! The test checks the following flow:
//!   1. A private function exists in module A in package P.
//!   2. Package P is upgrade with changed A and new module B.
//!      A makes private function public.
//!      B has `init_module` that stores closure to this new
//!      public function (and therefore allowed).
//!   3. Transaction reads resources stored by `init_module`
//!      and executes the closure.
//!
//! The test ensures that it is NOT possible to load **old** version
//! of the function in A that is private.

use crate::{assert_abort, assert_success, tests::common, MoveHarness};
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_language_e2e_tests::{
    account::Account,
    executor::{ExecutorMode, FakeExecutor},
};
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::{FeatureFlag, TimedFeatureFlag},
    transaction::{SignedTransaction, TransactionStatus},
};
use test_case::test_case;

fn build_publish_a(h: &mut MoveHarness, acc: &Account) -> SignedTransaction {
    let source = r#"
        module 0xcafe::a {
            struct Func has copy, drop, store, key { bar: || }

            fun take_string(s: std::string::String) {
                aptos_std::string_utils::to_string(&s);
            }

            public fun foo(_s: &signer) {}

            entry public fun bar(s: address) {
                let f = borrow_global<Func>(s);
                (f.bar)();
            }
        }
    "#;

    let mut builder = PackageBuilder::new("package_a_b");
    builder.add_source("a.move", source);
    builder.add_local_dep(
        "AptosFramework",
        &common::framework_dir_path("aptos-framework").to_string_lossy(),
    );
    let path = builder.write_to_temp().unwrap();
    h.create_publish_package(acc, path.path(), Some(BuildOptions::move_2()), |_| {})
}

fn build_upgrade_a_and_publish_b(h: &mut MoveHarness, acc: &Account) -> SignedTransaction {
    let source_a_v2 = r#"
        module 0xcafe::a {
            // Same memory layout as String ({ bytes: vector<u8> })
            // but without the UTF-8 invariant.
            struct MyString has copy, drop, store {
                bytes: vector<u8>,
            }

            struct Func has copy, drop, store, key { bar: || }

            public fun take_string(_s: MyString) {
                // v2: no-op, safe with arbitrary bytes.
            }

            public fun foo(s: &signer) {
                // 0xff is not valid UTF-8.
                let x = MyString { bytes: b"\xff" };
                let f = Func { bar: || take_string(x) };
                move_to(s, f);
            }

            entry public fun bar(s: address) {
                let f = borrow_global<Func>(s);
                (f.bar)();
            }
        }
    "#;

    let source_b = r#"
        module 0xcafe::b {
            fun init_module(account: &signer) {
                0xcafe::a::foo(account);
            }
        }
    "#;

    let mut builder = PackageBuilder::new("package_a_b");
    builder.add_source("a.move", source_a_v2);
    builder.add_source("b.move", source_b);
    builder.add_local_dep(
        "AptosFramework",
        &common::framework_dir_path("aptos-framework").to_string_lossy(),
    );
    let path = builder.write_to_temp().unwrap();
    h.create_publish_package(acc, path.path(), Some(BuildOptions::move_2()), |_| {})
}

#[test]
fn poc_type_confusion_module_upgrade_parallel() {
    let executor =
        FakeExecutor::from_head_genesis().set_executor_mode(ExecutorMode::BothComparison);
    let mut h = MoveHarness::new_with_executor(executor);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    let mut block = vec![];
    block.push(build_publish_a(&mut h, &acc));
    block.push(build_upgrade_a_and_publish_b(&mut h, &acc));

    let txns = (0..20)
        .map(|i| {
            let sender = AccountAddress::from_hex_literal(&format!("0x{:x}", 0x1000 + i)).unwrap();
            let sender = h.new_account_at(sender);
            h.create_entry_function(
                &sender,
                str::parse("0xcafe::a::bar").unwrap(),
                vec![],
                vec![bcs::to_bytes(acc.address()).unwrap()],
            )
        })
        .collect::<Vec<_>>();
    block.extend(txns);

    // Execute repeatedly to trigger the race.
    for _ in 0..60 {
        let results = h.executor.execute_block(block.clone()).unwrap();
        for result in results {
            assert_success!(result.status().clone());
        }
    }
}

/// Module with a stored closure over the public (persistent) function `foo`. `INCREMENT` is
/// substituted to distinguish versions: v1 adds 1, v2 adds 1000.
const MODULE_A_TEMPLATE: &str = r#"
    module 0xcafe::a {
        use aptos_framework::code;
        use std::signer;

        struct Res has key { f: |u64|u64 has copy + store }

        public fun foo(x: u64): u64 {
            x + INCREMENT
        }

        public entry fun store(s: &signer) {
            let f: |u64|u64 has copy + drop + store = |x| foo(x);
            move_to(s, Res { f });
        }

        public fun call_stored(addr: address): u64 acquires Res {
            let r = borrow_global<Res>(addr);
            (r.f)(41)
        }

        public entry fun check(addr: address, expected: u64) acquires Res {
            assert!(call_stored(addr) == expected, 200);
        }

        public entry fun call_then_upgrade(
            owner: &signer,
            metadata: vector<u8>,
            code: vector<vector<u8>>,
        ) acquires Res {
            // Resolves (and memoizes) the stored closure under the pre-upgrade module version.
            assert!(call_stored(signer::address_of(owner)) == 42, 100);
            code::publish_package_txn(owner, metadata, code);
        }
    }
"#;

/// New module published together with the upgraded `a`. Its `init_module` runs inside the
/// publishing transaction and re-invokes the closure that was already resolved against the
/// pre-upgrade version of `a` earlier in the same transaction.
const MODULE_B: &str = r#"
    module 0xcafe::b {
        fun init_module(_s: &signer) {
            assert!(0xcafe::a::call_stored(@0xcafe) == 1041, 777);
        }
    }
"#;

// Returns BCS-serialized package metadata and module code, ready to be passed to
// `code::publish_package_txn`.
fn build_package_a(sources: Vec<(&str, String)>) -> (Vec<u8>, Vec<Vec<u8>>) {
    let mut builder = PackageBuilder::new("package_a");
    for (name, source) in sources {
        builder.add_source(name, &source);
    }
    builder.add_local_dep(
        "AptosFramework",
        &common::framework_dir_path("aptos-framework").to_string_lossy(),
    );
    let path = builder.write_to_temp().unwrap();
    let package = BuiltPackage::build(path.path().to_path_buf(), BuildOptions::move_2()).unwrap();
    let code = package.extract_code();
    let metadata = bcs::to_bytes(&package.extract_metadata().unwrap()).unwrap();
    (metadata, code)
}

/// Publishes `a` v1 and stores a closure over `a::foo` (v1: returns `x + 1`).
fn publish_a_v1_and_store_closure(h: &mut MoveHarness, acc: &Account) {
    let mut builder = PackageBuilder::new("package_a");
    builder.add_source("a.move", &MODULE_A_TEMPLATE.replace("INCREMENT", "1"));
    builder.add_local_dep(
        "AptosFramework",
        &common::framework_dir_path("aptos-framework").to_string_lossy(),
    );
    let path = builder.write_to_temp().unwrap();
    let txn = h.create_publish_package(acc, path.path(), Some(BuildOptions::move_2()), |_| {});
    assert_success!(h.run(txn));

    assert_success!(h.run_entry_function(
        acc,
        str::parse("0xcafe::a::store").unwrap(),
        vec![],
        vec![],
    ));
}

/// Runs a single transaction which calls the stored closure (memoizing its resolution) and then
/// upgrades `a` (v2: `foo` returns `x + 1000`) together with the new module `b`, whose
/// `init_module` asserts the closure result. Returns the transaction status.
fn run_call_then_upgrade(h: &mut MoveHarness, acc: &Account) -> TransactionStatus {
    let (metadata, code) = build_package_a(vec![
        ("a.move", MODULE_A_TEMPLATE.replace("INCREMENT", "1000")),
        ("b.move", MODULE_B.to_string()),
    ]);

    let txn = h.create_entry_function(
        acc,
        str::parse("0xcafe::a::call_then_upgrade").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&metadata).unwrap(),
            bcs::to_bytes(&code).unwrap(),
        ],
    );
    h.run(txn)
}

fn check_stored_closure_result(h: &mut MoveHarness, acc: &Account, expected: u64) {
    assert_success!(h.run_entry_function(
        acc,
        str::parse("0xcafe::a::check").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(acc.address()).unwrap(),
            bcs::to_bytes(&expected).unwrap(),
        ],
    ));
}

/// A closure resolved earlier in a transaction must be re-resolved when its defining module is
/// republished within the same transaction, so that `init_module` of a newly published module
/// observes the upgraded code (`foo` returns 1041, not the stale 42). Exercised under both lazy
/// and eager loading, which use different `unmetered_get_module_hash_and_size` implementations.
///
/// Uses a testnet harness so the `RevalidateResolvedClosures` timed feature has a real activation
/// date, then advances past it to enable the fix.
#[test_case(true; "lazy_loading")]
#[test_case(false; "eager_loading")]
fn stale_closure_memo_re_resolved_after_same_txn_upgrade(enable_lazy_loading: bool) {
    let mut h = MoveHarness::new_testnet();
    if enable_lazy_loading {
        h.enable_features(vec![FeatureFlag::ENABLE_LAZY_LOADING], vec![]);
    } else {
        h.enable_features(vec![], vec![FeatureFlag::ENABLE_LAZY_LOADING]);
    }
    h.set_timed_feature(TimedFeatureFlag::RevalidateResolvedClosures, true);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    publish_a_v1_and_store_closure(&mut h, &acc);
    check_stored_closure_result(&mut h, &acc, 42);

    let status = run_call_then_upgrade(&mut h, &acc);
    assert_success!(status);

    // After the upgrade is committed, the closure binds to v2 for later transactions as well.
    check_stored_closure_result(&mut h, &acc, 1041);
}

/// Before the `RevalidateResolvedClosures` timed feature activates, the memoized resolution is not
/// revalidated: the closure keeps executing the pre-upgrade code within the publishing
/// transaction, so `init_module` of module `b` aborts and the upgrade is rolled back.
#[test]
fn stale_closure_memo_kept_when_timed_feature_disabled() {
    let mut h = MoveHarness::new_testnet();
    h.set_timed_feature(TimedFeatureFlag::RevalidateResolvedClosures, false);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    publish_a_v1_and_store_closure(&mut h, &acc);
    check_stored_closure_result(&mut h, &acc, 42);

    let status = run_call_then_upgrade(&mut h, &acc);
    assert_abort!(status, 777);

    // The upgrade was rolled back, so the stored closure still binds to v1.
    check_stored_closure_result(&mut h, &acc, 42);
}
