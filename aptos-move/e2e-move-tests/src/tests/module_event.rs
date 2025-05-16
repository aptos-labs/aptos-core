// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    account_address::AccountAddress, on_chain_config::FeatureFlag, transaction::ExecutionStatus,
};
use claims::assert_ok;
use move_core_types::{language_storage::TypeTag, vm_status::StatusCode};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

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
        vec![bcs::to_bytes(&10u64).unwrap()],
    );
    let events = h.get_events();
    assert_eq!(events.len(), 13);
    let my_event_tag = TypeTag::from_str("0xcafe::event::MyEvent").unwrap();
    let mut count = 0;
    for event in events.iter() {
        if event.type_tag() == &my_event_tag {
            let module_event = event.v2().unwrap();
            assert_eq!(
                bcs::from_bytes::<MyEvent>(module_event.event_data()).unwrap(),
                MyEvent {
                    seq: count as u64,
                    field: Field { field: false },
                    bytes: vec![],
                }
            );
            count += 1;
        }
    }
    assert_eq!(count, 10);
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

#[test]
fn test_event_emission_not_allowed_in_scripts() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    let mut build_options = BuildOptions::move_2().set_latest_language();
    build_options
        .experiments
        .push("skip-bailout-on-extended-checks".to_string());

    let mut builder = PackageBuilder::new("P1");
    let source = r#"
    module 0x123::test {
        #[event]
        struct Event has copy, drop, store;

        public fun new_event(): Event {
            Event
        }

        public fun with_callback(callback: |Event|) {
            let event = new_event();
            callback(event);
        }

        public fun emit_from_callback(callback: ||Event) {
            let event = callback();
            0x1::event::emit(event);
        }
    }
    "#;
    builder.add_source("test.move", source);
    builder.add_local_dep(
        "AptosFramework",
        &common::framework_dir_path("aptos-framework").to_string_lossy(),
    );

    let p1_path = builder.write_to_temp().unwrap();
    assert_success!(h.publish_package_with_options(&acc, p1_path.path(), build_options.clone()));

    let mut builder = PackageBuilder::new("P2");
    let sources = [
        r#"
        script {
            fun main_0() {
                let event = 0x123::test::new_event();
                0x1::event::emit<0x123::test::Event>(event);
            }
        }
        "#,
        r#"
        script {
            fun main_1() {
                let event = 0x123::test::new_event();
                let f = || 0x1::event::emit<0x123::test::Event>(event);
                f();
            }
        }
        "#,
        r#"
        script {
            fun main_2() {
                let f = |e| 0x1::event::emit<0x123::test::Event>(e);
                0x123::test::with_callback(f);
            }
        }
        "#,
    ];
    for (idx, source) in sources.iter().enumerate() {
        builder.add_source(&format!("main_{idx}.move"), source);
    }
    builder.add_local_dep("P1", p1_path.path().to_str().unwrap());
    builder.add_local_dep(
        "AptosFramework",
        &common::framework_dir_path("aptos-framework").to_string_lossy(),
    );

    let p2_path = builder.write_to_temp().unwrap();
    let p2_package = BuiltPackage::build(p2_path.path().to_path_buf(), build_options.clone())
        .expect("Should be able to build a package");

    let scripts = p2_package.extract_script_code();
    assert_eq!(scripts.len(), 3);

    for script in scripts {
        let txn = h.create_script(&acc, script, vec![], vec![]);
        let execution_status = assert_ok!(h.run_raw(txn).status().as_kept_status());
        assert!(matches!(
            execution_status,
            ExecutionStatus::MiscellaneousError(Some(StatusCode::INVALID_OPERATION_IN_SCRIPT))
        ));
    }

    let mut builder = PackageBuilder::new("P3");
    let source = r#"
        script {
            fun main() {
                let f = || 0x123::test::new_event();
                0x123::test::emit_from_callback(f);
            }
        }
        "#;
    builder.add_source("main.move", source);
    builder.add_local_dep("P1", p1_path.path().to_str().unwrap());

    let p3_path = builder.write_to_temp().unwrap();
    let p3_package = BuiltPackage::build(p3_path.path().to_path_buf(), build_options)
        .expect("Should be able to build a package");

    let mut scripts = p3_package.extract_script_code();
    assert_eq!(scripts.len(), 1);

    let txn = h.create_script(&acc, scripts.pop().unwrap(), vec![], vec![]);
    assert_success!(h.run_raw(txn).status().clone());
}

#[test]
fn test_event_emission_in_modules() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    let mut build_options = BuildOptions::move_2().set_latest_language();
    build_options
        .experiments
        .push("skip-bailout-on-extended-checks".to_string());

    let mut builder = PackageBuilder::new("P1");
    let source = r#"
    module 0x123::event {
        #[event]
        struct Event has copy, drop, store;

        public fun new_event(): Event {
            Event
        }
    }
    "#;
    builder.add_source("event.move", source);
    let p1_path = builder.write_to_temp().unwrap();
    assert_success!(h.publish_package_with_options(&acc, p1_path.path(), build_options.clone()));

    let sources = [
        r#"
        module 0x123::test2 {
            public fun emit() {
                let event = 0x123::event::new_event();
                0x1::event::emit<0x123::event::Event>(event);
            }
        }
        "#,
        r#"
        module 0x123::test3 {
            public fun emit() {
                let event = 0x123::event::new_event();
                let f = || 0x1::event::emit<0x123::event::Event>(event);
                f();
            }
        }
        "#,
        r#"
        module 0x123::test4 {
            public fun emit() {
                let event = 0x123::event::new_event();
                let f = || {
                  if (true) {
                    0x1::event::emit<0x123::event::Event>(event);
                  };
                };
                f();
            }
        }
        "#,
    ];
    for (idx, source) in sources.into_iter().enumerate() {
        // To make sure files and packages named consistently.
        let idx = idx + 2;

        let mut builder = PackageBuilder::new(&format!("P{idx}"));
        builder.add_source(&format!("test{idx}.move"), source);
        builder.add_local_dep("P1", p1_path.path().to_str().unwrap());
        builder.add_local_dep(
            "AptosFramework",
            &common::framework_dir_path("aptos-framework").to_string_lossy(),
        );
        let path = builder.write_to_temp().unwrap();
        let status = h.publish_package_with_options(&acc, path.path(), build_options.clone());
        let execution_status = assert_ok!(status.as_kept_status());
        assert!(matches!(
            execution_status,
            ExecutionStatus::MiscellaneousError(Some(StatusCode::EVENT_METADATA_VALIDATION_ERROR))
        ));
    }
}
