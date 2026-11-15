// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! REST rendering of values beyond the value depth limit.

use super::new_test_context;
use aptos_api_test_context::{current_function_name, TestContext};
use aptos_api_types::Address;
use aptos_sdk::types::LocalAccount;
use aptos_types::{
    account_address::AccountAddress,
    transaction::{EntryFunction, SignedTransaction, TransactionPayload},
};
use move_core_types::{
    function::{ClosureMask, MoveClosure},
    ident_str,
    identifier::Identifier,
    language_storage::ModuleId,
    value::{MoveTypeLayout, MoveValue},
};
use serde_json::{json, Value};
use std::{path::PathBuf, str::FromStr};

fn closure(captured: Vec<(MoveTypeLayout, MoveValue)>) -> MoveValue {
    MoveValue::closure(MoveClosure {
        module_id: ModuleId::new(AccountAddress::ONE, ident_str!("m").to_owned()),
        fun_id: ident_str!("f").to_owned(),
        ty_args: vec![],
        mask: ClosureMask::new((1u64 << captured.len()) - 1),
        captured,
    })
}

/// BCS bytes of `levels` nested closures, each capturing the next one wrapped in `vectors`
/// vectors. Built without recursion.
fn deep_closure_bytes(levels: usize, vectors: usize) -> Vec<u8> {
    let inner = closure(vec![]);
    let inner_bytes = inner.simple_serialize().expect("closure must serialize");
    let layout = (0..vectors).fold(MoveTypeLayout::Function, |l, _| {
        MoveTypeLayout::Vector(Box::new(l))
    });
    let value = (0..vectors).fold(inner, |v, _| MoveValue::Vector(vec![v]));
    let outer_bytes = closure(vec![(layout, value)])
        .simple_serialize()
        .expect("closure must serialize");
    let mut bytes = outer_bytes[..outer_bytes.len() - inner_bytes.len()].repeat(levels);
    bytes.extend(inner_bytes);
    bytes
}

async fn publish_nested_values(context: &mut TestContext, account: &mut LocalAccount) {
    let path =
        PathBuf::from(std::env!("CARGO_MANIFEST_DIR")).join("src/tests/move/pack_nested_values");
    let payload = TestContext::build_package_with_latest_language(path, vec![(
        "account".to_string(),
        account.address(),
    )]);
    context.publish_package(account, payload).await;
}

fn sign_call(
    context: &mut TestContext,
    account: &mut LocalAccount,
    function: &str,
    args: Vec<Vec<u8>>,
) -> SignedTransaction {
    let function = EntryFunction::new(
        ModuleId::new(account.address(), ident_str!("nested_values").to_owned()),
        Identifier::new(function).expect("valid identifier"),
        vec![],
        args,
    );
    account.sign_with_transaction_builder(
        context
            .transaction_factory()
            .payload(TransactionPayload::EntryFunction(function))
            .expiration_timestamp_secs(context.get_expiration_time())
            .upgrade_payload_with_rng(
                &mut context.rng,
                context.use_txn_payload_v2_format,
                context.use_orderless_transactions,
            ),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_txn_argument_beyond_depth_limit() {
    let mut context = new_test_context(current_function_name!());
    let mut account = context.create_account().await;
    publish_nested_values(&mut context, &mut account).await;

    // The call fails, and REST renders the argument as hex since it exceeds the depth limit.
    let arg = deep_closure_bytes(100, 100);
    let txn = sign_call(&mut context, &mut account, "take", vec![arg.clone()]);
    context.commit_block(std::slice::from_ref(&txn)).await;

    let resp = context
        .get(&format!(
            "/transactions/by_hash/{}",
            txn.committed_hash().to_hex_literal()
        ))
        .await;
    assert_eq!(resp["success"], json!(false));
    assert_eq!(
        resp["payload"]["arguments"],
        json!([format!("0x{}", hex::encode(&arg))])
    );
}

/// Stores bytes in a `Table<u64, vector<u8>>` that exceed the depth limit when read as a
/// `Wrapper`. Returns the table handle.
async fn store_nested_table_item(context: &mut TestContext, account: &mut LocalAccount) -> Address {
    publish_nested_values(context, account).await;
    // Read as a `Wrapper`, the length prefix of the stored `vector<u8>` takes the place of the
    // closure's own length prefix.
    let bytes = deep_closure_bytes(100, 100)[1..].to_vec();
    let txn = sign_call(context, account, "store", vec![
        bcs::to_bytes(&bytes).expect("bytes must serialize")
    ]);
    context.commit_block(std::slice::from_ref(&txn)).await;

    let holder = context
        .gen_resource(
            &account.address(),
            &format!(
                "{}::nested_values::Holder",
                account.address().to_hex_literal()
            ),
        )
        .await
        .expect("holder must exist");
    let handle = holder["data"]["items"]["handle"]
        .as_str()
        .expect("table handle must be a string");
    Address::from_str(handle).expect("table handle must be an address")
}

fn wrapper_type(account: &LocalAccount) -> String {
    format!(
        "{}::nested_values::Wrapper",
        account.address().to_hex_literal()
    )
}

fn table_item_request(value_type: &str) -> Value {
    json!({
        "key_type": "u64",
        "value_type": value_type,
        "key": "0",
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_table_item_beyond_depth_limit() {
    let mut context = new_test_context(current_function_name!());
    let mut account = context.create_account().await;
    let handle = store_nested_table_item(&mut context, &mut account).await;
    let path = format!("/tables/{}/item", handle);

    // Decoding under `Wrapper` exceeds the depth limit.
    let resp = context
        .expect_status_code(400)
        .post(&path, table_item_request(&wrapper_type(&account)))
        .await;
    assert_eq!(resp["error_code"], json!("invalid_input"), "{}", resp);
    let message = resp["message"].as_str().expect("message must be a string");
    assert!(message.contains("maximum nesting depth"), "{}", resp);

    let resp = context.post(&path, table_item_request("vector<u8>")).await;
    let bytes = resp.as_str().expect("value must be a hex string");
    assert!(bytes.starts_with("0x"), "{}", resp);
}
