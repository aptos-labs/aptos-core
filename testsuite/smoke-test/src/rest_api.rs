// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos::common::types::account_address_from_public_key;
use aptos_crypto::PrivateKey;
use aptos_sdk::move_types::language_storage::StructTag;
use aptos_transaction_builder::aptos_stdlib;
use aptos_types::account_address::AccountAddress;
use aptos_types::account_config::{AccountResource, CORE_CODE_ADDRESS};
use aptos_types::transaction::authenticator::AuthenticationKey;
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
    // FIXME: Use swarm instead of local node
    // let mut swarm = new_local_swarm_with_aptos(1).await;
    //let mut info = swarm.aptos_public_info();
    //let client = info.client();
    let rest_api = reqwest::Url::parse("http://localhost:8080/v1").unwrap();
    let client = aptos_rest_client::Client::new(rest_api.clone());
    let faucet_client = aptos_rest_client::FaucetClient::new(
        reqwest::Url::parse("http://localhost:8081").unwrap(),
        rest_api,
    );

    // Create account
    let mut keygen = aptos_keygen::KeyGen::from_seed([0u8; 32]);
    let private_key = keygen.generate_ed25519_private_key();
    let public_key = private_key.public_key();
    let account = account_address_from_public_key(&public_key);

    // Fund account
    faucet_client.fund(account, 10000000).await.unwrap();

    // Check get account
    let account_resource = client.get_account_bcs(account).await.unwrap().into_inner();
    let expected_auth_key = AuthenticationKey::ed25519(&public_key);
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
}
