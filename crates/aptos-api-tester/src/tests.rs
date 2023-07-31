// Copyright © Aptos Foundation

use crate::utils::TestFailure;
use anyhow::{anyhow, Result};
use aptos_api_types::{HexEncodedBytes, U64};
use aptos_cached_packages::aptos_stdlib::EntryFunctionCall;
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_logger::info;
use aptos_rest_client::{Account, Client};
use aptos_sdk::{
    bcs,
    coin_client::CoinClient,
    token_client::{
        build_and_submit_transaction, CollectionData, CollectionMutabilityConfig, RoyaltyOptions,
        TokenClient, TokenData, TokenMutabilityConfig, TransactionOptions,
    },
    types::LocalAccount,
};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{EntryFunction, TransactionPayload},
};
use move_core_types::{ident_str, language_storage::ModuleId};
use std::{collections::BTreeMap, path::PathBuf};

// fail messages
static FAIL_ACCOUNT_DATA: &str = "wrong account data";
static FAIL_BALANCE: &str = "wrong balance";
static FAIL_BALANCE_AFTER_TRANSACTION: &str = "wrong balance after transaction";
static FAIL_BALANCE_BEFORE_TRANSACTION: &str = "wrong balance before transaction";
static FAIL_COLLECTION_DATA: &str = "wrong collection data";
static FAIL_TOKEN_DATA: &str = "wrong token data";
static FAIL_TOKEN_BALANCE: &str = "wrong token balance";
static FAIL_TOKENS_BEFORE_CLAIM: &str = "found tokens for receiver when shouldn't";
static FAIL_TOKEN_BALANCE_AFTER_TRANSACTION: &str = "wrong token balance after transaction";
static FAIL_BYTECODE: &str = "wrong bytecode";
static FAIL_MODULE_INTERACTION: &str = "module interaction isn't reflected correctly";
static ERROR_NO_VERSION: &str = "transaction did not return version";
static ERROR_NO_BYTECODE: &str = "error while getting bytecode from blobs";
static ERROR_MODULE_INTERACTION: &str = "module interaction isn't reflected";

/// Tests new account creation. Checks that:
///   - account data exists
///   - account balance reflects funded amount
pub async fn test_newaccount(
    client: &Client,
    account: &LocalAccount,
    amount_funded: u64,
) -> Result<(), TestFailure> {
    // ask for account data
    let response = client.get_account(account.address()).await?;

    // check account data
    let expected_account = Account {
        authentication_key: account.authentication_key(),
        sequence_number: account.sequence_number(),
    };
    let actual_account = response.inner();

    if &expected_account != actual_account {
        info!(
            "fail: {}, expected {:?}, got {:?}",
            FAIL_ACCOUNT_DATA, expected_account, actual_account
        );
        return Err(TestFailure::Fail(FAIL_ACCOUNT_DATA));
    }

    // check account balance
    let expected_balance = U64(amount_funded);
    let actual_balance = client
        .get_account_balance(account.address())
        .await?
        .inner()
        .coin
        .value;

    if expected_balance != actual_balance {
        info!(
            "fail: {}, expected {:?}, got {:?}",
            FAIL_BALANCE, expected_balance, actual_balance
        );
        return Err(TestFailure::Fail(FAIL_BALANCE));
    }

    Ok(())
}

/// Tests coin transfer. Checks that:
///   - receiver balance reflects transferred amount
///   - receiver balance shows correct amount at the previous version
pub async fn test_cointransfer(
    client: &Client,
    coin_client: &CoinClient<'_>,
    account: &mut LocalAccount,
    receiver: AccountAddress,
    amount: u64,
) -> Result<(), TestFailure> {
    // get starting balance
    let starting_receiver_balance = u64::from(
        client
            .get_account_balance(receiver)
            .await?
            .inner()
            .coin
            .value,
    );

    // transfer coins to second account
    let pending_txn = coin_client
        .transfer(account, receiver, amount, None)
        .await?;
    let response = client.wait_for_transaction(&pending_txn).await?;

    // check receiver balance
    let expected_receiver_balance = U64(starting_receiver_balance + amount);
    let actual_receiver_balance = client
        .get_account_balance(receiver)
        .await?
        .inner()
        .coin
        .value;

    if expected_receiver_balance != actual_receiver_balance {
        info!(
            "fail: {}, expected {:?}, got {:?}",
            FAIL_BALANCE_AFTER_TRANSACTION, expected_receiver_balance, actual_receiver_balance
        );
        return Err(TestFailure::Fail(FAIL_BALANCE_AFTER_TRANSACTION));
    }

    // check account balance with a lower version number
    let version = match response.inner().version() {
        Some(version) => version,
        _ => {
            info!("error: {}", ERROR_MODULE_INTERACTION);
            return Err(TestFailure::Error(anyhow!(ERROR_NO_VERSION)));
        },
    };

    let expected_balance_at_version = U64(starting_receiver_balance);
    let actual_balance_at_version = client
        .get_account_balance_at_version(receiver, version - 1)
        .await?
        .inner()
        .coin
        .value;

    if expected_balance_at_version != actual_balance_at_version {
        info!(
            "fail: {}, expected {:?}, got {:?}",
            FAIL_BALANCE_BEFORE_TRANSACTION, expected_balance_at_version, actual_balance_at_version
        );
        return Err(TestFailure::Fail(FAIL_BALANCE_BEFORE_TRANSACTION));
    }

    Ok(())
}

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

