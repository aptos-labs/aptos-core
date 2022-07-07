// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block::{block_index_to_version, version_to_block_index},
    error::{ApiError, ApiResult},
    types::{
        test_coin_identifier, BlockIdentifier, Currency, MetadataRequest, NetworkIdentifier,
        PartialBlockIdentifier,
    },
    RosettaContext,
};
use aptos_crypto::{HashValue, ValidCryptoMaterial, ValidCryptoMaterialStringExt};
use aptos_logger::debug;
use aptos_rest_client::{Account, Response};
use aptos_sdk::move_types::language_storage::{StructTag, TypeTag};
use aptos_types::{account_address::AccountAddress, chain_id::ChainId};
use futures::future::BoxFuture;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{convert::Infallible, future::Future, str::FromStr};
use warp::Filter;

pub const BLOCKCHAIN: &str = "aptos";

/// Checks the request network matches the server network
pub fn check_network(
    network_identifier: NetworkIdentifier,
    server_context: &RosettaContext,
) -> ApiResult<()> {
    if network_identifier.blockchain == BLOCKCHAIN
        || ChainId::from_str(network_identifier.network.trim())
            .map_err(|_| ApiError::NetworkIdentifierMismatch)?
            == server_context.chain_id
    {
        Ok(())
    } else {
        Err(ApiError::NetworkIdentifierMismatch)
    }
}

/// Attaches RosettaContext to warp paths
pub fn with_context(
    context: RosettaContext,
) -> impl Filter<Extract = (RosettaContext,), Error = Infallible> + Clone {
    warp::any().map(move || context.clone())
}

pub fn with_empty_request() -> impl Filter<Extract = (MetadataRequest,), Error = Infallible> + Clone
{
    warp::any().map(move || MetadataRequest {})
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
        .map_err(|_| ApiError::AccountNotFound(Some(address.to_string())))
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
    T::from_encoded_string(str).map_err(|_| ApiError::deserialization_failed(type_name))
}

const DEFAULT_COIN: &str = "TC";
const DEFAULT_DECIMALS: u64 = 6;

pub fn native_coin() -> Currency {
    Currency {
        symbol: DEFAULT_COIN.to_string(),
        decimals: DEFAULT_DECIMALS,
    }
}

pub fn native_coin_tag() -> TypeTag {
    TypeTag::Struct(StructTag {
        address: AccountAddress::ONE,
        module: test_coin_identifier(),
        name: test_coin_identifier(),
        type_params: vec![],
    })
}

pub fn is_native_coin(currency: &Currency) -> ApiResult<()> {
    if currency == &native_coin() {
        Ok(())
    } else {
        Err(ApiError::UnsupportedCurrency(Some(currency.symbol.clone())))
    }
}

pub fn string_to_hash(str: &str) -> ApiResult<HashValue> {
    HashValue::from_str(strip_hex_prefix(str))
        .map_err(|err| ApiError::DeserializationFailed(Some(err.to_string())))
}

/// Determines which block to pull for the request
pub async fn get_block_index_from_request(
    rest_client: &aptos_rest_client::Client,
    partial_block_identifier: Option<PartialBlockIdentifier>,
    block_size: u64,
) -> ApiResult<u64> {
    Ok(match partial_block_identifier {
        Some(PartialBlockIdentifier {
            index: Some(_),
            hash: Some(_),
        }) => {
            return Err(ApiError::BlockParameterConflict);
        }
        // Lookup by block index
        Some(PartialBlockIdentifier {
            index: Some(block_index),
            hash: None,
        }) => block_index,
        // Lookup by block hash
        Some(PartialBlockIdentifier {
            index: None,
            hash: Some(hash),
        }) => {
            if hash == BlockIdentifier::genesis_txn().hash {
                0
            } else {
                // Lookup by hash doesn't work since we're faking blocks, need to verify that it's a
                // block
                let response = rest_client.get_transaction(string_to_hash(&hash)?).await?;
                let version = response.inner().version();

                if let Some(version) = version {
                    let block_index = version_to_block_index(block_size, version);
                    // If it's not the beginning of a block, then it's invalid
                    if version != block_index_to_version(block_size, block_index) {
                        return Err(ApiError::TransactionIsPending);
                    }

                    block_index
                } else {
                    // If the transaction is pending, it's incomplete
                    return Err(ApiError::BlockIncomplete);
                }
            }
        }
        // Lookup latest version
        _ => {
            let response = rest_client.get_ledger_information().await?;
            let state = response.state();
            // The current version won't be a full block, so we have to go one before it
            version_to_block_index(block_size, state.version) - 1
        }
    })
}
