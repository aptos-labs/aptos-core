// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::ed25519::Ed25519Signature;
use aptos_gas::{AptosGasParameters, FromOnChainGasSchedule};
use aptos_rest_client::aptos_api_types::{MoveModuleId, TransactionData};
use aptos_sdk::move_types::language_storage::StructTag;
use aptos_types::account_address::AccountAddress;
use aptos_types::account_config::{AccountResource, CORE_CODE_ADDRESS};
use aptos_types::on_chain_config::GasScheduleV2;
use aptos_types::transaction::authenticator::AuthenticationKey;
use aptos_types::transaction::{SignedTransaction, Transaction};
use cached_packages::aptos_stdlib;
use forge::Swarm;
use std::convert::TryFrom;
use std::str::FromStr;

use crate::smoke_test_environment::new_local_swarm_with_aptos;

#[tokio::test]
async fn test_get_index() {
    let mut swarm = new_local_swarm_with_aptos(1).await;
    let info = swarm.aptos_public_info();

    let resp = reqwest::get(info.url().to_owned()).await.unwrap();
    assert_eq!(reqwest::StatusCode::OK, resp.status());
}

#[tokio::test]
async fn test_basic_client() {
    let mut swarm = new_local_swarm_with_aptos(1).await;
    let mut info = swarm.aptos_public_info();

    info.client().get_ledger_information().await.unwrap();

    // NOTE(Gas): For some reason, there needs to be a lot of funds in the account in order for the
    //            test to pass.
    //            Is this caused by us increasing the default max gas amount in
    //            testsuite/forge/src/interface/aptos.rs?
    let mut account1 = info.create_and_fund_user_account(100_000).await.unwrap();
    let account2 = info.create_and_fund_user_account(100_000).await.unwrap();

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

// Test needs to be fixed to estimate over a longer period of time / probably needs an adjustable window
// to test
#[ignore]
#[tokio::test]
async fn test_gas_estimation() {
    let mut swarm = new_local_swarm_with_aptos(1).await;
    let mut public_info = swarm.aptos_public_info();

    let gas_schedule: GasScheduleV2 = public_info
        .client()
        .get_account_resource_bcs(CORE_CODE_ADDRESS, "0x1::gas_schedule::GasScheduleV2")
        .await
        .unwrap()
        .into_inner();
    let gas_params =
        AptosGasParameters::from_on_chain_gas_schedule(&gas_schedule.to_btree_map()).unwrap();

    // No transactions should always return 1 as the estimated gas
    assert_eq!(
        u64::from(gas_params.txn.min_price_per_gas_unit),
        public_info
            .client()
            .estimate_gas_price()
            .await
            .unwrap()
            .into_inner()
            .gas_estimate,
        "No transactions should equate to lowest gas price"
    );
    let account1 = public_info
        .create_and_fund_user_account(1000000)
        .await
        .expect("Should create account");
    let account2 = public_info
        .create_and_fund_user_account(1000000)
        .await
        .expect("Should create account");

    // When we have higher cost transactions, it should shift to a non-1 value (if it's higher than 1)
    let transfer1 = public_info
        .transaction_factory()
        .transfer(account2.address(), 10)
        .gas_unit_price(5)
        .sequence_number(0)
        .sender(account1.address())
        .build();
    let transfer2 = public_info
        .transaction_factory()
        .transfer(account2.address(), 10)
        .gas_unit_price(5)
        .sequence_number(1)
        .sender(account1.address())
        .build();
    let transfer3 = public_info
        .transaction_factory()
        .transfer(account2.address(), 10)
        .gas_unit_price(5)
        .sequence_number(2)
        .sender(account1.address())
        .build();
    let transfer4 = public_info
        .transaction_factory()
        .transfer(account2.address(), 10)
        .gas_unit_price(5)
        .sequence_number(3)
        .sender(account1.address())
        .build();
    let transfer5 = public_info
        .transaction_factory()
        .transfer(account2.address(), 10)
        .gas_unit_price(5)
        .sequence_number(4)
        .sender(account1.address())
        .build();
    let transfer1 = account1.sign_transaction(transfer1);
    let transfer2 = account1.sign_transaction(transfer2);
    let transfer3 = account1.sign_transaction(transfer3);
    let transfer4 = account1.sign_transaction(transfer4);
    let transfer5 = account1.sign_transaction(transfer5);
    public_info
        .client()
        .submit(&transfer1)
        .await
        .expect("Should successfully submit");
    public_info
        .client()
        .submit(&transfer2)
        .await
        .expect("Should successfully submit");
    public_info
        .client()
        .submit(&transfer3)
        .await
        .expect("Should successfully submit");
    public_info
        .client()
        .submit(&transfer4)
        .await
        .expect("Should successfully submit");
    public_info
        .client()
        .submit_and_wait_bcs(&transfer5)
        .await
        .expect("Should successfully submit and wait")
        .into_inner();

    assert_eq!(
        5,
        public_info
            .client()
            .estimate_gas_price()
            .await
            .unwrap()
            .into_inner()
            .gas_estimate,
        "Gas estimate should move based on median"
    );
}

#[tokio::test]
async fn test_bcs() {
    let mut swarm = new_local_swarm_with_aptos(1).await;
    let mut info = swarm.aptos_public_info();

    // Create accounts
    let mut local_account = info.create_and_fund_user_account(10000000).await.unwrap();
    let account = local_account.address();
    let public_key = local_account.public_key();
    let other_local_account = info.create_and_fund_user_account(10000000).await.unwrap();

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
        .wait_for_transaction_by_hash_bcs(
            expected_txn_hash,
            pending_transaction.request.expiration_timestamp_secs.0,
        )
        .await
        .unwrap()
        .into_inner();
    let expected_txn_version = expected_txn.version;

    // Check transactions on an account
    let transactions = client
        .get_account_transactions(account, Some(0), Some(2))
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
            txn.clone().committed_hash()
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
    assert_eq!(txn.transaction.as_signed_user_txn().unwrap(), &transfer_txn);

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
}