/// Helper function that publishes module and returns the bytecode.
async fn publish_module(client: &Client, account: &mut LocalAccount) -> Result<HexEncodedBytes> {
    // get file to compile
    let move_dir = PathBuf::from("./aptos-move/move-examples/hello_blockchain");

    // insert address
    let mut named_addresses: BTreeMap<String, AccountAddress> = BTreeMap::new();
    named_addresses.insert("hello_blockchain".to_string(), account.address());

    // build options
    let options = BuildOptions {
        named_addresses,
        ..BuildOptions::default()
    };

    // build module
    let package = BuiltPackage::build(move_dir, options)?;
    let blobs = package.extract_code();
    let metadata = package.extract_metadata()?;

    // create payload
    let payload: aptos_types::transaction::TransactionPayload =
        EntryFunctionCall::CodePublishPackageTxn {
            metadata_serialized: bcs::to_bytes(&metadata)
                .expect("PackageMetadata should deserialize"),
            code: blobs.clone(),
        }
        .encode();

    // create and submit transaction
    let pending_txn =
        build_and_submit_transaction(client, account, payload, TransactionOptions::default())
            .await?;
    client.wait_for_transaction(&pending_txn).await?;

    let blob = match blobs.get(0) {
        Some(bytecode) => bytecode.clone(),
        None => {
            info!("error: {}", ERROR_NO_BYTECODE);
            return Err(anyhow!(ERROR_NO_BYTECODE));
        },
    };

    Ok(HexEncodedBytes::from(blob))
}

/// Helper function that interacts with the message module.
async fn set_message(client: &Client, account: &mut LocalAccount, message: &str) -> Result<()> {
    // create payload
    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(account.address(), ident_str!("message").to_owned()),
        ident_str!("set_message").to_owned(),
        vec![],
        vec![bcs::to_bytes(message)?],
    ));

    // create and submit transaction
    let pending_txn =
        build_and_submit_transaction(client, account, payload, TransactionOptions::default())
            .await?;
    client.wait_for_transaction(&pending_txn).await?;

    Ok(())
}

/// Helper function that gets back the result of the interaction.
async fn get_message(client: &Client, address: AccountAddress) -> Option<String> {
    let resource = match client
        .get_account_resource(
            address,
            format!("{}::message::MessageHolder", address.to_hex_literal()).as_str(),
        )
        .await
    {
        Ok(response) => response.into_inner()?,
        Err(_) => return None,
    };

    Some(resource.data.get("message")?.as_str()?.to_owned())
}

/// Tests module publishing and interaction. Checks that:
///   - module data exists
///   - can interact with module
///   - resources reflect interaction
pub async fn test_publishmodule(
    client: &Client,
    account: &mut LocalAccount,
) -> Result<(), TestFailure> {
    // publish module
    let blob = publish_module(client, account).await?;

    // check module data
    let response = client
        .get_account_module(account.address(), "message")
        .await?;

    let expected_bytecode = &blob;
    let actual_bytecode = &response.inner().bytecode;

    if expected_bytecode != actual_bytecode {
        info!(
            "fail: {}, expected {:?}, got {:?}",
            FAIL_BYTECODE, expected_bytecode, actual_bytecode
        );
        return Err(TestFailure::Fail(FAIL_BYTECODE));
    }

    // interact with module
    let message = "test message";
    set_message(client, account, message).await?;

    // check that the message is sent
    let expected_message = message.to_string();
    let actual_message = match get_message(client, account.address()).await {
        Some(message) => message,
        None => {
            info!("error: {}", ERROR_MODULE_INTERACTION);
            return Err(TestFailure::Error(anyhow!(ERROR_MODULE_INTERACTION)));
        },
    };

    if expected_message != actual_message {
        info!(
            "fail: {}, expected {:?}, got {:?}",
            FAIL_MODULE_INTERACTION, expected_message, actual_message
        );
        return Err(TestFailure::Fail(FAIL_MODULE_INTERACTION));
    }

    Ok(())
}
