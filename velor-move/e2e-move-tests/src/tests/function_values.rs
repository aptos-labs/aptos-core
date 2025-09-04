// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! Smoke tests for function values (closures) introduced in Move 2.2. (Functional
//! tests are written as transactional tests elsewhere.)

use crate::{assert_success, tests::common, MoveHarness};
use velor_framework::BuildOptions;
use velor_package_builder::PackageBuilder;
use velor_types::account_address::AccountAddress;

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

          struct Work(|u64|u64) has store;

          entry fun initialize(s: &signer) {
              registry::store(s, Work(id_fun))
          }

          entry fun add(s: &signer, amount: u64) {
              let current = registry::remove<Work>(s);
              registry::store(s, Work(|x| more_work(current, amount, x)))
          }

          entry fun eval(s: &signer, amount: u64, expected: u64) {
              let todo = registry::remove<Work>(s);
              assert!(todo(amount) == expected)
          }

          public fun more_work(old: Work, x: u64, y: u64): u64 {
              old(x) + y
          }

          public fun id_fun(x: u64): u64 {
              x
          }
        }
    "#;
    builder.add_source("registry.move", source);
    builder.add_local_dep(
        "VelorFramework",
        &common::framework_dir_path("velor-framework").to_string_lossy(),
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

#[test]
fn function_value_persistent() {
    let mut builder = PackageBuilder::new("Package");
    let source = r#"
module 0x66::actions {
    #[persistent]
    package fun incr(x: u64): u64 {
        x + 1
    }

    #[persistent]
    package fun decr(x: u64): u64 {
        x - 1
    }
}

module 0x66::work {
    use 0x1::signer::address_of;
    use 0x66::actions;

    struct Work(|u64|u64) has key, copy, drop;

    entry fun set(s: &signer, incr: bool) acquires Work {
        if (exists<Work>(address_of(s))) {
            move_from<Work>(address_of(s));
        };
        if (incr) {
            move_to(s, Work(actions::incr))
        } else {
            move_to(s, Work(actions::decr))
        }
    }

    entry fun exec(s: &signer, x: u64, r: u64) acquires Work {
        // TODO: should be able to omit the parentheses in
        // `(Work[x])(y)` and instead write `Work[x](y)`
        assert!((Work[address_of(s)])(x) == r)
    }
}
    "#;
    builder.add_source("persistent.move", source);
    builder.add_local_dep(
        "VelorFramework",
        &common::framework_dir_path("velor-framework").to_string_lossy(),
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
        str::parse("0x66::work::set").unwrap(),
        vec![],
        vec![bcs::to_bytes(&true).unwrap()],
    ));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x66::work::exec").unwrap(),
        vec![],
        vec![bcs::to_bytes(&1u64).unwrap(), bcs::to_bytes(&2u64).unwrap()],
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x66::work::set").unwrap(),
        vec![],
        vec![bcs::to_bytes(&false).unwrap()],
    ));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0x66::work::exec").unwrap(),
        vec![],
        vec![bcs::to_bytes(&2u64).unwrap(), bcs::to_bytes(&1u64).unwrap()],
    ));
}
