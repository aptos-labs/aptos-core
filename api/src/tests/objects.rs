// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use super::new_test_context_with_orderless_flags;
use aptos_api_test_context::{current_function_name, TestContext};
use aptos_types::account_address::{self, AccountAddress};
use rstest::rstest;
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::PathBuf};

// This test verifies that READ APIs can parse objects and events from objects
// 1. Create account
// 2. Create an object
// 3. Read object
// 4. Transfer to cause transfer event
// 4. Read emitted event
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_gen_object(use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;

    // Prepare account
    let mut user = context.create_account().await;
    let user_addr = user.address();

    // Publish packages
    let named_addresses = vec![("hero".to_string(), user_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("../aptos-move/move-examples/token_objects/hero");
        TestContext::build_package(path, named_addresses)
    });
    context.publish_package(&mut user, txn).await;

    // Read default data
    let collection_addr = account_address::create_collection_address(user_addr, "Hero Quest!");
    let token_addr = account_address::create_token_address(user_addr, "Hero Quest!", "Wukong");
    let object_resource = "0x1::object::ObjectCore";
    let token_resource = "0x4::token::Token";
    let hero_resource = format!("0x{}::hero::Hero", user_addr.to_hex());

    let collection0 = context.gen_all_resources(&collection_addr).await;

    context
        .api_execute_entry_function(
            &mut user,
            &format!("0x{}::hero::mint_hero", user_addr.to_hex()),
            json!([]),
            json!(["The best hero ever!", "Male", "Wukong", "Monkey God", ""]),
        )
        .await;
    let collection1 = context.gen_all_resources(&collection_addr).await;
    let collection0_obj = to_object(collection0);
    let collection1_obj = to_object(collection1);
    assert_eq!(
        collection0_obj["0x1::object::ObjectCore"],
        collection1_obj["0x1::object::ObjectCore"]
    );
    assert_eq!(
        collection0_obj["0x4::collection::Collection"],
        collection1_obj["0x4::collection::Collection"]
    );

    let hero = context.gen_all_resources(&token_addr).await;
    let hero_map = to_object(hero);
    assert!(hero_map.contains_key(object_resource));
    assert!(hero_map.contains_key(token_resource));
    assert!(hero_map.contains_key(&hero_resource));
    let owner: AccountAddress = hero_map[object_resource]["owner"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(owner, user_addr);

    context
        .api_execute_entry_function(
            &mut user,
            "0x1::object::transfer_call",
            json!([]),
            json!([token_addr, token_addr]),
        )
        .await;

    let hero = context.gen_all_resources(&token_addr).await;
    let hero_map = to_object(hero);

    let owner: AccountAddress = hero_map[object_resource]["owner"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(owner, token_addr);
}

fn to_object(value: Value) -> BTreeMap<String, Value> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            (
                entry["type"].as_str().unwrap().into(),
                entry["data"].clone(),
            )
        })
        .collect()
}
