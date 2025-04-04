// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::{new_local_swarm_with_aptos, SwarmBuilder},
    txn_emitter::generate_traffic,
};
use aptos_cached_packages::aptos_stdlib;
use aptos_config::config::GasEstimationConfig;
use aptos_crypto::ed25519::Ed25519Signature;
use aptos_forge::{LocalSwarm, NodeExt, Swarm, TransactionType};
use aptos_global_constants::{DEFAULT_BUCKETS, GAS_UNIT_PRICE};
use aptos_rest_client::{
    aptos_api_types::{MoveModuleId, TransactionData, ViewFunction, ViewRequest},
    Client,
};
use aptos_sdk::move_types::language_storage::StructTag;
use aptos_types::{
    account_address::AccountAddress,
    account_config::{AccountResource, CORE_CODE_ADDRESS},
    on_chain_config::{ExecutionConfigV2, OnChainExecutionConfig, TransactionShufflerType},
    transaction::{authenticator::AuthenticationKey, SignedTransaction, Transaction},
};
use move_core_types::{
    ident_str,
    language_storage::{ModuleId, TypeTag},
};
use std::{convert::TryFrom, str::FromStr, sync::Arc, time::Duration};

#[tokio::test]
async fn test_get_index() {
    let swarm = new_local_swarm_with_aptos(1).await;
    let info = swarm.aptos_public_info();

    let resp = reqwest::get(info.url().to_owned()).await.unwrap();
    assert_eq!(reqwest::StatusCode::OK, resp.status());
}

#[tokio::test]
async fn test_basic_client() {
    let swarm = new_local_swarm_with_aptos(1).await;
    let mut info = swarm.aptos_public_info();

    info.client().get_ledger_information().await.unwrap();

    // NOTE(Gas): For some reason, there needs to be a lot of funds in the account in order for the
    //            test to pass.
    //            Is this caused by us increasing the default max gas amount in
    //            testsuite/forge/src/interface/aptos.rs?
    let account1 = info
        .create_and_fund_user_account(10_000_000_000)
        .await
        .unwrap();
    let account2 = info
        .create_and_fund_user_account(10_000_000_000)
        .await
        .unwrap();

    let tx = account1.sign_with_transaction_builder(
        info.transaction_factory()
            .payload(aptos_stdlib::aptos_coin_transfer(account2.address(), 1)),
    );
    let pending_txn = info.client().submit(&tx).await.unwrap().into_inner();

    info.client()
        .wait_for_transaction(&pending_txn)
        .await
        .unwrap();

    info.client()
        .get_transaction_by_hash(pending_txn.hash.into())
        .await
        .unwrap();

    info.client()
        .get_account_resources(CORE_CODE_ADDRESS)
        .await
        .unwrap();

    info.client().get_transactions(None, None).await.unwrap();
}

fn next_bucket(gas_unit_price: u64) -> u64 {
    *DEFAULT_BUCKETS
        .iter()
        .find(|bucket| **bucket > gas_unit_price)
        .unwrap()
}

async fn block_height(client: &Client) -> u64 {
    client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .block_height
}

