// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tests for capturing option type in function values.

use crate::{assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_package_builder::PackageBuilder;
use aptos_transaction_simulation::Account;
use aptos_types::{
    on_chain_config::FeatureFlag, transaction::TransactionStatus, vm_status::StatusCode,
};
use move_core_types::account_address::AccountAddress;

#[test]
fn test_vm_value_fv_capture_option_1() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x99").unwrap());

    // test directly capture option type
    h.enable_features(vec![], vec![FeatureFlag::ENABLE_CAPTURE_OPTION]);
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x99::m {
            use std::option;

            struct FunctionStore has key {
                f: ||option::Option<u64> has copy+drop+store,
            }

            public fun id(x: option::Option<u64>): option::Option<u64> {
                x
            }

            fun init_module(account: &signer) {
                let v = option::none();
                let f: ||option::Option<u64> has copy+drop+store = || id(v);
                move_to(account, FunctionStore { f });
            }
            entry fun entry_func() {
                let v = option::none();
                let _f: ||option::Option<u64> has copy+drop+store = || id(v);
            }
        }
        "#,
    );
    // before timed feature flag is enabled
    assert_success!(result);

    h.new_epoch();
    let result = h.run_entry_function(
        &acc,
        str::parse("0x99::m::entry_func").unwrap(),
        vec![],
        vec![],
    );
    assert_vm_status!(result, StatusCode::UNABLE_TO_CAPTURE_OPTION_TYPE);

    h.enable_features(vec![FeatureFlag::ENABLE_CAPTURE_OPTION], vec![]);
    let result = h.run_entry_function(
        &acc,
        str::parse("0x99::m::entry_func").unwrap(),
        vec![],
        vec![],
    );
    assert_success!(result);
}

#[test]
fn test_vm_value_fv_capture_option_2() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x99").unwrap());

    // test capture option type in a struct
    h.enable_features(vec![], vec![FeatureFlag::ENABLE_CAPTURE_OPTION]);
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x99::m {
            use std::option;

            struct OptionStore has copy,drop,store {
                o: option::Option<u64>,
            }

            struct FunctionStore has key {
                f: ||option::Option<u64> has copy+drop+store,
            }

            public fun id(x: OptionStore): option::Option<u64> {
                x.o
            }

            entry fun entry_func() {
                let v = OptionStore { o: option::none() };
                let _f: ||option::Option<u64> has copy+drop+store = || id(v);
            }
        }
        "#,
    );
    // before timed feature flag is enabled
    assert_success!(result);
    h.new_epoch();
    let result = h.run_entry_function(
        &acc,
        str::parse("0x99::m::entry_func").unwrap(),
        vec![],
        vec![],
    );
    assert_vm_status!(result, StatusCode::UNABLE_TO_CAPTURE_OPTION_TYPE);

    h.enable_features(vec![FeatureFlag::ENABLE_CAPTURE_OPTION], vec![]);
    let result = h.run_entry_function(
        &acc,
        str::parse("0x99::m::entry_func").unwrap(),
        vec![],
        vec![],
    );
    assert_success!(result);
}

#[test]
fn test_vm_value_fv_capture_option_3() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x99").unwrap());

    // test capture option type in a vector, even if the vector is empty
    h.enable_features(vec![], vec![FeatureFlag::ENABLE_CAPTURE_OPTION]);
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x99::m {
            use std::option;
            use std::vector;

            struct FunctionStore has key {
                f: ||option::Option<u64> has copy+drop+store,
            }

            public fun id(_x: vector<option::Option<u64>>): option::Option<u64> {
                option::none()
            }

            entry fun entry_func() {
                let v = vector::empty();
                let _f: ||option::Option<u64> has copy+drop+store = || id(v);
            }
        }
        "#,
    );
    // before timed feature flag is enabled
    assert_success!(result);
    h.new_epoch();
    let result = h.run_entry_function(
        &acc,
        str::parse("0x99::m::entry_func").unwrap(),
        vec![],
        vec![],
    );
    assert_vm_status!(result, StatusCode::UNABLE_TO_CAPTURE_OPTION_TYPE);

    h.enable_features(vec![FeatureFlag::ENABLE_CAPTURE_OPTION], vec![]);
    let result = h.run_entry_function(
        &acc,
        str::parse("0x99::m::entry_func").unwrap(),
        vec![],
        vec![],
    );
    assert_success!(result);
}

#[test]
fn test_vm_value_fv_capture_option_4() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x99").unwrap());

    // test capture option type in a struct, parameterized by an option type
    h.enable_features(vec![], vec![FeatureFlag::ENABLE_CAPTURE_OPTION]);
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x99::m {
            use std::option;

            struct Store<T: store+drop+copy> has copy,drop,store {
                o: T,
            }

            struct FunctionStore has key {
                f: ||option::Option<u64> has copy+drop+store,
            }

            public fun id<T: store+drop+copy>(x: Store<T>): T {
                x.o
            }

            fun init_module(account: &signer) {
                let v = Store { o: option::none() };
                let f: ||option::Option<u64> has copy+drop+store = || id(v);
                move_to(account, FunctionStore { f });
            }

            entry fun entry_func() {
                let v = Store { o: option::none() };
                let _f: ||option::Option<u64> has copy+drop+store = || id(v);
            }
        }
        "#,
    );
    // before timed feature flag is enabled
    assert_success!(result);
    h.new_epoch();
    let result = h.run_entry_function(
        &acc,
        str::parse("0x99::m::entry_func").unwrap(),
        vec![],
        vec![],
    );
    assert_vm_status!(result, StatusCode::UNABLE_TO_CAPTURE_OPTION_TYPE);

    h.enable_features(vec![FeatureFlag::ENABLE_CAPTURE_OPTION], vec![]);
    let result = h.run_entry_function(
        &acc,
        str::parse("0x99::m::entry_func").unwrap(),
        vec![],
        vec![],
    );
    assert_success!(result);
}

#[test]
fn test_vm_value_fv_capture_option_5() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x99").unwrap());

    // test capture option type in a function type
    h.enable_features(vec![], vec![FeatureFlag::ENABLE_CAPTURE_OPTION]);
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x99::m {
            use std::option;


            struct FunctionStore has key {
                f: ||u64 has copy+drop+store,
            }

            public fun id(f: |option::Option<u64>| u64 has copy+drop+store): u64 {
                f(option::some(3))
            }

            public fun id2(_v: option::Option<u64>): u64 {
                3
            }

            fun init_module(account: &signer) {
                let v :|option::Option<u64>|u64 has copy+drop+store = id2;
                let f: ||u64 has copy+drop+store = || id(v);
                move_to(account, FunctionStore { f });
            }

            entry fun entry_func() {
                let v :|option::Option<u64>|u64 has copy+drop+store = id2;
                let _f: ||u64 has copy+drop+store = || id(v);
            }
        }
        "#,
    );
    // before timed feature flag is enabled
    assert_success!(result);
    h.new_epoch();
    let result = h.run_entry_function(
        &acc,
        str::parse("0x99::m::entry_func").unwrap(),
        vec![],
        vec![],
    );
    // since arguments of function type do not show in the layout, execution should succeed
    assert_success!(result);

    h.enable_features(vec![FeatureFlag::ENABLE_CAPTURE_OPTION], vec![]);
    let result = h.run_entry_function(
        &acc,
        str::parse("0x99::m::entry_func").unwrap(),
        vec![],
        vec![],
    );
    assert_success!(result);
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
