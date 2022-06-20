// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Rosetta Account API
//!
//! See: [Account API Spec](https://www.rosetta-api.org/docs/AccountApi.html)
//!

use crate::{
    common::{check_network, get_account, get_account_balance, handle_request, with_context},
    error::{ApiError, ApiResult},
    types::{AccountBalanceRequest, AccountBalanceResponse},
    RosettaContext,
};
use aptos_logger::{debug, trace};
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
) -> ApiResult<AccountBalanceResponse> {
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
    let rest_client = server_context.rest_client()?;
    let address = request.account_identifier.account_address()?;
    let response = get_account(rest_client, address).await?;
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

    let response = get_account_balance(rest_client, address).await?;
    let balance = response.into_inner();

    let response = AccountBalanceResponse {
        block_identifier: txn_info.into(),
        balances: vec![balance.into()],
    };

    Ok(response)
}
