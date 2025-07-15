// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tests for using function values as keys in tables.

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_language_e2e_tests::account::Account;
use aptos_package_builder::PackageBuilder;
use aptos_types::{account_address::AccountAddress, transaction::TransactionStatus};

#[test]
fn fv_in_table() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x99").unwrap());

    // Initial publish
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x99::m {
            use aptos_std::table;
            use std::signer;

            struct Container<T1: copy+drop, T2> has key { t: table::Table<T1, T2> }

            #[persistent]
            fun foo(f: ||u64): u64 {
                if (f() == 1)
                    1
                else
                    2
            }
            #[persistent]
            fun bar(f: ||u64): u64 {
                if (f() == 1)
                    2
                else
                    1
            }

            // Stores a function value of type `| ||u64 |u64` with a key of the same type in a table.
            public entry fun test_store(account: &signer) {
                let f1: | ||u64 |u64 has copy+store+drop = |x| foo(x);
                let f2: | ||u64 |u64 has store+copy+drop = |x| foo(x);

                let table = table::new<| ||u64 |u64 has copy+store+drop, | ||u64 |u64 has store+copy+drop>();
                table::add(&mut table, f1, f2);
                move_to<Container<| ||u64 |u64 has copy+store+drop, | ||u64 |u64 has store+copy+drop>>(account, Container {t: table});
            }

            // Fecth a function value from the table and call it with different arguments.
            public entry fun test_fetch(account: &signer) {
                let f1: | ||u64 |u64 has copy+store+drop = |x| foo(x);
                let table = borrow_global<Container<| ||u64 |u64 has copy+store+drop, | ||u64 |u64 has store+copy+drop>>(signer::address_of(account));
                let f2 = table::borrow(&(table.t), f1);
                let arg = || 1;
                assert!((*f2)(arg) == 1, 0);
                let arg = || 2;
                assert!((*f2)(arg) == 2, 0);
            }

            // Test the non-existence of a key
            public entry fun not_contain(account: &signer){
                let f1: | ||u64 |u64 has copy+store+drop = |x| bar(x);
                let table = borrow_global<Container<| ||u64 |u64 has copy+store+drop, | ||u64 |u64 has store+copy+drop>>(signer::address_of(account));
                let contains_key = table::contains(&(table.t), f1);
                assert!(!contains_key, 0);
            }

            // Test the existence of a key
            public entry fun contain(account: &signer){
                let f1: | ||u64 |u64 has copy+store+drop = |x| foo(x);
                let table = borrow_global<Container<| ||u64 |u64 has copy+store+drop, | ||u64 |u64 has store+copy+drop>>(signer::address_of(account));
                let contains_key = table::contains(&(table.t), f1);
                assert!(contains_key, 0);
            }

            // Test the existence of a key (variant aspect 1: parameter name of function value used as key)
            // Expected result: no impact
            public entry fun contain_with_diff_param_name(account: &signer){
                let f1: | ||u64 |u64 has copy+store+drop = |y| foo(y);
                let table = borrow_global<Container<| ||u64 |u64 has copy+store+drop, | ||u64 |u64 has store+copy+drop>>(signer::address_of(account));
                let contains_key = table::contains(&(table.t), f1);
                assert!(contains_key, 0);
            }

            // Test updating a function value saved in table
            public entry fun update(account: &signer){
                let f1: | ||u64 |u64 has copy+store+drop = |x| foo(x);
                let table = borrow_global_mut<Container<| ||u64 |u64 has copy+store+drop, | ||u64 |u64 has store+copy+drop>>(signer::address_of(account));

                let f2: | ||u64 |u64 has store+copy+drop = |x| bar(x);
                table::upsert(&mut (table.t), f1, f2);

                let f2 = table::borrow(&(table.t), f1);
                let arg = || 1;
                assert!((*f2)(arg) == 2, 0);
                let arg = || 2;
                assert!((*f2)(arg) == 1, 0);
            }

            // Test removing a function value saved in table
            public entry fun remove(account: &signer){
                let f1: | ||u64 |u64 has copy+store+drop = |x| foo(x);
                let table = borrow_global_mut<Container<| ||u64 |u64 has copy+store+drop, | ||u64 |u64 has store+copy+drop>>(signer::address_of(account));
                table::remove(&mut (table.t), f1);
                let contains_key = table::contains(&(table.t), f1);
                assert!(!contains_key, 0);
            }
        }
        "#,
    );
    assert_success!(result);
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::m::test_store").unwrap(),
        vec![],
        vec![],
    ));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::m::test_fetch").unwrap(),
        vec![],
        vec![],
    ));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::m::not_contain").unwrap(),
        vec![],
        vec![],
    ));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::m::contain").unwrap(),
        vec![],
        vec![],
    ));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::m::contain_with_diff_param_name").unwrap(),
        vec![],
        vec![],
    ));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::m::update").unwrap(),
        vec![],
        vec![],
    ));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::m::remove").unwrap(),
        vec![],
        vec![],
    ));
}