async fn test_gas_estimation_inner(swarm: &mut LocalSwarm) {
    let client = swarm.validators().next().unwrap().rest_client();
    let estimation = match client.estimate_gas_price().await {
        Ok(res) => res.into_inner(),
        Err(e) => panic!("Client error: {:?}", e),
    };
    println!("{:?}", estimation);
    // Note: in testing GAS_UNIT_PRICE = 0
    assert_eq!(Some(GAS_UNIT_PRICE), estimation.deprioritized_gas_estimate);
    assert_eq!(GAS_UNIT_PRICE, estimation.gas_estimate);
    assert_eq!(
        Some(next_bucket(GAS_UNIT_PRICE)),
        estimation.prioritized_gas_estimate
    );

    let txn_gas_price = 100;
    let all_validators: Vec<_> = swarm.validators().map(|v| v.peer_id()).collect();
    let txn_stat = generate_traffic(
        swarm,
        &all_validators,
        Duration::from_secs(20),
        txn_gas_price,
        vec![vec![(
            TransactionType::CoinTransfer {
                invalid_transaction_ratio: 0,
                sender_use_account_pool: false,
                non_conflicting: false,
                use_fa_transfer: false,
            },
            100,
        )]],
    )
    .await
    .unwrap();
    println!("{:?}", txn_stat.rate());

    let estimation = match client.estimate_gas_price().await {
        Ok(res) => res.into_inner(),
        Err(e) => panic!("Client error: {:?}", e),
    };
    println!("{:?}", estimation);
    // Note: it's quite hard to get deprioritized_gas_estimate higher in smoke tests
    assert_eq!(next_bucket(txn_gas_price), estimation.gas_estimate);
    assert_eq!(
        Some(next_bucket(next_bucket(txn_gas_price))),
        estimation.prioritized_gas_estimate
    );

    // Wait for enough empty blocks to reset the prices
    let num_blocks_to_reset = GasEstimationConfig::default().aggressive_block_history as u64;
    let base_height = block_height(&client).await;
    loop {
        let num_blocks_passed = block_height(&client).await - base_height;
        if num_blocks_passed > num_blocks_to_reset {
            println!("{} blocks passed, done sleeping", num_blocks_passed);
            break;
        }
        println!("{} blocks passed, sleeping 10 secs...", num_blocks_passed);
        // Exercise cache
        client.estimate_gas_price().await.unwrap();
        std::thread::sleep(Duration::from_secs(10));
    }

    // Multiple times, to exercise cache
    for _i in 0..2 {
        let estimation = match client.estimate_gas_price().await {
            Ok(res) => res.into_inner(),
            Err(e) => panic!("Client error: {:?}", e),
        };
        println!("{:?}", estimation);
        // Note: in testing GAS_UNIT_PRICE = 0
        assert_eq!(Some(GAS_UNIT_PRICE), estimation.deprioritized_gas_estimate);
        assert_eq!(GAS_UNIT_PRICE, estimation.gas_estimate);
        assert_eq!(
            Some(next_bucket(GAS_UNIT_PRICE)),
            estimation.prioritized_gas_estimate
        );
    }
}

#[tokio::test]
async fn test_gas_estimation_txns_limit() {
    let mut swarm = SwarmBuilder::new_local(1)
        .with_init_config(Arc::new(|_, conf, _| {
            let max_block_txns = 3;
            conf.api.gas_estimation.enabled = true;
            // Use a small full block threshold to make gas estimates update sooner.
            conf.api.gas_estimation.full_block_txns = max_block_txns as usize;
            // Wait for full blocks with small block size to advance consensus at a fast rate.
            conf.consensus.quorum_store_poll_time_ms = 200;
            conf.consensus.wait_for_full_blocks_above_pending_blocks = 0;
            conf.consensus.max_sending_block_txns = max_block_txns;
            conf.consensus.quorum_store.sender_max_batch_txns = conf
                .consensus
                .quorum_store
                .sender_max_batch_txns
                .min(max_block_txns as usize);
            conf.consensus.quorum_store.receiver_max_batch_txns = conf
                .consensus
                .quorum_store
                .receiver_max_batch_txns
                .min(max_block_txns as usize);
        }))
        .build()
        .await;

    test_gas_estimation_inner(&mut swarm).await;
}

