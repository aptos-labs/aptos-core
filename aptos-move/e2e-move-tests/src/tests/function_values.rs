// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Smoke tests for function values (closures) introduced in Move 2.2. (Functional
//! tests are written as transactional tests elsewhere.)

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_package_builder::PackageBuilder;
use aptos_types::account_address::AccountAddress;

#[test]
fn function_value_registry() {
    let mut builder = PackageBuilder::new("Package");
    let source = r#"
        module 0x66::registry {
          use 0x1::signer;
          struct R<T>(T) has key;
          public fun store<T: store>(s: &signer, x: T) {
              move_to(s, R(x))
          }
          public fun remove<T: store>(s: &signer): T acquires R {
              let R(x) = move_from<R<T>>(signer::address_of(s));
              x
          }
        }

        module 0x66::delayed_work {
          use 0x66::registry;

          struct Work(|u64|u64 has store) has store;

          fun doit(self: Work): |u64|u64 {
              let Work(fn) = self;
              fn
          }

          entry fun initialize(s: &signer) {
              registry::store(s, Work(id_fun))
          }

          entry fun add(s: &signer, amount: u64) {
              let current = registry::remove<Work>(s);
              registry::store(s, Work(|x| more_work(current, amount, x)))
          }

          entry fun eval(s: &signer, amount: u64, expected: u64) {
              let todo = registry::remove<Work>(s);
              assert!(doit(todo)(amount) == expected)
          }

          public fun more_work(old: Work, x: u64, y: u64): u64 {
              doit(old)(x) + y
          }

          public fun id_fun(x: u64): u64 {
              x
          }
        }
    "#;
    builder.add_source("registry.move", source);
    builder.add_local_dep(
        "AptosFramework",
        &common::framework_dir_path("aptos-framework").to_string_lossy(),
    );
    let path = builder.write_to_temp().unwrap();

    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x66").unwrap());
    assert_success!(h.publish_package_with_options(
        &acc,
        path.path(),
        BuildOptions::move_2().set_latest_language()
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x66::delayed_work::initialize").unwrap(),
        vec![],
        vec![],
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x66::delayed_work::add").unwrap(),
        vec![],
        vec![bcs::to_bytes(&10u64).unwrap()],
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x66::delayed_work::add").unwrap(),
        vec![],
        vec![bcs::to_bytes(&5u64).unwrap()],
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x66::delayed_work::eval").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&11u64).unwrap(),
            bcs::to_bytes(&26u64).unwrap()
        ],
    ));
}