#[test]
fn fv_in_table_with_refs() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x99").unwrap());

    // Initial publish
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x99::m1 {
            use aptos_std::table;
            use std::signer;
            use std::vector;

            struct Container<T1: copy+drop, T2> has key { t: table::Table<T1, T2> }

            #[persistent]
            fun foo(_f: &||u64, x: &u64): &u64 {
                x
            }

            // Stores a function value of type `| &||u64 | &||u64` with a key of the same type in a table.
            public entry fun test_store(account: &signer) {
                let f1: | &||u64, &u64|&u64 has copy+store+drop = |f, x| foo(f, x);
                let f2: | &||u64, &u64|&u64 has store+copy+drop = |f, x| foo(f, x);

                let table = table::new<| &||u64, &u64|&u64 has copy+store+drop, | &||u64, &u64|&u64 has copy+store+drop>();
                table::add(&mut table, f1, f2);
                move_to<Container<| &||u64, &u64|&u64 has copy+store+drop, | &||u64, &u64|&u64 has copy+store+drop>>(account, Container {t: table});
            }

            // Test the existence of a key
            public entry fun contain(account: &signer){
                let f1: | &||u64, &u64|&u64 has copy+store+drop = |g, y| foo(g, y);
                let table = borrow_global<Container<| &||u64, &u64|&u64 has copy+store+drop, | &||u64, &u64|&u64 has copy+store+drop>>(signer::address_of(account));
                let contains_key = table::contains(&(table.t), f1);
                assert!(contains_key, 0);
            }

            // Test saving references via function value args in vector
            public entry fun ref_in_vec(){
                let f1: | &||u64, &u64|&u64 has copy+store+drop = |g, y| foo(g, y);
                let f2: | &||u64, &u64|&u64 has copy+store+drop = |g, y| foo(g, y);
                let v = vector::empty<| &||u64, &u64|&u64 has copy+store+drop>();
                vector::push_back(&mut v, f1);
                vector::push_back(&mut v, f2);
                assert!(v[0] == v[1]);
            }

            // Test the existence of a key saved in a vector
            public entry fun contain_via_vec(account: &signer){
                let f1: | &||u64, &u64|&u64 has copy+store+drop = |g, y| foo(g, y);
                let f2: | &||u64, &u64|&u64 has copy+store+drop = |g, y| foo(g, y);
                let v = vector::empty<| &||u64, &u64|&u64 has copy+store+drop>();
                vector::push_back(&mut v, f1);
                vector::push_back(&mut v, f2);
                let table = borrow_global<Container<| &||u64, &u64|&u64 has copy+store+drop, | &||u64, &u64|&u64 has copy+store+drop>>(signer::address_of(account));
                let contains_key = table::contains(&(table.t), v[0]);
                assert!(contains_key, 0);
            }

            // Fecth a function value from the table and check the result.
            public entry fun test_fetch(account: &signer) {
                let f1: | &||u64, &u64|&u64 has copy+store+drop = |g, y| foo(g, y);
                let table = borrow_global<Container<| &||u64, &u64|&u64 has copy+store+drop, | &||u64, &u64|&u64 has copy+store+drop>>(signer::address_of(account));
                let f2 = table::borrow(&(table.t), f1);
                assert!(*f2 == f1);
            }
        }
        "#,
    );
    assert_success!(result);
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::m1::test_store").unwrap(),
        vec![],
        vec![],
    ));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::m1::contain").unwrap(),
        vec![],
        vec![],
    ));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::m1::ref_in_vec").unwrap(),
        vec![],
        vec![],
    ));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::m1::contain_via_vec").unwrap(),
        vec![],
        vec![],
    ));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::m1::test_fetch").unwrap(),
        vec![],
        vec![],
    ));
}