#[tokio::test]
#[ignore]
// This test is ignored because after enabling gas limit, the txn emitter fails.
// TODO (bchocho): Fix this test.
async fn test_gas_estimation_gas_used_limit() {
    let mut swarm = SwarmBuilder::new_local(1)
        .with_init_genesis_config(Arc::new(|conf| {
            conf.execution_config = OnChainExecutionConfig::V2(ExecutionConfigV2 {
                transaction_shuffler_type: TransactionShufflerType::NoShuffling,
                block_gas_limit: Some(1),
            });
        }))
        .with_init_config(Arc::new(|_, conf, _| {
            let max_block_txns = 3;
            conf.api.gas_estimation.enabled = true;
            // The full block threshold will never be hit
            conf.api.gas_estimation.full_block_txns = (max_block_txns * 2) as usize;
            // Wait for full blocks with small block size to advance consensus at a fast rate.
            conf.consensus.quorum_store_poll_time_ms = 200;
            conf.consensus.wait_for_full_blocks_above_pending_blocks = 0;
            conf.consensus.max_sending_block_txns = max_block_txns;
            conf.consensus.quorum_store.sender_max_batch_txns = conf
                .consensus
                .quorum_store
                .sender_max_batch_txns
                .min(max_block_txns as usize);
            conf.consensus.quorum_store.receiver_max_batch_txns = conf
                .consensus
                .quorum_store
                .receiver_max_batch_txns
                .min(max_block_txns as usize);
        }))
        .build()
        .await;

    test_gas_estimation_inner(&mut swarm).await;
}

