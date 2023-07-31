// Copyright © Aptos Foundation

use crate::utils::{
    create_account, create_and_fund_account, get_client, get_faucet_client, NetworkName,
    TestFailure,
};
use anyhow::{anyhow, Result};
use aptos_api_types::U64;
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_sdk::{coin_client::CoinClient, types::LocalAccount};
use aptos_types::account_address::AccountAddress;

static FAIL_BALANCE_AFTER_TRANSACTION: &str = "wrong balance after transaction";
static FAIL_BALANCE_BEFORE_TRANSACTION: &str = "wrong balance before transaction";
static ERROR_MODULE_INTERACTION: &str = "module interaction isn't reflected";
static ERROR_NO_VERSION: &str = "transaction did not return version";

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

pub async fn setup_and_run_cointransfer(network_name: NetworkName) -> Result<(), TestFailure> {
    // spin up clients
    let client = get_client(network_name);
    let faucet_client = get_faucet_client(network_name);
    let coin_client = CoinClient::new(&client);

    // create and fund accounts
    let mut account = create_and_fund_account(&faucet_client).await?;
    let receiver = create_account(&faucet_client).await?;

    // run test
    test_cointransfer(
        &client,
        &coin_client,
        &mut account,
        receiver.address(),
        1_000,
    )
    .await
}
