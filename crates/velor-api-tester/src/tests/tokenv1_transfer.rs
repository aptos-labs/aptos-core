// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consts::FUND_AMOUNT,
    persistent_check,
    strings::{
        CHECK_ACCOUNT_DATA, CHECK_COLLECTION_METADATA, CHECK_RECEIVER_BALANCE,
        CHECK_SENDER_BALANCE, CHECK_TOKEN_METADATA, CLAIM_TOKEN, CREATE_COLLECTION, CREATE_TOKEN,
        ERROR_COULD_NOT_CREATE_AND_SUBMIT_TRANSACTION, ERROR_COULD_NOT_FINISH_TRANSACTION,
        ERROR_COULD_NOT_FUND_ACCOUNT, ERROR_NO_COLLECTION_DATA, ERROR_NO_TOKEN_BALANCE,
        ERROR_NO_TOKEN_DATA, FAIL_WRONG_COLLECTION_DATA, FAIL_WRONG_TOKEN_BALANCE,
        FAIL_WRONG_TOKEN_DATA, OFFER_TOKEN, SETUP,
    },
    time_fn,
    tokenv1_client::{
        CollectionData, CollectionMutabilityConfig, RoyaltyOptions, TokenClient, TokenData,
        TokenMutabilityConfig,
    },
    utils::{
        check_balance, create_and_fund_account, emit_step_metrics, NetworkName, TestFailure,
        TestName,
    },
};
use velor_api_types::U64;
use velor_logger::error;
use velor_rest_client::Client;
use velor_sdk::types::LocalAccount;
use velor_types::account_address::AccountAddress;

const COLLECTION_NAME: &str = "test collection";
const TOKEN_NAME: &str = "test token";
const TOKEN_SUPPLY: u64 = 10;
const OFFER_AMOUNT: u64 = 2;

/// Tests nft transfer. Checks that:
///   - collection data exists
///   - token data exists
///   - token balance reflects transferred amount
pub async fn test(network_name: NetworkName, run_id: &str) -> Result<(), TestFailure> {
    // setup
    let (client, mut account, mut receiver) = emit_step_metrics(
        time_fn!(setup, network_name),
        TestName::TokenV1Transfer,
        SETUP,
        network_name,
        run_id,
    )?;
    let token_client = TokenClient::new(&client);

    // persistently check that API returns correct account data (auth key and sequence number)
    emit_step_metrics(
        time_fn!(
            persistent_check::address_address,
            CHECK_ACCOUNT_DATA,
            check_account_data,
            &client,
            account.address(),
            receiver.address()
        ),
        TestName::TokenV1Transfer,
        CHECK_ACCOUNT_DATA,
        network_name,
        run_id,
    )?;

    // create collection
    emit_step_metrics(
        time_fn!(create_collection, &client, &token_client, &mut account),
        TestName::TokenV1Transfer,
        CREATE_COLLECTION,
        network_name,
        run_id,
    )?;

    // persistently check that API returns correct collection metadata
    emit_step_metrics(
        time_fn!(
            persistent_check::token_address,
            CHECK_COLLECTION_METADATA,
            check_collection_metadata,
            &token_client,
            account.address()
        ),
        TestName::TokenV1Transfer,
        CHECK_COLLECTION_METADATA,
        network_name,
        run_id,
    )?;

    // create token
    emit_step_metrics(
        time_fn!(create_token, &client, &token_client, &mut account),
        TestName::TokenV1Transfer,
        CREATE_TOKEN,
        network_name,
        run_id,
    )?;

    // persistently check that API returns correct token metadata
    emit_step_metrics(
        time_fn!(
            persistent_check::token_address,
            CHECK_TOKEN_METADATA,
            check_token_metadata,
            &token_client,
            account.address()
        ),
        TestName::TokenV1Transfer,
        CHECK_TOKEN_METADATA,
        network_name,
        run_id,
    )?;

    // offer token
    emit_step_metrics(
        time_fn!(
            offer_token,
            &client,
            &token_client,
            &mut account,
            receiver.address()
        ),
        TestName::TokenV1Transfer,
        OFFER_TOKEN,
        network_name,
        run_id,
    )?;

    // persistently check that sender token balance is correct
    emit_step_metrics(
        time_fn!(
            persistent_check::token_address,
            CHECK_SENDER_BALANCE,
            check_sender_balance,
            &token_client,
            account.address()
        ),
        TestName::TokenV1Transfer,
        CHECK_SENDER_BALANCE,
        network_name,
        run_id,
    )?;

    // claim token
    emit_step_metrics(
        time_fn!(
            claim_token,
            &client,
            &token_client,
            &mut receiver,
            account.address()
        ),
        TestName::TokenV1Transfer,
        CLAIM_TOKEN,
        network_name,
        run_id,
    )?;

    // persistently check that receiver token balance is correct
    emit_step_metrics(
        time_fn!(
            persistent_check::token_address_address,
            CHECK_RECEIVER_BALANCE,
            check_receiver_balance,
            &token_client,
            receiver.address(),
            account.address()
        ),
        TestName::TokenV1Transfer,
        CHECK_RECEIVER_BALANCE,
        network_name,
        run_id,
    )?;

    Ok(())
}

