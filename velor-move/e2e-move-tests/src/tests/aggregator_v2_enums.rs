// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aggregator_v2::AggV2TestHarness,
    tests::{aggregator_v2::AggregatorMode, common},
};
use velor_framework::BuildOptions;
use velor_language_e2e_tests::executor::ExecutorMode;
use velor_package_builder::PackageBuilder;
use velor_types::transaction::SignedTransaction;
use claims::{assert_ok, assert_some};
use move_core_types::parser::parse_struct_tag;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct Integer {
    value: u128,
    max_value: u128,
}

#[derive(Deserialize, Serialize)]
struct Aggregator {
    value: u128,
    max_value: u128,
}

#[derive(Deserialize, Serialize)]
enum Counter {
    Aggregator(Aggregator),
    Integer(Integer),
}

#[test]
fn test_aggregators_in_enums() {
    let mut h = make_harness(155);

    // Create a large block, where a counter is incremented 150 times. During the
    // test, we switch between parallel (aggregator) and non-parallel (integer)
    // implementations.
    let mut txns = vec![initialize(&mut h)];
    for _ in 0..50 {
        txns.push(increment(&mut h));
    }
    txns.push(switch(&mut h));
    for _ in 0..20 {
        txns.push(increment(&mut h));
    }
    txns.push(switch(&mut h));
    for _ in 0..50 {
        txns.push(increment(&mut h));
    }
    txns.push(switch(&mut h));
    for _ in 0..30 {
        txns.push(increment(&mut h));
    }
    txns.push(switch(&mut h));
    let outputs = h.run_block(txns);

    // All transactions must succeed.
    assert!(outputs.into_iter().all(|o| {
        let execution_status = assert_ok!(o.status().as_kept_status());
        execution_status.is_success()
    }));

    // Test the final value: it must be 150.
    let counter = assert_some!(h.harness.read_resource::<Counter>(
        h.account.address(),
        parse_struct_tag("0x1::enums_with_aggregators::Counter").unwrap(),
    ));
    let value = match counter {
        Counter::Aggregator(aggregator) => aggregator.value,
        Counter::Integer(_) => {
            unreachable!("Counter has to be an aggregator after even number of switches")
        },
    };
    assert_eq!(value, 150);
}

fn make_harness(num_txns: usize) -> AggV2TestHarness {
    let source = r"
    module 0x1::enums_with_aggregators {
      use velor_framework::aggregator_v2::{Self, Aggregator};

      struct Integer has store, drop {
        value: u128,
        max_value: u128,
      }

      fun add(integer: &mut Integer, value: u128) {
        integer.value = integer.value + value;
      }

      enum Counter has key, drop {
        Aggregator { aggregator: Aggregator<u128> },
        Integer { integer: Integer },
      }

      public entry fun initialize(account: &signer, parallel: bool) {
        let counter = if (parallel) {
          let aggregator = aggregator_v2::create_aggregator(1000);
          Counter::Aggregator { aggregator }
        } else {
          let integer = Integer { value: 0, max_value: 1000 };
          Counter::Integer { integer }
        };
        move_to(account, counter);
      }

      public entry fun increment(addr: address) acquires Counter {
        let counter = borrow_global_mut<Counter>(addr);
        match (counter) {
          Counter::Aggregator { aggregator } => {
            aggregator_v2::add(aggregator, 1);
          },
          Counter::Integer { integer } => {
            add(integer, 1);
          },
        }
      }

      public entry fun switch(addr: address) acquires Counter {
        let counter = borrow_global_mut<Counter>(addr);
        match (counter) {
          Counter::Aggregator { aggregator } => {
            let value = aggregator_v2::read(aggregator);
            let integer = Integer { value, max_value: 1000 };
            *counter = Counter::Integer { integer };
          },
          Counter::Integer { integer } => {
            let aggregator = aggregator_v2::create_aggregator(1000);
            aggregator_v2::add(&mut aggregator, integer.value);
            *counter = Counter::Aggregator { aggregator };
          },
        }
      }
    }
    ";

    // Create a package with testing code.
    let mut builder = PackageBuilder::new("enums_with_aggregators");
    builder.add_source("enums_with_aggregators.move", source);
    builder.add_local_dep(
        "VelorFramework",
        &common::framework_dir_path("velor-framework").to_string_lossy(),
    );
    let path = builder.write_to_temp().unwrap();

    let mut h = crate::tests::aggregator_v2::setup_allow_fallback(
        ExecutorMode::BothComparison,
        AggregatorMode::BothComparison,
        num_txns + 1,
    );

    // Publish the package to ensure subsequent tests can use that code.
    let txn = h.harness.create_publish_package(
        &h.account,
        path.path(),
        Some(BuildOptions::move_2()),
        |_| {},
    );
    h.run_block(vec![txn]);
    h
}

fn initialize(h: &mut AggV2TestHarness) -> SignedTransaction {
    h.harness.create_entry_function(
        &h.account,
        str::parse("0x1::enums_with_aggregators::initialize").unwrap(),
        vec![],
        vec![bcs::to_bytes(&true).unwrap()],
    )
}

fn increment(h: &mut AggV2TestHarness) -> SignedTransaction {
    h.harness.create_entry_function(
        &h.account,
        str::parse("0x1::enums_with_aggregators::increment").unwrap(),
        vec![],
        vec![bcs::to_bytes(h.account.address()).unwrap()],
    )
}

fn switch(h: &mut AggV2TestHarness) -> SignedTransaction {
    h.harness.create_entry_function(
        &h.account,
        str::parse("0x1::enums_with_aggregators::switch").unwrap(),
        vec![],
        vec![bcs::to_bytes(h.account.address()).unwrap()],
    )
}
