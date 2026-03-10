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

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_language_e2e_tests::{
    account::Account,
    executor::{ExecutorMode, FakeExecutor},
};
use aptos_package_builder::PackageBuilder;
use aptos_types::{account_address::AccountAddress, transaction::SignedTransaction};

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