#[tokio::test]
async fn test_bcs() {
    let swarm = new_local_swarm_with_aptos(1).await;
    let mut info = swarm.aptos_public_info();

    // Create accounts
    let mut local_account = info
        .create_and_fund_user_account(100_000_000_000)
        .await
        .unwrap();
    let account = local_account.address();
    let public_key = local_account.public_key();
    let other_local_account = info
        .create_and_fund_user_account(100_000_000_000)
        .await
        .unwrap();

    let client = info.client();
    // Check get account
    let account_resource = client.get_account_bcs(account).await.unwrap().into_inner();
    let expected_auth_key = AuthenticationKey::ed25519(public_key);
    let onchain_auth_key =
        AuthenticationKey::try_from(account_resource.authentication_key()).unwrap();
    assert_eq!(expected_auth_key, onchain_auth_key);
    assert_eq!(0, account_resource.sequence_number());

    // Check get resources
    let resources = client
        .get_account_resources_bcs(account)
        .await
        .unwrap()
        .into_inner();
    let bytes = resources
        .get(&StructTag::from_str("0x1::account::Account").unwrap())
        .unwrap();
    let account_resource: AccountResource = bcs::from_bytes(bytes).unwrap();
    assert_eq!(0, account_resource.sequence_number());

    let single_account_resource: AccountResource = client
        .get_account_resource_bcs(account, "0x1::account::Account")
        .await
        .unwrap()
        .into_inner();
    assert_eq!(account_resource, single_account_resource);

    // Check Modules align
    let modules = client
        .get_account_modules(AccountAddress::ONE)
        .await
        .unwrap()
        .into_inner();
    let bcs_modules = client
        .get_account_modules_bcs(AccountAddress::ONE)
        .await
        .unwrap()
        .into_inner();

    assert_eq!(modules.len(), bcs_modules.len());
    let module_bytecode = modules.first().unwrap().clone().try_parse_abi().unwrap();
    let module_abi = module_bytecode.abi.as_ref().unwrap();
    let module_id = MoveModuleId {
        address: module_abi.address,
        name: module_abi.name.clone(),
    };
    assert_eq!(
        &module_bytecode.bytecode.0,
        bcs_modules.get(&module_id).unwrap()
    );

    let json_module = client
        .get_account_module(AccountAddress::ONE, module_id.name.as_str())
        .await
        .unwrap()
        .into_inner();
    let bcs_module = client
        .get_account_module_bcs(AccountAddress::ONE, module_id.name.as_str())
        .await
        .unwrap()
        .into_inner();
    assert_eq!(json_module.bytecode.0, bcs_module);
    assert_eq!(module_bytecode.bytecode.0, bcs_module);

    // Transfer money to make a transaction
    let pending_transaction = info
        .transfer(&mut local_account, &other_local_account, 500)
        .await
        .unwrap();
    let expected_txn_hash = pending_transaction.hash.into();
    let expected_txn = client
        .wait_for_transaction_bcs(&pending_transaction)
        .await
        .unwrap()
        .into_inner();
    let expected_txn_version = expected_txn.version;

    // Check transactions on an account
    let transactions = client
        .get_account_ordered_transactions(account, Some(0), Some(2))
        .await
        .unwrap()
        .into_inner();
    let transactions_bcs = client
        .get_account_transactions_bcs(account, Some(0), Some(2))
        .await
        .unwrap()
        .into_inner();

    // Should only have the transfer up there
    assert!(transactions_bcs.contains(&expected_txn));
    assert_eq!(1, transactions_bcs.len());
    assert_eq!(transactions.len(), transactions_bcs.len());

    for (i, expected_transaction) in transactions.iter().enumerate() {
        let bcs_txn = transactions_bcs.get(i).unwrap();
        assert_eq!(bcs_txn.version, expected_transaction.version().unwrap());
        let expected_hash =
            aptos_crypto::HashValue::from(expected_transaction.transaction_info().unwrap().hash);

        let bcs_hash = if let Transaction::UserTransaction(ref txn) = bcs_txn.transaction {
            txn.committed_hash()
        } else {
            panic!("BCS transaction is not a user transaction! {:?}", bcs_txn);
        };
        assert_eq!(expected_hash, bcs_hash);
    }

    // Check that the transaction is able to be looked up by hash and version
    let expected_txn_data = TransactionData::OnChain(expected_txn);

    assert_eq!(
        expected_txn_data,
        client
            .get_transaction_by_hash_bcs(expected_txn_hash)
            .await
            .unwrap()
            .into_inner()
    );
    assert_eq!(
        expected_txn_data,
        client
            .get_transaction_by_version_bcs(expected_txn_version)
            .await
            .unwrap()
            .into_inner()
    );

    // Check that the first 5 transactions match
    let json_txns = client
        .get_transactions(Some(0), Some(5))
        .await
        .unwrap()
        .into_inner();
    let bcs_txns = client
        .get_transactions_bcs(Some(0), Some(5))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(5, json_txns.len());
    assert_eq!(json_txns.len(), bcs_txns.len());

    // Ensure same hashes and versions for each transaction
    for (i, json_txn) in json_txns.iter().enumerate() {
        let bcs_txn = bcs_txns.get(i).unwrap();

        assert_eq!(json_txn.version().unwrap(), bcs_txn.version);
        assert_eq!(
            aptos_crypto::HashValue::from(json_txn.transaction_info().unwrap().hash),
            bcs_txn.info.transaction_hash()
        );
    }

    // Test simulation of a transaction should be the same in BCS & JSON
    let transfer_txn = info
        .transaction_factory()
        .transfer(other_local_account.address(), 500)
        .sender(local_account.address())
        .sequence_number(local_account.sequence_number())
        .build();
    let signed_txn = SignedTransaction::new(
        transfer_txn,
        local_account.public_key().clone(),
        Ed25519Signature::dummy_signature(),
    );

    let json_txns = client.simulate(&signed_txn).await.unwrap().into_inner();
    let json_txn = json_txns.first().unwrap();

    let bcs_txn = client.simulate_bcs(&signed_txn).await.unwrap().into_inner();
    assert_eq!(
        aptos_crypto::HashValue::from(json_txn.info.hash),
        bcs_txn.info.transaction_hash()
    );

    // Actually submit the transaction, and ensure it submits and succeeds
    // TODO: check failure case?
    let transfer_txn = local_account.sign_with_transaction_builder(
        info.transaction_factory()
            .transfer(other_local_account.address(), 500),
    );

    let txn = client
        .submit_and_wait_bcs(&transfer_txn)
        .await
        .unwrap()
        .into_inner();
    let txn_version = txn.version;
    assert_eq!(
        txn.transaction.try_as_signed_user_txn().unwrap(),
        &transfer_txn
    );

    // Check blocks
    let json_block = client
        .get_block_by_version(txn_version, true)
        .await
        .unwrap()
        .into_inner();
    let bcs_block = client
        .get_block_by_version_bcs(txn_version, true)
        .await
        .unwrap()
        .into_inner();

    assert_eq!(json_block.block_height.0, bcs_block.block_height);
    assert_eq!(
        aptos_crypto::HashValue::from(json_block.block_hash),
        bcs_block.block_hash
    );

    let json_txns = json_block.transactions.unwrap();
    let first_json_txn = json_txns.first().unwrap();
    let bcs_txns = bcs_block.transactions.unwrap();
    let first_bcs_txn = bcs_txns.first().unwrap();
    assert_eq!(first_json_txn.version().unwrap(), first_bcs_txn.version);
    assert_eq!(
        aptos_crypto::HashValue::from(first_json_txn.transaction_info().unwrap().hash),
        first_bcs_txn.info.transaction_hash()
    );

    let json_block_by_height = client
        .get_block_by_height(bcs_block.block_height, true)
        .await
        .unwrap()
        .into_inner();
    let bcs_block_by_height = client
        .get_block_by_height_bcs(bcs_block.block_height, true)
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        json_block_by_height.block_height.0,
        bcs_block_by_height.block_height
    );
    assert_eq!(bcs_block.block_height, bcs_block_by_height.block_height);
    assert_eq!(bcs_block.block_hash, bcs_block_by_height.block_hash);
    assert_eq!(
        aptos_crypto::HashValue::from(json_block_by_height.block_hash),
        bcs_block_by_height.block_hash
    );

    let json_events = client
        .get_account_events(
            AccountAddress::ONE,
            "0x1::block::BlockResource",
            "new_block_events",
            Some(0),
            Some(1),
        )
        .await
        .unwrap()
        .into_inner();
    let bcs_events = client
        .get_account_events_bcs(
            AccountAddress::ONE,
            "0x1::block::BlockResource",
            "new_block_events",
            Some(0),
            Some(1),
        )
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        json_events.first().unwrap().version.0,
        bcs_events.first().unwrap().transaction_version
    );

    // Test that more than 25 transactions can be retrieved
    let json_txns = client
        .get_transactions(Some(0), Some(30))
        .await
        .unwrap()
        .into_inner();
    let bcs_txns = client
        .get_transactions_bcs(Some(0), Some(30))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(json_txns.len(), 30);
    assert_eq!(json_txns.len(), bcs_txns.len());
}

