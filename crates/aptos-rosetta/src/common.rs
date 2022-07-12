// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::{ApiError, ApiResult},
    types::{
        test_coin_identifier, Currency, CurrencyMetadata, MetadataRequest, NetworkIdentifier,
        PartialBlockIdentifier,
    },
    RosettaContext,
};
use aptos_crypto::{ValidCryptoMaterial, ValidCryptoMaterialStringExt};
use aptos_logger::debug;
use aptos_rest_client::{aptos_api_types::BlockInfo, Account, Response};
use aptos_sdk::move_types::language_storage::{StructTag, TypeTag};
use aptos_types::{account_address::AccountAddress, chain_id::ChainId};
use futures::future::BoxFuture;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{convert::Infallible, fmt::LowerHex, future::Future, str::FromStr};
use warp::Filter;

/// The year 2000 in seconds, as this is the lower limit for Rosetta API implementations
const Y2K_SECS: u64 = 946713600000;
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
pub fn get_timestamp(block_info: BlockInfo) -> u64 {
    // note: timestamps are in microseconds, so we convert to milliseconds
    let mut timestamp = block_info.block_timestamp / 1000;

    // Rosetta doesn't like timestamps before 2000
    if timestamp < Y2K_SECS {
        timestamp = Y2K_SECS;
    }
    timestamp
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
        metadata: Some(CurrencyMetadata {
            move_type: native_coin_tag().to_string(),
        }),
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

/// Determines which block to pull for the request
pub async fn get_block_index_from_request(
    server_context: &RosettaContext,
    partial_block_identifier: Option<PartialBlockIdentifier>,
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
            server_context
                .block_cache()?
                .get_block_index_by_hash(&aptos_rest_client::aptos_api_types::HashValue::from_str(
                    &hash,
                )?)
                .await?
        }
        // Lookup latest version
        _ => {
            let response = server_context
                .rest_client()?
                .get_ledger_information()
                .await?;
            let state = response.state();

            server_context
                .block_cache()?
                .get_block_index_by_version(state.version)
                .await?
        }
    })
}

pub fn to_hex_lower<T: LowerHex>(obj: &T) -> String {
    format!("{:x}", obj)
}
