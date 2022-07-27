// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    current_function_name,
    protos::extractor::transaction::{TransactionType, Txn_data},
    protos::extractor::transaction_payload::{Payload, PayloadType},
    runtime::SfStreamer,
    tests::new_test_context,
};
use aptos_sdk::types::account_config::aptos_root_address;
use move_deps::move_core_types::value::MoveValue;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use serde_json::Value;

#[tokio::test]
async fn test_genesis_works() {
    let test_context = new_test_context(current_function_name!());

    let addr: SocketAddr = format!("{}:{}", "127.0.0.1", 8083).parse().unwrap();
    let context = Arc::new(test_context.context);
    let mut streamer = SfStreamer::new(addr, context, 0, None);
    let converted = streamer.batch_convert_once(10).await;

    // position 0 should be genesis
    let txn = converted.first().unwrap().clone();
    assert_eq!(txn.version, 0);
    assert_eq!(txn.type_.unwrap(), TransactionType::GENESIS);
    assert_eq!(txn.block_height, 0);
    if let Txn_data::GenesisTxn(txn) = txn.txn_data.unwrap() {
        assert_eq!(
            txn.events[0].key.account_address,
            aptos_root_address().to_string()
        );
    }
}

#[tokio::test]
async fn test_block_transactions_work() {
    let mut test_context = new_test_context(current_function_name!());

    // create user transactions
    let account = test_context.gen_account();
    let txn = test_context.create_user_account(&account);
    test_context.commit_block(&vec![txn.clone()]).await;

    let addr: SocketAddr = format!("{}:{}", "127.0.0.1", 8083).parse().unwrap();
    let context = Arc::new(test_context.clone().context);
    let mut streamer = SfStreamer::new(addr, context, 0, None);

    // emulating real stream, getting first block
    let converted_0 = streamer.batch_convert_once(1).await;
    let txn = converted_0.first().unwrap().clone();
    assert_eq!(txn.version, 0);
    assert_eq!(txn.type_.unwrap(), TransactionType::GENESIS);

    // getting second block
    let converted_1 = streamer.batch_convert_once(3).await;
    // block metadata expected
    let txn = converted_1[0].clone();
    assert_eq!(txn.version, 1);
    assert_eq!(txn.type_.unwrap(), TransactionType::BLOCK_METADATA);
    if let Txn_data::BlockMetadataTxn(txn) = txn.txn_data.unwrap() {
        assert_eq!(txn.round, 1);
    }
    // user txn expected
    let txn = converted_1[1].clone();
    assert_eq!(txn.version, 2);
    assert_eq!(txn.type_.unwrap(), TransactionType::USER);
    if let Txn_data::UserTxn(txn) = txn.txn_data.unwrap() {
        assert_eq!(
            txn.request.payload.type_.unwrap(),
            PayloadType::SCRIPT_FUNCTION_PAYLOAD
        );
        if let Payload::ScriptFunctionPayload(payload) =
            txn.request.payload.clone().unwrap().payload.unwrap()
        {
            let address_str = MoveValue::Address(account.address()).to_string();
            let address_str = Value::String(address_str).to_string();
            assert_eq!(*payload.arguments.first().unwrap(), address_str);
        }
    }
    // state checkpoint expected
    let txn = converted_1[2].clone();
    assert_eq!(txn.version, 3);
    assert_eq!(txn.type_.unwrap(), TransactionType::STATE_CHECKPOINT);
}

#[tokio::test]
async fn test_block_height_works() {
    let mut test_context = new_test_context(current_function_name!());

    // Creating 2 blocks w/ user transactions and 1 empty block
    let mut root_account = test_context.root_account();
    let account = test_context.gen_account();
    let txn = test_context.create_user_account_by(&mut root_account, &account);
    test_context.commit_block(&vec![txn.clone()]).await;
    let account = test_context.gen_account();
    let txn = test_context.create_user_account_by(&mut root_account, &account);
    test_context.commit_block(&vec![txn.clone()]).await;
    test_context.commit_block(&vec![]).await;

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

    let addr: SocketAddr = format!("{}:{}", "127.0.0.1", 8083).parse().unwrap();
    let context = Arc::new(test_context.clone().context);
    let mut streamer = SfStreamer::new(addr, context, 0, None);

    let converted = streamer.batch_convert_once(100).await;
    // Making sure that version - block height mapping is correct and that version is in order
    for (i, txn) in converted.iter().enumerate() {
        assert_eq!(txn.version as usize, i);
        assert_eq!(
            txn.block_height as usize,
            *block_mapping.get(&i).unwrap() as usize
        );
    }
}
