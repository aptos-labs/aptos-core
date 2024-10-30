// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_package_builder::PackageBuilder;
use aptos_types::on_chain_config::FeatureFlag;
use move_core_types::{language_storage::TypeTag, vm_status::StatusCode};
use rstest::rstest;
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
fn test_module_event_enabled(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    h.enable_features(vec![FeatureFlag::MODULE_EVENT], vec![]);
    let account = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    let mut build_options = aptos_framework::BuildOptions::default();
    build_options
        .named_addresses
        .insert("event".to_string(), *account.address());

    let result = h.publish_package_with_options(
        &account,
        &common::test_dir_path("../../../move-examples/event"),
        build_options.clone(),
    );
    assert_success!(result);
    h.run_entry_function(
        &account,
        str::parse(format!("{}::event::emit", account.address()).as_str()).unwrap(),
        vec![],
        vec![bcs::to_bytes(&10u64).unwrap()],
    );
    let events = h.get_events();
    assert_eq!(events.len(), 13);
    let my_event_tag =
        TypeTag::from_str(format!("{}::event::MyEvent", account.address()).as_str()).unwrap();
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
fn verify_module_event_upgrades(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    h.enable_features(vec![FeatureFlag::MODULE_EVENT], vec![]);
    let account = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    // Initial code
    let source = format!(
        r#"
        module {}::M {{
            #[event]
            struct Event1 {{ }}

            struct Event2 {{ }}
        }}
        "#,
        account.address()
    );
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source.as_str());
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package(&account, path.path());
    assert_success!(result);

    // Compatible upgrade -- add event attribute.
    let source = format!(
        r#"
        module {}::M {{
            #[event]
            struct Event1 {{ }}

            #[event]
            struct Event2 {{ }}
        }}
        "#,
        account.address()
    );
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source.as_str());
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package(&account, path.path());
    assert_success!(result);

    // Incompatible upgrades -- remove existing event attribute
    let source = format!(
        r#"
        module {}::M {{
            struct Event1 {{ }}

            #[event]
            struct Event2 {{ }}
        }}
        "#,
        account.address()
    );
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source.as_str());
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package(&account, path.path());
    assert_vm_status!(result, StatusCode::EVENT_METADATA_VALIDATION_ERROR);
}