#[tokio::test]
async fn test_view_function() {
    let swarm = new_local_swarm_with_aptos(1).await;
    let info = swarm.aptos_public_info();
    let client: &Client = info.client();

    let address = AccountAddress::ONE;

    // Non-BCS
    let view_request = ViewRequest {
        function: "0x1::coin::is_account_registered".parse().unwrap(),
        type_arguments: vec!["0x1::aptos_coin::AptosCoin".parse().unwrap()],
        arguments: vec![serde_json::Value::String(address.to_hex_literal())],
    };

    // Balance should be 0 and there should only be one return value
    let json_ret_values = client.view(&view_request, None).await.unwrap().into_inner();
    assert_eq!(json_ret_values.len(), 1);
    assert!(!json_ret_values[0].as_bool().unwrap());

    // BCS
    let bcs_view_request = ViewFunction {
        module: ModuleId::new(address, ident_str!("coin").into()),
        function: ident_str!("is_account_registered").into(),
        ty_args: vec![TypeTag::Struct(Box::new(
            StructTag::from_str("0x1::aptos_coin::AptosCoin").unwrap(),
        ))],
        args: vec![bcs::to_bytes(&address).unwrap()],
    };

    // Balance should be 0 and there should only be one return value
    let bcs_ret_values: Vec<bool> = client
        .view_bcs(&bcs_view_request, None)
        .await
        .unwrap()
        .into_inner();
    assert_eq!(bcs_ret_values.len(), 1);
    assert!(!bcs_ret_values[0]);
}
