// Copyright © Aptos Foundation

use crate::{
    fail_message::{
        ERROR_COULD_NOT_CREATE_TRANSACTION, ERROR_COULD_NOT_FINISH_TRANSACTION,
        ERROR_COULD_NOT_FUND_ACCOUNT, ERROR_NO_COLLECTION_DATA, ERROR_NO_TOKEN_BALANCE,
        ERROR_NO_TOKEN_DATA, FAIL_WRONG_COLLECTION_DATA, FAIL_WRONG_TOKEN_BALANCE,
        FAIL_WRONG_TOKEN_DATA,
    },
    persistent_check,
    utils::{create_and_fund_account, get_client, get_faucet_client, NetworkName, TestFailure},
};
use aptos_api_types::U64;
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_sdk::{
    token_client::{
        CollectionData, CollectionMutabilityConfig, RoyaltyOptions, TokenClient, TokenData,
        TokenMutabilityConfig,
    },
    types::LocalAccount,
};
use aptos_types::account_address::AccountAddress;

static COLLECTION_NAME: &str = "test collection";
static TOKEN_NAME: &str = "test token";
static TOKEN_SUPPLY: u64 = 10;
static OFFER_AMOUNT: u64 = 2;

/// Tests nft transfer. Checks that:
///   - collection data exists
///   - token data exists
///   - token balance reflects transferred amount
pub async fn test(network_name: NetworkName) -> Result<(), TestFailure> {
    // setup
    let (client, mut account, mut receiver) = setup(network_name).await?;
    let token_client = TokenClient::new(&client);

    // create collection
    create_collection(&client, &token_client, &mut account).await?;

    // check collection metadata persistently
    persistent_check::token_address(
        "check_collection_metadata",
        check_collection_metadata,
        &token_client,
        account.address(),
    )
    .await?;

    // create token
    create_token(&client, &token_client, &mut account).await?;

    // check token metadata persistently
    persistent_check::token_address(
        "check_token_metadata",
        check_token_metadata,
        &token_client,
        account.address(),
    )
    .await?;

    // offer token
    offer_token(&client, &token_client, &mut account, receiver.address()).await?;

    // check senders balance persistently
    persistent_check::token_address(
        "check_sender_balance",
        check_sender_balance,
        &token_client,
        account.address(),
    )
    .await?;

    // claim token
    claim_token(&client, &token_client, &mut receiver, account.address()).await?;

    // check receivers balance persistently
    persistent_check::token_address_address(
        "check_receiver_balance",
        check_receiver_balance,
        &token_client,
        receiver.address(),
        account.address(),
    )
    .await?;

    Ok(())
}

// Steps

async fn setup(
    network_name: NetworkName,
) -> Result<(Client, LocalAccount, LocalAccount), TestFailure> {
    // spin up clients
    let client = get_client(network_name);
    let faucet_client = get_faucet_client(network_name);

    // create account
    let account = match create_and_fund_account(&faucet_client).await {
        Ok(account) => account,
        Err(e) => {
            info!(
                "test: nft_transfer part: setup ERROR: {}, with error {:?}",
                ERROR_COULD_NOT_FUND_ACCOUNT, e
            );
            return Err(e.into());
        },
    };

    // create receiver
    let receiver = match create_and_fund_account(&faucet_client).await {
        Ok(receiver) => receiver,
        Err(e) => {
            info!(
                "test: nft_transfer part: setup ERROR: {}, with error {:?}",
                ERROR_COULD_NOT_FUND_ACCOUNT, e
            );
            return Err(e.into());
        },
    };

    Ok((client, account, receiver))
}

async fn create_collection(
    client: &Client,
    token_client: &TokenClient<'_>,
    account: &mut LocalAccount,
) -> Result<(), TestFailure> {
    // set up collection data
    let collection_data = collection_data();

    // create transaction
    let pending_txn = match token_client
        .create_collection(
            account,
            &collection_data.name,
            &collection_data.description,
            &collection_data.uri,
            collection_data.maximum.into(),
            None,
        )
        .await
    {
        Ok(txn) => txn,
        Err(e) => {
            info!(
                "test: nft_transfer part: create_collection ERROR: {}, with error {:?}",
                ERROR_COULD_NOT_CREATE_TRANSACTION, e
            );
            return Err(e.into());
        },
    };

    // wait for transaction to finish
    if let Err(e) = client.wait_for_transaction(&pending_txn).await {
        info!(
            "test: nft_transfer part: create_collection ERROR: {}, with error {:?}",
            ERROR_COULD_NOT_FINISH_TRANSACTION, e
        );
        return Err(e.into());
    };

    Ok(())
}

