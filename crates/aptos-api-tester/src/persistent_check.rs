// Copyright © Aptos Foundation

use crate::{fail_message::ERROR_COULD_NOT_CHECK, utils::TestFailure};
use anyhow::anyhow;
use aptos_rest_client::Client;
use aptos_sdk::{types::LocalAccount, token_client::{TokenClient, Token}};
use aptos_types::account_address::AccountAddress;
use futures::Future;
use std::time::Duration;
use tokio::time::Instant;

pub async fn account<'a, 'b, F, Fut>(
    f: F,
    client: &'a Client,
    account: &'b LocalAccount,
) -> Result<(), TestFailure>
where
    F: Fn(&'a Client, &'b LocalAccount) -> Fut,
    Fut: Future<Output = Result<(), TestFailure>>,
{
    // set a default error in case checks never start
    let mut result: Result<(), TestFailure> = Err(anyhow!(ERROR_COULD_NOT_CHECK).into());
    let timer = Instant::now();

    // try to get a good result for 30 seconds
    while Instant::now().duration_since(timer) < Duration::from_secs(30) {
        result = f(client, account).await;
        if result.is_ok() {
            break;
        }
    }

    // return last failure if no good result occurs
    result
}

pub async fn address<'a, F, Fut>(
    f: F,
    client: &'a Client,
    address: AccountAddress,
) -> Result<(), TestFailure>
where
    F: Fn(&'a Client, AccountAddress) -> Fut,
    Fut: Future<Output = Result<(), TestFailure>>,
{
    // set a default error in case checks never start
    let mut result: Result<(), TestFailure> = Err(anyhow!(ERROR_COULD_NOT_CHECK).into());
    let timer = Instant::now();

    // try to get a good result for 30 seconds
    while Instant::now().duration_since(timer) < Duration::from_secs(30) {
        result = f(client, address).await;
        if result.is_ok() {
            break;
        }
    }

    // return last failure if no good result occurs
    result
}

pub async fn address_version<'a, F, Fut>(
    f: F,
    client: &'a Client,
    address: AccountAddress,
    version: u64,
) -> Result<(), TestFailure>
where
    F: Fn(&'a Client, AccountAddress, u64) -> Fut,
    Fut: Future<Output = Result<(), TestFailure>>,
{
    // set a default error in case checks never start
    let mut result: Result<(), TestFailure> = Err(anyhow!(ERROR_COULD_NOT_CHECK).into());
    let timer = Instant::now();

    // try to get a good result for 30 seconds
    while Instant::now().duration_since(timer) < Duration::from_secs(30) {
        result = f(client, address, version).await;
        if result.is_ok() {
            break;
        }
    }

    // return last failure if no good result occurs
    result
}

pub async fn token_address<'a, F, Fut>(
    f: F,
    token_client: &'a TokenClient<'a>,
    address: AccountAddress,
) -> Result<(), TestFailure>
where
    F: Fn(&'a TokenClient<'a>, AccountAddress) -> Fut,
    Fut: Future<Output = Result<(), TestFailure>>,
{
    // set a default error in case checks never start
    let mut result: Result<(), TestFailure> = Err(anyhow!(ERROR_COULD_NOT_CHECK).into());
    let timer = Instant::now();

    // try to get a good result for 30 seconds
    while Instant::now().duration_since(timer) < Duration::from_secs(30) {
        result = f(token_client, address).await;
        if result.is_ok() {
            break;
        }
    }

    // return last failure if no good result occurs
    result
}

pub async fn token_address_address<'a, F, Fut>(
    f: F,
    token_client: &'a TokenClient<'a>,
    address: AccountAddress,
    address2: AccountAddress,
) -> Result<(), TestFailure>
where
    F: Fn(&'a TokenClient<'a>, AccountAddress, AccountAddress) -> Fut,
    Fut: Future<Output = Result<(), TestFailure>>,
{
    // set a default error in case checks never start
    let mut result: Result<(), TestFailure> = Err(anyhow!(ERROR_COULD_NOT_CHECK).into());
    let timer = Instant::now();

    // try to get a good result for 30 seconds
    while Instant::now().duration_since(timer) < Duration::from_secs(30) {
        result = f(token_client, address, address2).await;
        if result.is_ok() {
            break;
        }
    }

    // return last failure if no good result occurs
    result
}
