// Copyright © Aptos Foundation

use crate::utils::{TestFailure, NetworkName, get_client, get_faucet_client, create_and_fund_account};
use aptos_api_types::U64;
use aptos_logger::info;
use aptos_rest_client::{Account, Client};
use aptos_sdk::types::LocalAccount;

static FAIL_ACCOUNT_DATA: &str = "wrong account data";
static FAIL_BALANCE: &str = "wrong balance";

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

pub async fn setup_and_run_newaccount(network_name: NetworkName) -> Result<(), TestFailure> {
    // spin up clients
    let client = get_client(network_name);
    let faucet_client = get_faucet_client(network_name);

    // create and fund account
    let account = create_and_fund_account(&faucet_client).await?;

    // run test
    test_newaccount(&client, &account, 100_000_000).await
}
