// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// Persistent checking is a mechanism to increase tolerancy to eventual consistency issues. In our
// earlier tests we have observed that parallel runs of the flows returned higher failure rates
// than serial runs, and these extra failures displayed the following pattern: 1) the flow submits
// a transaction to the API (such as account creation), 2) the flow reads the state from the API,
// and gets a result that does not include the transaction. We attribute this to the second call
// ending up on a different node which is not yet up to sync. Therefore, for state checks, we
// repeat the whole check for a period of time until it is successful, and throw a failure only if
// it fails to succeed. Note that every time a check fails we will still get a failure log.

// TODO: The need for having a different persistent check wrapper for each function signature is
// due to a lack of overloading in Rust. Consider using macros to reduce code duplication.

use crate::{
    consts::{PERSISTENCY_TIMEOUT, SLEEP_PER_CYCLE},
    strings::ERROR_COULD_NOT_CHECK,
    tokenv1_client::TokenClient,
    utils::TestFailure,
};
use anyhow::anyhow;
use velor_api_types::HexEncodedBytes;
use velor_rest_client::Client;
use velor_sdk::types::LocalAccount;
use velor_types::account_address::AccountAddress;
use futures::Future;
use tokio::time::{sleep, Instant};

pub async fn account<'a, 'b, F, Fut>(
    step: &str,
    f: F,
    client: &'a Client,
    account: &'b LocalAccount,
) -> Result<(), TestFailure>
where
    F: Fn(&'a Client, &'b LocalAccount) -> Fut,
    Fut: Future<Output = Result<(), TestFailure>>,
{
    // set a default error in case checks never start
    let mut result: Result<(), TestFailure> = Err(could_not_check(step));
    let timer = Instant::now();

    // try to get a good result
    while Instant::now().duration_since(timer) < *PERSISTENCY_TIMEOUT {
        result = f(client, account).await;
        if result.is_ok() {
            break;
        }
        sleep(*SLEEP_PER_CYCLE).await;
    }

    // return last failure if no good result occurs
    result
}

pub async fn address<'a, F, Fut>(
    step: &str,
    f: F,
    client: &'a Client,
    address: AccountAddress,
) -> Result<(), TestFailure>
where
    F: Fn(&'a Client, AccountAddress) -> Fut,
    Fut: Future<Output = Result<(), TestFailure>>,
{
    // set a default error in case checks never start
    let mut result: Result<(), TestFailure> = Err(could_not_check(step));
    let timer = Instant::now();

    // try to get a good result
    while Instant::now().duration_since(timer) < *PERSISTENCY_TIMEOUT {
        result = f(client, address).await;
        if result.is_ok() {
            break;
        }
        sleep(*SLEEP_PER_CYCLE).await;
    }

    // return last failure if no good result occurs
    result
}

pub async fn address_address<'a, F, Fut>(
    step: &str,
    f: F,
    client: &'a Client,
    address: AccountAddress,
    address2: AccountAddress,
) -> Result<(), TestFailure>
where
    F: Fn(&'a Client, AccountAddress, AccountAddress) -> Fut,
    Fut: Future<Output = Result<(), TestFailure>>,
{
    // set a default error in case checks never start
    let mut result: Result<(), TestFailure> = Err(could_not_check(step));
    let timer = Instant::now();

    // try to get a good result
    while Instant::now().duration_since(timer) < *PERSISTENCY_TIMEOUT {
        result = f(client, address, address2).await;
        if result.is_ok() {
            break;
        }
        sleep(*SLEEP_PER_CYCLE).await;
    }

    // return last failure if no good result occurs
    result
}

pub async fn address_bytes<'a, 'b, F, Fut>(
    step: &str,
    f: F,
    client: &'a Client,
    address: AccountAddress,
    bytes: &'b HexEncodedBytes,
) -> Result<(), TestFailure>
where
    F: Fn(&'a Client, AccountAddress, &'b HexEncodedBytes) -> Fut,
    Fut: Future<Output = Result<(), TestFailure>>,
{
    // set a default error in case checks never start
    let mut result: Result<(), TestFailure> = Err(could_not_check(step));
    let timer = Instant::now();

    // try to get a good result
    while Instant::now().duration_since(timer) < *PERSISTENCY_TIMEOUT {
        result = f(client, address, bytes).await;
        if result.is_ok() {
            break;
        }
        sleep(*SLEEP_PER_CYCLE).await;
    }

    // return last failure if no good result occurs
    result
}

pub async fn address_version<'a, F, Fut>(
    step: &str,
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
    let mut result: Result<(), TestFailure> = Err(could_not_check(step));
    let timer = Instant::now();

    // try to get a good result
    while Instant::now().duration_since(timer) < *PERSISTENCY_TIMEOUT {
        result = f(client, address, version).await;
        if result.is_ok() {
            break;
        }
        sleep(*SLEEP_PER_CYCLE).await;
    }

    // return last failure if no good result occurs
    result
}

pub async fn token_address<'a, F, Fut>(
    step: &str,
    f: F,
    token_client: &'a TokenClient<'a>,
    address: AccountAddress,
) -> Result<(), TestFailure>
where
    F: Fn(&'a TokenClient<'a>, AccountAddress) -> Fut,
    Fut: Future<Output = Result<(), TestFailure>>,
{
    // set a default error in case checks never start
    let mut result: Result<(), TestFailure> = Err(could_not_check(step));
    let timer = Instant::now();

    // try to get a good result
    while Instant::now().duration_since(timer) < *PERSISTENCY_TIMEOUT {
        result = f(token_client, address).await;
        if result.is_ok() {
            break;
        }
        sleep(*SLEEP_PER_CYCLE).await;
    }

    // return last failure if no good result occurs
    result
}

pub async fn token_address_address<'a, F, Fut>(
    step: &str,
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
    let mut result: Result<(), TestFailure> = Err(could_not_check(step));
    let timer = Instant::now();

    // try to get a good result
    while Instant::now().duration_since(timer) < *PERSISTENCY_TIMEOUT {
        result = f(token_client, address, address2).await;
        if result.is_ok() {
            break;
        }
        sleep(*SLEEP_PER_CYCLE).await;
    }

    // return last failure if no good result occurs
    result
}

// Utils

fn could_not_check(step: &str) -> TestFailure {
    anyhow!(format!("{} in step: {}", ERROR_COULD_NOT_CHECK, step)).into()
}
