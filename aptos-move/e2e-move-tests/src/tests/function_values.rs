// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Smoke tests for function values (closures) introduced in Move 2.2. (Functional
//! tests are written as transactional tests elsewhere.)

// Note[Orderless]: Done
use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_package_builder::PackageBuilder;
use rstest::rstest;

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn function_value_registry(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut builder = PackageBuilder::new("Package");
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let acc = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    let source = format!(
        r#"
        module {}::registry {{
          use 0x1::signer;
          struct R<T>(T) has key;
          public fun store<T: store>(s: &signer, x: T) {{
              move_to(s, R(x))
        }}
          public fun remove<T: store>(s: &signer): T acquires R {{
              let R(x) = move_from<R<T>>(signer::address_of(s));
              x
          }}
        }}

        module {}::delayed_work {{
          use {}::registry;

          struct Work(|u64|u64) has store;

          entry fun initialize(s: &signer) {{
              registry::store(s, Work(id_fun))
          }}

          entry fun add(s: &signer, amount: u64) {{
              let current = registry::remove<Work>(s);
              registry::store(s, Work(|x| more_work(current, amount, x)))
          }}

          entry fun eval(s: &signer, amount: u64, expected: u64) {{
              let todo = registry::remove<Work>(s);
              assert!(todo(amount) == expected)
          }}

          public fun more_work(old: Work, x: u64, y: u64): u64 {{
              old(x) + y
          }}

          public fun id_fun(x: u64): u64 {{
              x
          }}
        }}
    "#,
        acc.address(),
        acc.address(),
        acc.address()
    );
    builder.add_source("registry.move", source.as_str());
    builder.add_local_dep(
        "AptosFramework",
        &common::framework_dir_path("aptos-framework").to_string_lossy(),
    );
    let path = builder.write_to_temp().unwrap();

    assert_success!(h.publish_package_with_options(
        &acc,
        path.path(),
        BuildOptions::move_2().set_latest_language()
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse(format!("{}::delayed_work::initialize", acc.address()).as_str()).unwrap(),
        vec![],
        vec![],
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse(format!("{}::delayed_work::add", acc.address()).as_str()).unwrap(),
        vec![],
        vec![bcs::to_bytes(&10u64).unwrap()],
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse(format!("{}::delayed_work::add", acc.address()).as_str()).unwrap(),
        vec![],
        vec![bcs::to_bytes(&5u64).unwrap()],
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse(format!("{}::delayed_work::eval", acc.address()).as_str()).unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&11u64).unwrap(),
            bcs::to_bytes(&26u64).unwrap()
        ],
    ));
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn function_value_persistent(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut builder = PackageBuilder::new("Package");
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let acc = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });

    let source = format!(
        r#"
module {}::actions {{
    #[persistent]
    package fun incr(x: u64): u64 {{
        x + 1
    }}

    #[persistent]
    package fun decr(x: u64): u64 {{
        x - 1
    }}
}}

module {}::work {{
    use 0x1::signer::address_of;
    use {}::actions;

    struct Work(|u64|u64) has key, copy, drop;

    entry fun set(s: &signer, incr: bool) acquires Work {{
        if (exists<Work>(address_of(s))) {{
            move_from<Work>(address_of(s));
        }};
        if (incr) {{
            move_to(s, Work(actions::incr))
        }} else {{
            move_to(s, Work(actions::decr))
        }}
    }}

    entry fun exec(s: &signer, x: u64, r: u64) acquires Work {{
        // TODO: should be able to omit the parentheses in
        // `(Work[x])(y)` and instead write `Work[x](y)`
        assert!((Work[address_of(s)])(x) == r)
    }}
}}
    "#,
        acc.address(),
        acc.address(),
        acc.address()
    );
    builder.add_source("persistent.move", source.as_str());
    builder.add_local_dep(
        "AptosFramework",
        &common::framework_dir_path("aptos-framework").to_string_lossy(),
    );
    let path = builder.write_to_temp().unwrap();

    assert_success!(h.publish_package_with_options(
        &acc,
        path.path(),
        BuildOptions::move_2().set_latest_language()
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse(&format!("{}::work::set", acc.address())).unwrap(),
        vec![],
        vec![bcs::to_bytes(&true).unwrap()],
    ));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse(&format!("{}::work::exec", acc.address())).unwrap(),
        vec![],
        vec![bcs::to_bytes(&1u64).unwrap(), bcs::to_bytes(&2u64).unwrap()],
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse(&format!("{}::work::set", acc.address())).unwrap(),
        vec![],
        vec![bcs::to_bytes(&false).unwrap()],
    ));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse(&format!("{}::work::exec", acc.address())).unwrap(),
        vec![],
        vec![bcs::to_bytes(&2u64).unwrap(), bcs::to_bytes(&1u64).unwrap()],
    ));
}
