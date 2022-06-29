// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::{ApiError, ApiResult},
    types::NetworkIdentifier,
    RosettaContext,
};
use aptos_crypto::ValidCryptoMaterial;
use aptos_logger::debug;
use aptos_rest_client::{aptos::Balance, Account, Response, Transaction};
use aptos_types::{account_address::AccountAddress, chain_id::ChainId};
use futures::future::BoxFuture;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    convert::{Infallible, TryInto},
    future::Future,
    str::FromStr,
};
use warp::Filter;

pub const BLOCKCHAIN: &str = "aptos";

/// Checks the request network matches the server network
pub fn check_network(
    network_identifier: NetworkIdentifier,
    server_context: &RosettaContext,
) -> ApiResult<()> {
    if network_identifier.blockchain == BLOCKCHAIN
        || ChainId::from_str(network_identifier.network.trim()).map_err(|_| ApiError::BadNetwork)?
            == server_context.chain_id
    {
        Ok(())
    } else {
        Err(ApiError::BadNetwork)
    }
}

/// Attaches RosettaContext to warp paths
pub fn with_context(
    context: RosettaContext,
) -> impl Filter<Extract = (RosettaContext,), Error = Infallible> + Clone {
    warp::any().map(move || context.clone())
}

#[derive(Deserialize, Serialize)]
pub struct EmptyRequest;

pub fn with_empty_request() -> impl Filter<Extract = (EmptyRequest,), Error = Infallible> + Clone {
    warp::any().map(move || EmptyRequest)
}

/// Handles a generic request to warp
pub fn handle_request<'a, F, R, Req, Resp>(
    handler: F,
) -> impl Fn(
    Req,
    RosettaContext,
) -> BoxFuture<'static, Result<warp::reply::WithStatus<warp::reply::Json>, Infallible>>
       + Clone
where
    F: FnOnce(Req, RosettaContext) -> R + Clone + Copy + Send + 'static,
    R: Future<Output = Result<Resp, ApiError>> + Send,
    Req: Deserialize<'a> + Send + 'static,
    Resp: std::fmt::Debug + Serialize,
{
    move |request, options| {
        let fut = async move {
            match handler(request, options).await {
                Ok(response) => {
                    debug!(
                        "Response: {}",
                        serde_json::to_string_pretty(&response).unwrap()
                    );
                    Ok(warp::reply::with_status(
                        warp::reply::json(&response),
                        warp::http::StatusCode::OK,
                    ))
                }
                Err(api_error) => {
                    debug!("Error: {:?}", api_error);
                    let status = api_error.status_code();
                    Ok(warp::reply::with_status(
                        warp::reply::json(&api_error.into_error()),
                        status,
                    ))
                }
            }
        };
        Box::pin(fut)
    }
}

pub async fn get_account(
    rest_client: &aptos_rest_client::Client,
    address: AccountAddress,
) -> ApiResult<Response<Account>> {
    rest_client
        .get_account(address)
        .await
        .map_err(|_| ApiError::AccountNotFound)
}

pub async fn get_account_balance(
    rest_client: &aptos_rest_client::Client,
    address: AccountAddress,
) -> ApiResult<Response<Balance>> {
    rest_client
        .get_account_balance(address)
        .await
        .map_err(|_| ApiError::AccountNotFound)
}

pub async fn get_genesis_transaction(
    rest_client: &aptos_rest_client::Client,
) -> ApiResult<Response<Transaction>> {
    Ok(rest_client.get_transaction_by_version(0).await?)
}

/// Retrieve the timestamp according ot the Rosetta spec (milliseconds)
pub fn get_timestamp<T>(response: &Response<T>) -> u64 {
    // note: timestamps are in microseconds, so we convert to milliseconds
    response.state().timestamp_usecs / 1000
}

/// Strips the `0x` prefix on hex strings
pub fn strip_hex_prefix(str: &str) -> &str {
    str.strip_prefix("0x").unwrap_or(str)
}

pub fn encode_bcs<T: Serialize>(obj: &T) -> ApiResult<String> {
    let bytes = bcs::to_bytes(obj)?;
    Ok(hex::encode(bytes))
}

pub fn decode_bcs<T: DeserializeOwned>(str: &str, type_name: &'static str) -> ApiResult<T> {
    let bytes = hex::decode(str)?;
    bcs::from_bytes(&bytes).map_err(|_| ApiError::deserialization_failed(type_name))
}

pub fn decode_key<T: DeserializeOwned + ValidCryptoMaterial>(
    str: &str,
    type_name: &'static str,
) -> ApiResult<T> {
    hex::decode(str)?
        .as_slice()
        .try_into()
        .map_err(|_| ApiError::deserialization_failed(type_name))
}