#[test]
fn fv_in_table_with_captured_vars() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x99").unwrap());

    // Initial publish
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x99::m2 {
            use aptos_std::table;
            use std::signer;

            struct Container<T1: copy+drop, T2> has key { t: table::Table<T1, T2> }

            #[persistent]
            fun foo<T: copy+drop>(x: u64, _y: T):u64 {
                if (x == 1)
                    1
                else
                    2
            }

            #[persistent]
            fun bar<T: copy+drop>(x: u64, _y: T):u64 {
                if (x == 1)
                    2
                else
                    1
            }

            // Stores a function value of type `|u64, u64|u64` with a captured variable of a generic type, using a key of the same type in a table.
            public entry fun test_store(account: &signer) {
                let y = 1;
                let f1: |u64|u64 has copy+store+drop = |x| foo(x, y);
                let f2: |u64|u64 has copy+store+drop = |x| foo(x, y);

                let table = table::new<|u64|u64 has copy+store+drop, |u64|u64 has copy+store+drop>();
                table::add(&mut table, f1, f2);
                move_to<Container<|u64|u64 has copy+store+drop, |u64|u64 has copy+store+drop>>(account, Container {t: table});
            }

            // Test the existence of a key
            public entry fun contain(account: &signer){
                let y = 1;
                let f1: |u64|u64 has copy+store+drop = |x| foo(x, y);
                let table = borrow_global<Container<|u64|u64 has copy+store+drop, |u64|u64 has copy+store+drop>>(signer::address_of(account));
                let contains_key = table::contains(&(table.t), f1);
                assert!(contains_key, 0);
            }

            // Test the existence of a key (variant 1: different captured variable name)
            public entry fun contain_var1(account: &signer){
                let z = 1;
                let f1: |u64|u64 has copy+store+drop = |x| foo(x, z);
                let table = borrow_global<Container<|u64|u64 has copy+store+drop, |u64|u64 has copy+store+drop>>(signer::address_of(account));
                let contains_key = table::contains(&(table.t), f1);
                assert!(contains_key, 0);
            }

            // Test the non-existence of a key (variant 1: different captured variable value)
            public entry fun not_contain_var1(account: &signer){
                let z = 2;
                let f1: |u64|u64 has copy+store+drop = |x| foo(x, z);
                let table = borrow_global<Container<|u64|u64 has copy+store+drop, |u64|u64 has copy+store+drop>>(signer::address_of(account));
                let contains_key = table::contains(&(table.t), f1);
                assert!(!contains_key, 0);
            }

            // Test the non-existence of a key (variant 2: different captured variable type)
            public entry fun not_contain_var2(account: &signer){
                let a1: address = @0x1;
                let f1: |u64|u64 has copy+store+drop = |x| foo(x, a1);
                let table = borrow_global<Container<|u64|u64 has copy+store+drop, |u64|u64 has copy+store+drop>>(signer::address_of(account));
                let contains_key = table::contains(&(table.t), f1);
                assert!(!contains_key, 0);
            }

            // Test updating a function value saved in table
            public entry fun update(account: &signer) {
                let table = borrow_global_mut<Container<|u64|u64 has copy+store+drop, |u64|u64 has copy+store+drop>>(signer::address_of(account));

                // check the original value
                let z = 1;
                let f1: |u64|u64 has copy+store+drop = |x| foo(x, z);
                let f2 = table::borrow(&(table.t), f1);
                assert!((*f2)(1) == 1, 0);

                // update the value
                let f2: |u64|u64 has copy+store+drop = |x| bar(x, z);
                table::upsert(&mut (table.t), f1, f2);

                // check the updated value
                let f2 = table::borrow(&(table.t), f1);
                assert!((*f2)(1) == 2, 0);
            }

            // Test removing a function value saved in table
            public entry fun remove(account: &signer){
                let table = borrow_global_mut<Container<|u64|u64 has copy+store+drop, |u64|u64 has copy+store+drop>>(signer::address_of(account));
                let z = 1;
                let f1: |u64|u64 has copy+store+drop = |x| foo(x, z);
                table::remove(&mut (table.t), f1);
                let contains_key = table::contains(&(table.t), f1);
                assert!(!contains_key, 0);
            }
        }
        "#,
    );
    assert_success!(result);
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::m2::test_store").unwrap(),
        vec![],
        vec![],
    ));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::m2::contain").unwrap(),
        vec![],
        vec![],
    ));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::m2::contain_var1").unwrap(),
        vec![],
        vec![],
    ));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::m2::not_contain_var1").unwrap(),
        vec![],
        vec![],
    ));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::m2::not_contain_var2").unwrap(),
        vec![],
        vec![],
    ));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::m2::update").unwrap(),
        vec![],
        vec![],
    ));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x99::m2::remove").unwrap(),
        vec![],
        vec![],
    ));
}

fn publish(h: &mut MoveHarness, account: &Account, source: &str) -> TransactionStatus {
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);
    builder.add_local_dep(
        "AptosStdlib",
        &common::framework_dir_path("aptos-stdlib").to_string_lossy(),
    );
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
