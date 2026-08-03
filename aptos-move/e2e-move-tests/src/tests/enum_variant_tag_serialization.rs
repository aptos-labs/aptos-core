// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_language_e2e_tests::executor::{ExecutorMode, FakeExecutor};
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    on_chain_config::FeatureFlag,
    transaction::{ExecutionStatus, TransactionStatus},
};
use claims::assert_ok;
use move_core_types::account_address::AccountAddress;

const V1_A: &str = r#"
    module 0xcafe::a {
        use aptos_framework::aggregator_v2::{Self, Aggregator};
        use aptos_framework::code;
        use std::signer;

        enum Counter has key, drop {
            Aggregator { value: Aggregator<u64> },
            Empty,
        }

        public entry fun initialize(account: &signer) {
            move_to(account, Counter::Aggregator { value: aggregator_v2::create_aggregator(1000) });
        }

        public entry fun trigger(
            account: &signer,
            metadata: vector<u8>,
            pkg_code: vector<vector<u8>>,
        ) acquires Counter {
            // Pin the V1 layout in the data cache before staging V2.
            { let _c = borrow_global<Counter>(signer::address_of(account)); };
            code::publish_package_txn(account, metadata, pkg_code);
        }
    }
"#;

const V2_A: &str = r#"
    module 0xcafe::a {
        use aptos_framework::aggregator_v2::{Self, Aggregator};
        use aptos_framework::code;
        use std::signer;

        enum Counter has key, drop {
            Aggregator { value: Aggregator<u64> },
            Empty,
            NewEmpty,
        }

        public entry fun initialize(account: &signer) {
            move_to(account, Counter::Aggregator { value: aggregator_v2::create_aggregator(1000) });
        }

        public entry fun trigger(
            account: &signer,
            metadata: vector<u8>,
            pkg_code: vector<vector<u8>>,
        ) acquires Counter {
            { let _c = borrow_global<Counter>(signer::address_of(account)); };
            code::publish_package_txn(account, metadata, pkg_code);
        }

        public fun set_new_empty(addr: address) acquires Counter {
            let c = borrow_global_mut<Counter>(addr);
            *c = Counter::NewEmpty;
        }
    }
"#;

const V2_B: &str = r#"
    module 0xcafe::b {
        use std::signer;
        fun init_module(account: &signer) {
            0xcafe::a::set_new_empty(signer::address_of(account));
        }
    }
"#;

fn build_v2_package_bytes() -> (Vec<u8>, Vec<Vec<u8>>) {
    let mut builder = PackageBuilder::new("counter_pkg");
    builder.add_source("a.move", V2_A);
    builder.add_source("b.move", V2_B);
    builder.add_local_dep(
        "AptosFramework",
        &common::framework_dir_path("aptos-framework").to_string_lossy(),
    );
    let path = builder.write_to_temp().unwrap();

    let package = BuiltPackage::build(path.path().to_path_buf(), BuildOptions::move_2())
        .expect("building V2 package must succeed");
    let metadata = bcs::to_bytes(&package.extract_metadata().unwrap()).unwrap();
    (metadata, package.extract_code())
}

#[test]
fn out_of_range_variant_tag_does_not_stall_block() {
    let executor =
        FakeExecutor::from_head_genesis().set_executor_mode(ExecutorMode::SequentialOnly);
    let mut h = MoveHarness::new_with_executor(executor);
    h.enable_features(
        vec![
            FeatureFlag::AGGREGATOR_V2_API,
            FeatureFlag::AGGREGATOR_V2_DELAYED_FIELDS,
            FeatureFlag::RESOURCE_GROUPS_SPLIT_IN_VM_CHANGE_SET,
        ],
        vec![],
    );
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Publish V1 and create the `Counter::Aggregator` resource.
    let mut builder = PackageBuilder::new("counter_pkg");
    builder.add_source("a.move", V1_A);
    builder.add_local_dep(
        "AptosFramework",
        &common::framework_dir_path("aptos-framework").to_string_lossy(),
    );
    let v1_path = builder.write_to_temp().unwrap();
    let publish_v1 =
        h.create_publish_package(&acc, v1_path.path(), Some(BuildOptions::move_2()), |_| {});
    assert_success!(h.run_block(vec![publish_v1]).pop().unwrap());

    let initialize = h.create_entry_function(
        &acc,
        str::parse("0xcafe::a::initialize").unwrap(),
        vec![],
        vec![],
    );
    assert_success!(h.run_block(vec![initialize]).pop().unwrap());

    // Single transaction: borrow the V1 resource, upgrade to V2, and let V2's
    // `init_module` write the appended `NewEmpty` variant.
    let (metadata, code) = build_v2_package_bytes();
    let trigger = h.create_entry_function(
        &acc,
        str::parse("0xcafe::a::trigger").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&metadata).unwrap(),
            bcs::to_bytes(&code).unwrap(),
        ],
    );

    // The block must not return a fatal error, and the trigger transaction must
    // resolve to an isolated failure.
    let outputs = assert_ok!(h.executor.execute_block(vec![trigger]));
    assert_eq!(outputs.len(), 1);
    let status = outputs[0].status();
    assert!(
        !matches!(status, TransactionStatus::Keep(ExecutionStatus::Success)),
        "trigger transaction must fail cleanly, got: {:?}",
        status
    );
}
