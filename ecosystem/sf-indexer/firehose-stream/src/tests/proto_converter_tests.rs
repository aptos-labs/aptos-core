// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    runtime::FirehoseStreamer,
    tests::{new_test_context, TestContext},
};

use aptos_api_test_context::current_function_name;
use aptos_protos::extractor::v1::{
    transaction::{TransactionType, TxnData},
    transaction_payload::{Payload, Type as PayloadType},
    write_set_change::Change::WriteTableItem,
    Transaction as TransactionPB,
};

use aptos_sdk::types::{account_config::aptos_test_root_address, LocalAccount};
use move_deps::{
    move_core_types::{account_address::AccountAddress, value::MoveValue},
    move_package::BuildConfig,
};
use serde_json::{json, Value};
use std::{collections::HashMap, convert::TryInto, path::PathBuf, sync::Arc, time::Duration};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_genesis_works() {
    let test_context = new_test_context(current_function_name!());

    let context = Arc::new(test_context.context);
    let mut streamer = FirehoseStreamer::new(context, 0, None);
    let converted = streamer.convert_next_block().await;

    // position 0 should be genesis
    let txn = converted.first().unwrap().clone();
    assert_eq!(txn.version, 0);
    assert_eq!(txn.r#type(), TransactionType::Genesis);
    assert_eq!(txn.block_height, 0);
    if let TxnData::Genesis(txn) = txn.txn_data.unwrap() {
        assert_eq!(
            txn.events[0].key.clone().unwrap().account_address,
            aptos_test_root_address().to_string()
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_block_transactions_work() {
    let mut test_context = new_test_context(current_function_name!());

    // create user transactions
    let account = test_context.gen_account();
    let txn = test_context.create_user_account(&account);
    test_context.commit_block(&vec![txn.clone()]).await;

    let context = Arc::new(test_context.clone().context);
    let mut streamer = FirehoseStreamer::new(context, 0, None);

    // emulating real stream, getting first block
    let block_0 = streamer.convert_next_block().await;
    let txn = block_0.first().unwrap().clone();
    assert_eq!(txn.version, 0);
    assert_eq!(txn.r#type(), TransactionType::Genesis);

    // getting second block
    let block_1 = streamer.convert_next_block().await;
    // block metadata expected
    let txn = block_1[0].clone();
    assert_eq!(txn.version, 1);
    assert_eq!(txn.r#type(), TransactionType::BlockMetadata);
    if let TxnData::BlockMetadata(txn) = txn.txn_data.unwrap() {
        assert_eq!(txn.round, 1);
    }
    // user txn expected
    let txn = block_1[1].clone();
    assert_eq!(txn.version, 2);
    assert_eq!(txn.r#type(), TransactionType::User);
    if let TxnData::User(txn) = txn.txn_data.as_ref().unwrap() {
        assert_eq!(
            txn.request
                .as_ref()
                .unwrap()
                .payload
                .as_ref()
                .unwrap()
                .r#type(),
            PayloadType::EntryFunctionPayload
        );
        if let Payload::EntryFunctionPayload(payload) = txn
            .request
            .as_ref()
            .unwrap()
            .payload
            .as_ref()
            .unwrap()
            .payload
            .as_ref()
            .unwrap()
        {
            let address_str = MoveValue::Address(account.address()).to_string();
            let address_str = Value::String(address_str).to_string();
            assert_eq!(*payload.arguments.first().unwrap(), address_str);
        }
    }

    // TODO: Add golden back after code freeze (removing now to avoid merge conflicts)
    // test_context.check_golden_output(&converted_1);

    // state checkpoint expected
    let txn = block_1[2].clone();
    assert_eq!(txn.version, 3);
    assert_eq!(txn.r#type(), TransactionType::StateCheckpoint);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_block_height_and_ts_work() {
    let start_ts_usecs = 1000 * 1000000;
    let mut test_context = new_test_context(current_function_name!());
    test_context.set_fake_time_usecs(start_ts_usecs as u64);

    // Creating 2 blocks w/ user transactions and 1 empty block
    let mut root_account = test_context.root_account();
    let account = test_context.gen_account();
    let txn = test_context.create_user_account_by(&mut root_account, &account);
    test_context.commit_block(&vec![txn.clone()]).await;
    let account = test_context.gen_account();
    let txn = test_context.create_user_account_by(&mut root_account, &account);
    test_context.commit_block(&vec![txn.clone()]).await;
    test_context.commit_block(&[]).await;

    // key is version and value is block_height
    let block_mapping = HashMap::from([
        (0, 0),
        (1, 1),
        (2, 1),
        (3, 1),
        (4, 2),
        (5, 2),
        (6, 2),
        (7, 3),
        (8, 3),
    ]);

    let context = Arc::new(test_context.clone().context);

    let streamer = FirehoseStreamer::new(context, 0, None);
    let converted = fetch_all_stream(streamer).await;

    assert_eq!(converted.len(), 9);
    // Making sure that version - block height mapping is correct and that version is in order
    for (i, txn) in converted.iter().enumerate() {
        assert_eq!(txn.version as usize, i);
        assert_eq!(
            txn.block_height as usize,
            *block_mapping.get(&i).unwrap() as usize
        );
        if txn.block_height == 0 {
            // Genesis timestamp is 0
            assert_eq!(txn.timestamp.clone().unwrap().seconds as u64, 0);
        } else {
            // Seconds should be going up once every 2 blocks because we increment twice
            let expected_secs =
                Duration::from_micros(start_ts_usecs).as_secs() + txn.block_height / 2;
            assert_eq!(txn.timestamp.clone().unwrap().seconds as u64, expected_secs);
            // Converting to nanos
            let expected_nanos = Duration::from_micros(
                start_ts_usecs + txn.block_height * Duration::from_secs(1).as_micros() as u64 / 2,
            )
            .subsec_nanos();
            assert_eq!(txn.timestamp.clone().unwrap().nanos as u32, expected_nanos);
        }
    }
}

#[ignore] // TODO: disabled because of bundle publishing deactivated; reactivate
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_table_item_parsing_works() {
    let mut test_context = new_test_context(current_function_name!());
    let ctx = &mut test_context;
    let mut account = ctx.gen_account();
    let acc = &mut account;
    let txn = ctx.create_user_account(acc);
    ctx.commit_block(&vec![txn.clone()]).await;
    make_test_tables(ctx, acc).await;

    // This is a subset of k-v added from TableTestData move module
    let expected_items: HashMap<String, String> = HashMap::from([
        (json!(2).to_string(), json!(3).to_string()),
        (json!("abc").to_string(), json!("abc").to_string()),
        (json!("0x1").to_string(), json!("0x1").to_string()),
        (
            json!(["abc", "abc"]).to_string(),
            json!(["abc", "abc"]).to_string(),
        ),
    ]);

    let context = Arc::new(test_context.clone().context);
    let streamer = FirehoseStreamer::new(context, 0, None);
    let converted = fetch_all_stream(streamer).await;

    let mut table_kv: HashMap<String, String> = HashMap::new();
    for parsed_txn in &converted {
        if parsed_txn.r#type() != TransactionType::User {
            continue;
        }
        for write_set_change in parsed_txn.info.as_ref().unwrap().changes.clone() {
            if let WriteTableItem(item) = write_set_change.change.unwrap() {
                let data = item.data.unwrap();
                table_kv.insert(data.key, data.value);
            }
        }
    }

    for (expected_k, expected_v) in expected_items.into_iter() {
        println!(
            "Expected key: {}, expected value: {}, actual value maybe: {:?}",
            expected_k,
            expected_v,
            table_kv.get(&expected_k)
        );
        assert_eq!(table_kv.get(&expected_k).unwrap(), &expected_v);
    }

    // TODO: Add golden back after code freeze (removing now to avoid merge conflicts)
    // test_context.check_golden_output(&converted[1..]);
}

async fn make_test_tables(ctx: &mut TestContext, account: &mut LocalAccount) {
    let module = build_test_module(account.address()).await;

    ctx.api_publish_module(account, module.try_into().unwrap())
        .await;
    ctx.api_execute_entry_function(
        account,
        "TableTestData",
        "make_test_tables",
        json!([]),
        json!([]),
    )
    .await
}

async fn build_test_module(account: AccountAddress) -> Vec<u8> {
    let package_dir = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("api/move-test-package");
    let build_config = BuildConfig {
        generate_docs: false,
        install_dir: Some(package_dir.clone()),
        additional_named_addresses: [("TestAccount".to_string(), account)].into(),
        ..Default::default()
    };
    let package = build_config
        .compile_package(&package_dir, &mut Vec::new())
        .unwrap();

    let mut out = Vec::new();
    package
        .root_modules_map()
        .iter_modules()
        .first()
        .unwrap()
        .serialize(&mut out)
        .unwrap();
    out
}

async fn fetch_all_stream(mut streamer: FirehoseStreamer) -> Vec<TransactionPB> {
    // Overfetching should work
    let mut res = streamer.convert_next_block().await;
    for _ in 0..20 {
        res.append(&mut streamer.convert_next_block().await);
    }
    res
}