async fn check_collection_metadata(
    token_client: &TokenClient<'_>,
    address: AccountAddress,
) -> Result<(), TestFailure> {
    // set up collection data
    let collection_data = collection_data();

    // expected
    let expected = collection_data.clone();

    // actual
    let actual = match token_client
        .get_collection_data(address, &collection_data.name)
        .await
    {
        Ok(data) => data,
        Err(e) => {
            info!(
                "test: nft_transfer part: check_collection_metadata ERROR: {}, with error {:?}",
                ERROR_NO_COLLECTION_DATA, e
            );
            return Err(e.into());
        },
    };

    // compare
    if expected != actual {
        info!(
            "test: nft_transfer part: check_collection_metadata FAIL: {}, expected {:?}, got {:?}",
            FAIL_WRONG_COLLECTION_DATA, expected, actual
        );
        return Err(TestFailure::Fail(FAIL_WRONG_COLLECTION_DATA));
    }

    Ok(())
}

async fn create_token(
    client: &Client,
    token_client: &TokenClient<'_>,
    account: &mut LocalAccount,
) -> Result<(), TestFailure> {
    // set up token data
    let token_data = token_data(account.address());

    // create transaction
    let pending_txn = match token_client
        .create_token(
            account,
            COLLECTION_NAME,
            &token_data.name,
            &token_data.description,
            token_data.supply.into(),
            &token_data.uri,
            token_data.maximum.into(),
            None,
            None,
        )
        .await
    {
        Ok(txn) => txn,
        Err(e) => {
            info!(
                "test: nft_transfer part: create_token ERROR: {}, with error {:?}",
                ERROR_COULD_NOT_CREATE_TRANSACTION, e
            );
            return Err(e.into());
        },
    };

    // wait for transaction to finish
    if let Err(e) = client.wait_for_transaction(&pending_txn).await {
        info!(
            "test: nft_transfer part: create_token ERROR: {}, with error {:?}",
            ERROR_COULD_NOT_FINISH_TRANSACTION, e
        );
        return Err(e.into());
    };

    Ok(())
}

async fn check_token_metadata(
    token_client: &TokenClient<'_>,
    address: AccountAddress,
) -> Result<(), TestFailure> {
    // set up token data
    let token_data = token_data(address);

    // expected
    let expected = token_data;

    // actual
    let actual = match token_client
        .get_token_data(address, COLLECTION_NAME, TOKEN_NAME)
        .await
    {
        Ok(data) => data,
        Err(e) => {
            info!(
                "test: nft_transfer part: check_token_metadata ERROR: {}, with error {:?}",
                ERROR_NO_TOKEN_DATA, e
            );
            return Err(e.into());
        },
    };

    // compare
    if expected != actual {
        info!(
            "test: nft_transfer part: check_token_metadata FAIL: {}, expected {:?}, got {:?}",
            FAIL_WRONG_TOKEN_DATA, expected, actual
        );
        return Err(TestFailure::Fail(FAIL_WRONG_TOKEN_DATA));
    }

    Ok(())
}

async fn offer_token(
    client: &Client,
    token_client: &TokenClient<'_>,
    account: &mut LocalAccount,
    receiver: AccountAddress,
) -> Result<(), TestFailure> {
    // create transaction
    let pending_txn = match token_client
        .offer_token(
            account,
            receiver,
            account.address(),
            COLLECTION_NAME,
            TOKEN_NAME,
            OFFER_AMOUNT,
            None,
            None,
        )
        .await
    {
        Ok(txn) => txn,
        Err(e) => {
            info!(
                "test: nft_transfer part: offer_token ERROR: {}, with error {:?}",
                ERROR_COULD_NOT_CREATE_TRANSACTION, e
            );
            return Err(e.into());
        },
    };

    // wait for transaction to finish
    if let Err(e) = client.wait_for_transaction(&pending_txn).await {
        info!(
            "test: nft_transfer part: offer_token ERROR: {}, with error {:?}",
            ERROR_COULD_NOT_FINISH_TRANSACTION, e
        );
        return Err(e.into());
    };

    Ok(())
}