// Steps

async fn setup(
    network_name: NetworkName,
) -> Result<(Client, LocalAccount, LocalAccount), TestFailure> {
    // spin up clients
    let client = network_name.get_client();
    let faucet_client = network_name.get_faucet_client();

    // create account
    let account = match create_and_fund_account(&faucet_client, TestName::TokenV1Transfer).await {
        Ok(account) => account,
        Err(e) => {
            error!(
                "test: nft_transfer part: setup ERROR: {}, with error {:?}",
                ERROR_COULD_NOT_FUND_ACCOUNT, e
            );
            return Err(e.into());
        },
    };

    // create receiver
    let receiver = match create_and_fund_account(&faucet_client, TestName::TokenV1Transfer).await {
        Ok(receiver) => receiver,
        Err(e) => {
            error!(
                "test: nft_transfer part: setup ERROR: {}, with error {:?}",
                ERROR_COULD_NOT_FUND_ACCOUNT, e
            );
            return Err(e.into());
        },
    };

    Ok((client, account, receiver))
}

async fn check_account_data(
    client: &Client,
    account: AccountAddress,
    receiver: AccountAddress,
) -> Result<(), TestFailure> {
    check_balance(TestName::TokenV1Transfer, client, account, U64(FUND_AMOUNT)).await?;
    check_balance(
        TestName::TokenV1Transfer,
        client,
        receiver,
        U64(FUND_AMOUNT),
    )
    .await?;

    Ok(())
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
            error!(
                "test: nft_transfer part: create_collection ERROR: {}, with error {:?}",
                ERROR_COULD_NOT_CREATE_AND_SUBMIT_TRANSACTION, e
            );
            return Err(e.into());
        },
    };

    // wait for transaction to finish
    if let Err(e) = client.wait_for_transaction(&pending_txn).await {
        error!(
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
            error!(
                "test: nft_transfer part: check_collection_metadata ERROR: {}, with error {:?}",
                ERROR_NO_COLLECTION_DATA, e
            );
            return Err(e.into());
        },
    };

    // compare
    if expected != actual {
        error!(
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
            error!(
                "test: nft_transfer part: create_token ERROR: {}, with error {:?}",
                ERROR_COULD_NOT_CREATE_AND_SUBMIT_TRANSACTION, e
            );
            return Err(e.into());
        },
    };

    // wait for transaction to finish
    if let Err(e) = client.wait_for_transaction(&pending_txn).await {
        error!(
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
            error!(
                "test: nft_transfer part: check_token_metadata ERROR: {}, with error {:?}",
                ERROR_NO_TOKEN_DATA, e
            );
            return Err(e.into());
        },
    };

    // compare
    if expected != actual {
        error!(
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
            error!(
                "test: nft_transfer part: offer_token ERROR: {}, with error {:?}",
                ERROR_COULD_NOT_CREATE_AND_SUBMIT_TRANSACTION, e
            );
            return Err(e.into());
        },
    };

    // wait for transaction to finish
    if let Err(e) = client.wait_for_transaction(&pending_txn).await {
        error!(
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
            error!(
                "test: nft_transfer part: claim_token ERROR: {}, with error {:?}",
                ERROR_COULD_NOT_CREATE_AND_SUBMIT_TRANSACTION, e
            );
            return Err(e.into());
        },
    };

    // wait for transaction to finish
    if let Err(e) = client.wait_for_transaction(&pending_txn).await {
        error!(
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
            error!(
                "test: nft_transfer part: {} ERROR: {}, with error {:?}",
                part, ERROR_NO_TOKEN_BALANCE, e
            );
            return Err(e.into());
        },
    };

    // compare
    if expected != actual {
        error!(
            "test: nft_transfer part: {} FAIL: {}, expected {:?}, got {:?}",
            part, FAIL_WRONG_TOKEN_BALANCE, expected, actual
        );
        return Err(TestFailure::Fail(FAIL_WRONG_TOKEN_BALANCE));
    }

    Ok(())
}
