// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    account_address::AccountAddress, move_utils::MemberId, on_chain_config::FeatureFlag,
};
use std::{path::Path, str::FromStr};

fn run_transactions(features: Vec<FeatureFlag>, path: &Path, function_names: &[&str]) -> u64 {
    let mut h = MoveHarness::new_with_features(features, vec![]);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    assert_success!(h.publish_package(&acc, path));

    h.run_entry_function(
        &acc,
        MemberId::from_str("0x123::test::init").unwrap(),
        vec![],
        vec![],
    );
    let mut txns = vec![];
    for name in function_names.iter() {
        txns.push(h.create_entry_function(
            &acc,
            MemberId::from_str(format!("0x123::test::{name}").as_str()).unwrap(),
            vec![],
            vec![],
        ));
    }
    let statuses = h.run_block_get_output(txns);
    statuses.iter().map(|out| out.gas_used()).sum()
}

fn check_for_transactions(path: &Path, function_names: &[&str]) {
    assert_eq!(
        run_transactions(vec![], path, function_names),
        run_transactions(
            vec![FeatureFlag::LIGHTWEIGHT_RESOURCE_EXISTENCE],
            path,
            function_names
        ),
    );
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

    check_for_transactions(path.path(), &["check"]);
    check_for_transactions(path.path(), &["modify"]);
    check_for_transactions(path.path(), &["check", "modify"]);
    check_for_transactions(path.path(), &["check", "check", "modify"]);
    check_for_transactions(path.path(), &["check", "modify", "check", "modify"]);
}
