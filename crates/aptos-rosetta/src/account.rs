// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Rosetta Account API
//!
//! See: [Account API Spec](https://www.rosetta-api.org/docs/AccountApi.html)
//!

use crate::{
    common::{check_network, handle_request, with_context},
    error::ApiError,
    types::{AccountBalanceRequest, AccountBalanceResponse, Amount, BlockIdentifier, Currency},
    RosettaContext,
};
use aptos_logger::{debug, trace};
use aptos_types::account_address::AccountAddress;
use std::str::FromStr;
use warp::Filter;

/// Account routes e.g. balance
pub fn routes(
    server_context: RosettaContext,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::post().and(
        warp::path!("account" / "balance")
            .and(warp::body::json())
            .and(with_context(server_context))
            .and_then(handle_request(account_balance)),
    )
}

/// Account balance command
///
/// [API Spec](https://www.rosetta-api.org/docs/AccountApi.html#accountbalance)
async fn account_balance(
    request: AccountBalanceRequest,
    server_context: RosettaContext,
) -> Result<AccountBalanceResponse, ApiError> {
    debug!("/account/balance");
    trace!(
        request = ?request,
        server_context = ?server_context,
        "account_balance for [{}]",
        request.account_identifier.address
    );

    let network_identifier = request.network_identifier;

    check_network(network_identifier, &server_context)?;

    // TODO: support lookups of account balance at specific blocks for now
    if request.block_identifier.is_some() {
        return Err(ApiError::HistoricBalancesUnsupported);
    }

    let rest_client = server_context.rest_client;

    let address = AccountAddress::from_str(&request.account_identifier.address)
        .map_err(|err| ApiError::AptosError(err.to_string()))?;
    let response = rest_client.get_account(address).await;
    let response = response.map_err(|_| ApiError::AccountNotFound)?;
    let state = response.state();
    let txns = rest_client
        .get_transactions(Some(state.version), Some(1))
        .await
        .map_err(|err| ApiError::AptosError(err.to_string()))?
        .into_inner();
    let txn = txns
        .first()
        .ok_or_else(|| ApiError::AptosError("Transaction not found".to_string()))?;

    let txn_info = txn
        .transaction_info()
        .map_err(|err| ApiError::AptosError(err.to_string()))?;

    let block_identifier = BlockIdentifier {
        index: txn_info.version.0,
        hash: txn_info.hash.to_string(),
    };

    let response = rest_client
        .get_account_balance(address)
        .await
        .map_err(|err| ApiError::AptosError(err.to_string()))?;
    let balance = response.into_inner();

    // TODO: Cleanup to match reality
    let balances = vec![Amount {
        value: balance.coin.value.to_string(),
        currency: Currency {
            symbol: "APTOS".to_string(),
            decimals: 6,
        },
    }];

    let response = AccountBalanceResponse {
        block_identifier,
        balances,
    };

    Ok(response)
}
