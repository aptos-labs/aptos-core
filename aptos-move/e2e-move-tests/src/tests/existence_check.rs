// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_package_builder::PackageBuilder;
use aptos_transaction_simulation::Account;
use aptos_types::{
    account_address::AccountAddress, move_utils::MemberId, on_chain_config::FeatureFlag,
    transaction::TransactionOutput,
};
use std::{path::Path, str::FromStr};

fn make_harness_and_account(
    path: &Path,
    enabled_features: Vec<FeatureFlag>,
    disabled_features: Vec<FeatureFlag>,
) -> (MoveHarness, Account) {
    let mut harness = MoveHarness::new_with_features(enabled_features, disabled_features);
    let acc = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    assert_success!(harness.publish_package(&acc, path));
    harness.run_entry_function(
        &acc,
        MemberId::from_str("0x123::test::init").unwrap(),
        vec![],
        vec![],
    );
    (harness, acc)
}

fn run_transactions(
    harness: &mut MoveHarness,
    acc: &mut Account,
    function_names: &[&str],
) -> Vec<TransactionOutput> {
    let txns = function_names
        .iter()
        .map(|name| {
            harness.create_entry_function(
                acc,
                MemberId::from_str(format!("0x123::test::{name}").as_str()).unwrap(),
                vec![],
                vec![],
            )
        })
        .collect();
    harness.run_block_get_output(txns)
}

#[test]
fn test_lightweight_resource_existence() {
    let source = r#"
    module 0x123::test {
        struct T has key {
            a: u64,
            b: 0x1::string::String,
        }

        #[event]
        struct DummyEvent has drop, store {}

        public entry fun init(publisher: &signer) {
            move_to<T>(publisher, T{ a: 239, b: 0x1::string::utf8(b"Lorem ipsum") });
        }

        public entry fun check() {
            if (exists<T>(@0x123)) {
                0x1::event::emit(DummyEvent{});
            }
        }

        public entry fun read() acquires T {
            if (exists<T>(@0x123)) {
                let _ = borrow_global<T>(@0x123);
            }
        }

        public entry fun modify() acquires T {
            let t = borrow_global_mut<T>(@0x123);
            t.a = t.a + 1;
        }
    }
    "#;

    let mut builder = PackageBuilder::new("P1");
    builder.add_source("test.move", source);
    builder.add_local_dep(
        "AptosFramework",
        &common::framework_dir_path("aptos-framework").to_string_lossy(),
    );
    let path = builder.write_to_temp().unwrap();

    let (mut harness_no_feat, mut acc_no_feat) =
        make_harness_and_account(path.path(), vec![], vec![
            FeatureFlag::LIGHTWEIGHT_RESOURCE_EXISTENCE,
        ]);
    let (mut harness_with_feat, mut acc_with_feat) = make_harness_and_account(
        path.path(),
        vec![FeatureFlag::LIGHTWEIGHT_RESOURCE_EXISTENCE],
        vec![],
    );

    let mut check_for_transactions = |function_names| {
        assert_eq!(
            run_transactions(&mut harness_no_feat, &mut acc_no_feat, function_names),
            run_transactions(&mut harness_with_feat, &mut acc_with_feat, function_names),
        );
    };

    check_for_transactions(&["check"]);
    check_for_transactions(&["modify"]);
    check_for_transactions(&["check", "modify"]);
    check_for_transactions(&["check", "check", "modify"]);
    check_for_transactions(&["check", "modify", "check", "modify"]);
    check_for_transactions(&["read"]);
    check_for_transactions(&["check", "read"]);
}
