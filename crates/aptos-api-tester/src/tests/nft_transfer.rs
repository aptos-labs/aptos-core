// Copyright © Aptos Foundation

use aptos_api_types::U64;
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_sdk::{token_client::{TokenClient, CollectionData, CollectionMutabilityConfig, TokenData, TokenMutabilityConfig, RoyaltyOptions}, types::LocalAccount};

use crate::utils::{TestFailure, NetworkName, get_client, get_faucet_client, create_and_fund_account};

static FAIL_COLLECTION_DATA: &str = "wrong collection data";
static FAIL_TOKEN_DATA: &str = "wrong token data";
static FAIL_TOKEN_BALANCE: &str = "wrong token balance";
static FAIL_TOKENS_BEFORE_CLAIM: &str = "found tokens for receiver when shouldn't";
static FAIL_TOKEN_BALANCE_AFTER_TRANSACTION: &str = "wrong token balance after transaction";

/// Tests nft transfer. Checks that:
///   - collection data exists
///   - token data exists
///   - token balance reflects transferred amount
pub async fn test_nfttransfer(
    client: &Client,
    token_client: &TokenClient<'_>,
    account: &mut LocalAccount,
    receiver: &mut LocalAccount,
) -> Result<(), TestFailure> {
    // create collection
    let collection_name = "test collection".to_string();
    let collection_description = "collection description".to_string();
    let collection_uri = "collection uri".to_string();
    let collection_maximum = 1000;

    let pending_txn = token_client
        .create_collection(
            account,
            &collection_name,
            &collection_description,
            &collection_uri,
            collection_maximum,
            None,
        )
        .await?;
    client.wait_for_transaction(&pending_txn).await?;

    // create token
    let token_name = "test token".to_string();
    let token_description = "token description".to_string();
    let token_uri = "token uri".to_string();
    let token_maximum = 1000;
    let token_supply = 10;

    let pending_txn = token_client
        .create_token(
            account,
            &collection_name,
            &token_name,
            &token_description,
            token_supply,
            &token_uri,
            token_maximum,
            None,
            None,
        )
        .await?;
    client.wait_for_transaction(&pending_txn).await?;

    // check collection metadata
    let expected_collection_data = CollectionData {
        name: collection_name.clone(),
        description: collection_description,
        uri: collection_uri,
        maximum: U64(collection_maximum),
        mutability_config: CollectionMutabilityConfig {
            description: false,
            maximum: false,
            uri: false,
        },
    };
    let actual_collection_data = token_client
        .get_collection_data(account.address(), &collection_name)
        .await?;

    if expected_collection_data != actual_collection_data {
        info!(
            "fail: {}, expected {:?}, got {:?}",
            FAIL_COLLECTION_DATA, expected_collection_data, actual_collection_data
        );
        return Err(TestFailure::Fail(FAIL_COLLECTION_DATA));
    }

    // check token metadata
    let expected_token_data = TokenData {
        name: token_name.clone(),
        description: token_description,
        uri: token_uri,
        maximum: U64(token_maximum),
        mutability_config: TokenMutabilityConfig {
            description: false,
            maximum: false,
            properties: false,
            royalty: false,
            uri: false,
        },
        supply: U64(token_supply),
        royalty: RoyaltyOptions {
            payee_address: account.address(),
            royalty_points_denominator: U64(0),
            royalty_points_numerator: U64(0),
        },
        largest_property_version: U64(0),
    };
    let actual_token_data = token_client
        .get_token_data(account.address(), &collection_name, &token_name)
        .await?;

    if expected_token_data != actual_token_data {
        info!(
            "fail: {}, expected {:?}, got {:?}",
            FAIL_TOKEN_DATA, expected_token_data, actual_token_data
        );
        return Err(TestFailure::Fail(FAIL_TOKEN_DATA));
    }

    // offer token
    let pending_txn = token_client
        .offer_token(
            account,
            receiver.address(),
            account.address(),
            &collection_name,
            &token_name,
            2,
            None,
            None,
        )
        .await?;
    client.wait_for_transaction(&pending_txn).await?;

    // check token balance for the sender
    let expected_sender_token_balance = U64(8);
    let actual_sender_token_balance = token_client
        .get_token(
            account.address(),
            account.address(),
            &collection_name,
            &token_name,
        )
        .await?
        .amount;

    if expected_sender_token_balance != actual_sender_token_balance {
        info!(
            "fail: {}, expected {:?}, got {:?}",
            FAIL_TOKEN_BALANCE, expected_sender_token_balance, actual_sender_token_balance
        );
        return Err(TestFailure::Fail(FAIL_TOKEN_BALANCE));
    }

    // check that token store isn't initialized for the receiver
    if token_client
        .get_token(
            receiver.address(),
            account.address(),
            &collection_name,
            &token_name,
        )
        .await
        .is_ok()
    {
        info!(
            "fail: {}, expected no token client resource for the receiver",
            FAIL_TOKENS_BEFORE_CLAIM
        );
        return Err(TestFailure::Fail(FAIL_TOKENS_BEFORE_CLAIM));
    }

    // claim token
    let pending_txn = token_client
        .claim_token(
            receiver,
            account.address(),
            account.address(),
            &collection_name,
            &token_name,
            None,
            None,
        )
        .await?;
    client.wait_for_transaction(&pending_txn).await?;

    // check token balance for the receiver
    let expected_receiver_token_balance = U64(2);
    let actual_receiver_token_balance = token_client
        .get_token(
            receiver.address(),
            account.address(),
            &collection_name,
            &token_name,
        )
        .await?
        .amount;

    if expected_receiver_token_balance != actual_receiver_token_balance {
        info!(
            "{}, expected {:?}, got {:?}",
            FAIL_TOKEN_BALANCE_AFTER_TRANSACTION,
            expected_receiver_token_balance,
            actual_receiver_token_balance
        );
        return Err(TestFailure::Fail(FAIL_TOKEN_BALANCE_AFTER_TRANSACTION));
    }

    Ok(())
}

pub async fn setup_and_run_nfttransfer(network_name: NetworkName) -> Result<(), TestFailure> {
    // spin up clients
    let client = get_client(network_name);
    let faucet_client = get_faucet_client(network_name);
    let token_client = TokenClient::new(&client);

    // create and fund accounts
    let mut account = create_and_fund_account(&faucet_client).await?;
    let mut receiver = create_and_fund_account(&faucet_client).await?;

    // run test
    test_nfttransfer(&client, &token_client, &mut account, &mut receiver).await
}
