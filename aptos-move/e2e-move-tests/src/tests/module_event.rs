// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_package_builder::PackageBuilder;
use aptos_types::{account_address::AccountAddress, on_chain_config::FeatureFlag};
use move_core_types::vm_status::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
struct Field {
    field: bool,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
struct MyEvent {
    seq: u64,
    field: Field,
    bytes: Vec<u64>,
}

#[test]
fn test_module_event_enabled() {
    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::MODULE_EVENT], vec![]);

    let addr = AccountAddress::from_hex_literal("0xcafe").unwrap();
    let account = h.new_account_at(addr);

    let mut build_options = aptos_framework::BuildOptions::default();
    build_options
        .named_addresses
        .insert("event".to_string(), addr);

    let result = h.publish_package_with_options(
        &account,
        &common::test_dir_path("../../../move-examples/event"),
        build_options.clone(),
    );
    assert_success!(result);
    h.run_entry_function(
        &account,
        str::parse("0xcafe::event::emit").unwrap(),
        vec![],
        vec![bcs::to_bytes(&5u64).unwrap()],
    );
    let events = h.get_events();
    let struct_tags = vec![
        "0x1::code::PublishPackage",
        "0x1::fungible_asset::Withdraw",
        "0x1::transaction_fee::FeeStatement",
        "0xcafe::event::MyEvent",
        "0xcafe::event::MyEvent",
        "0xcafe::event::MyEvent",
        "0xcafe::event::MyEvent",
        "0xcafe::event::MyEvent",
        "0x1::fungible_asset::Withdraw",
        "0x1::transaction_fee::FeeStatement",
    ];
    assert_eq!(
        events
            .iter()
            .map(|e| e.type_tag().to_string())
            .collect::<Vec<_>>(),
        struct_tags
    );
}

#[test]
fn verify_module_event_upgrades() {
    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::MODULE_EVENT], vec![]);
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap());

    // Initial code
    let source = r#"
        module 0xf00d::M {
            #[event]
            struct Event1 { }

            struct Event2 { }
        }
        "#;
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package(&account, path.path());
    assert_success!(result);

    // Compatible upgrade -- add event attribute.
    let source = r#"
        module 0xf00d::M {
            #[event]
            struct Event1 { }

            #[event]
            struct Event2 { }
        }
        "#;
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package(&account, path.path());
    assert_success!(result);

    // Incompatible upgrades -- remove existing event attribute
    let source = r#"
        module 0xf00d::M {
            struct Event1 { }

            #[event]
            struct Event2 { }
        }
        "#;
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package(&account, path.path());
    assert_vm_status!(result, StatusCode::EVENT_METADATA_VALIDATION_ERROR);
}
