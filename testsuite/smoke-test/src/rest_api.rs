// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_rest_client::aptos_api_types::TransactionData;
use aptos_sdk::move_types::language_storage::StructTag;
use aptos_transaction_builder::aptos_stdlib;
use aptos_types::account_address::AccountAddress;
use aptos_types::account_config::{AccountResource, CORE_CODE_ADDRESS};
use aptos_types::transaction::authenticator::AuthenticationKey;
use aptos_types::transaction::Transaction;
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

    // TODO(Gas): double check if this is correct
    let mut account1 = info.create_and_fund_user_account(10_000).await.unwrap();
    // TODO(Gas): double check if this is correct
    let account2 = info.create_and_fund_user_account(10_000).await.unwrap();

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
    let (module_id, _) = modules.iter().next().unwrap();
    assert_eq!(
        &modules.get(module_id).unwrap().bytecode.0,
        bcs_modules.get(module_id).unwrap()
    );

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
}
