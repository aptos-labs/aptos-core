// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tests for enum type upgrade compatibility

use crate::{assert_success, assert_vm_status, tests::common, MoveHarness};
use velor_framework::BuildOptions;
use velor_language_e2e_tests::account::Account;
use velor_package_builder::PackageBuilder;
use velor_types::{account_address::AccountAddress, transaction::TransactionStatus};
use move_core_types::vm_status::StatusCode;

#[test]
fn enum_upgrade() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x815").unwrap());

    // Initial publish
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m {
            enum Data<T> has key {
               V1{x: ||T has copy + store }
            }
        }
    "#,
    );
    assert_success!(result);

    // incompatible variant
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m {
            enum Data<T> has key {
               V1 {x: ||T has store}
            }
        }
    "#,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    // identical variant
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m {
            enum Data<T> has key {
               V1 {x: ||T has copy + store + drop}
            }
        }
    "#,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    // prepare test for executing function value stored in an enum
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m {
            use std::signer;
            enum Data<T> has key {
               V1 {x: ||T has copy + store}
            }

            public fun make_data<T>(f: ||T has copy + drop + store): Data<T> {
                Data::V1 {x: f}
            }

            public fun store_v1<T: store>(s: &signer, data: Data<T>) {
                move_to(s, data);
            }

            public fun retrieve_data_and_execute<T:store>(s: &signer): T {
                let data = borrow_global<Data<T>>(signer::address_of(s));
                (data.x)()
            }

        }

        module 0x815::n {
            use 0x815::m;

            public fun f(x: u64): u64 {
                x + 3
            }

            public entry fun create_store_data(s: &signer) {
                let k = 3;
                let f: ||u64 has copy + drop + store = || f(k);
                m::store_v1(s, m::make_data(f));
            }

            public entry fun execute_stored_data(s: &signer) {
                assert!(m::retrieve_data_and_execute<u64>(s) == 6, 99);
            }

        }
    "#,
    );
    assert_success!(result);

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x815::n::create_store_data").unwrap(),
        vec![],
        vec![],
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x815::n::execute_stored_data").unwrap(),
        vec![],
        vec![],
    ));

    // update enum with a new variant
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m {
            use std::signer;
            enum Data<T> has key {
               V1 {x: ||T has copy + store},
               V2 {x1: ||T has copy + drop + store, x: ||T has copy + store}
            }

            public fun make_data<T>(f: ||T has copy + drop + store): Data<T> {
                Data::V1 {x: f}
            }

            public fun make_data_v2<T>(f1: ||T has copy + drop + store, f2: ||T has copy + drop + store): Data<T> {
                Data::V2 {x1: f1, x: f2}
            }

            public fun store_v1<T: store>(s: &signer, data: Data<T>) {
                move_to(s, data);
            }

            public fun retrieve_data_and_execute<T:store>(s: &signer): T {
                let data = borrow_global<Data<T>>(signer::address_of(s));
                (data.x)()
            }

        }

        module 0x815::n {
            use 0x815::m;

            public fun f(x: u64): u64 {
                x + 3
            }

            public entry fun create_store_data(s: &signer) {
                let k = 3;
                let f: ||u64 has copy + drop + store = || f(k);
                m::store_v1(s, m::make_data(f));
            }

            public entry fun execute_stored_data(s: &signer) {
                assert!(m::retrieve_data_and_execute<u64>(s) == 6, 99);
            }

        }
    "#,
    );
    assert_success!(result);

    // execution still exceeds after enum upgrade
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x815::n::execute_stored_data").unwrap(),
        vec![],
        vec![],
    ));
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