async fn check_sender_balance(
    token_client: &TokenClient<'_>,
    address: AccountAddress,
) -> Result<(), TestFailure> {
    check_token_balance(
        token_client,
        address,
        address,
        U64(TOKEN_SUPPLY - OFFER_AMOUNT),
        "check_sender_balance",
    )
    .await
}

async fn claim_token(
    client: &Client,
    token_client: &TokenClient<'_>,
    receiver: &mut LocalAccount,
    sender: AccountAddress,
) -> Result<(), TestFailure> {
    // create transaction
    let pending_txn = match token_client
        .claim_token(
            receiver,
            sender,
            sender,
            COLLECTION_NAME,
            TOKEN_NAME,
            None,
            None,
        )
        .await
    {
        Ok(txn) => txn,
        Err(e) => {
            info!(
                "test: nft_transfer part: claim_token ERROR: {}, with error {:?}",
                ERROR_COULD_NOT_CREATE_TRANSACTION, e
            );
            return Err(e.into());
        },
    };

    // wait for transaction to finish
    if let Err(e) = client.wait_for_transaction(&pending_txn).await {
        info!(
            "test: nft_transfer part: claim_token ERROR: {}, with error {:?}",
            ERROR_COULD_NOT_FINISH_TRANSACTION, e
        );
        return Err(e.into());
    };

    Ok(())
}

async fn check_receiver_balance(
    token_client: &TokenClient<'_>,
    address: AccountAddress,
    creator: AccountAddress,
) -> Result<(), TestFailure> {
    check_token_balance(
        token_client,
        address,
        creator,
        U64(OFFER_AMOUNT),
        "check_receiver_balance",
    )
    .await
}

// Utils

fn collection_data() -> CollectionData {
    CollectionData {
        name: COLLECTION_NAME.to_string(),
        description: "collection description".to_string(),
        uri: "collection uri".to_string(),
        maximum: U64(1000),
        mutability_config: CollectionMutabilityConfig {
            description: false,
            maximum: false,
            uri: false,
        },
    }
}

fn token_data(address: AccountAddress) -> TokenData {
    TokenData {
        name: TOKEN_NAME.to_string(),
        description: "token description".to_string(),
        uri: "token uri".to_string(),
        maximum: U64(1000),
        mutability_config: TokenMutabilityConfig {
            description: false,
            maximum: false,
            properties: false,
            royalty: false,
            uri: false,
        },
        supply: U64(TOKEN_SUPPLY),
        royalty: RoyaltyOptions {
            // change this when you use!
            payee_address: address,
            royalty_points_denominator: U64(0),
            royalty_points_numerator: U64(0),
        },
        largest_property_version: U64(0),
    }
}

async fn check_token_balance(
    token_client: &TokenClient<'_>,
    address: AccountAddress,
    creator: AccountAddress,
    expected: U64,
    part: &str,
) -> Result<(), TestFailure> {
    // actual
    let actual = match token_client
        .get_token(address, creator, COLLECTION_NAME, TOKEN_NAME)
        .await
    {
        Ok(data) => data.amount,
        Err(e) => {
            info!(
                "test: nft_transfer part: {} ERROR: {}, with error {:?}",
                part, ERROR_NO_TOKEN_BALANCE, e
            );
            return Err(e.into());
        },
    };

    // compare
    if expected != actual {
        info!(
            "test: nft_transfer part: {} FAIL: {}, expected {:?}, got {:?}",
            part, FAIL_WRONG_TOKEN_BALANCE, expected, actual
        );
        return Err(TestFailure::Fail(FAIL_WRONG_TOKEN_BALANCE));
    }

    Ok(())
}
