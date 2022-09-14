// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::types::{APTOS_COIN_MODULE, APTOS_COIN_RESOURCE};
use crate::{
    error::{ApiError, ApiResult},
    types::{
        Currency, CurrencyMetadata, MetadataRequest, NetworkIdentifier, PartialBlockIdentifier,
    },
    RosettaContext,
};
use aptos_crypto::{ValidCryptoMaterial, ValidCryptoMaterialStringExt};
use aptos_logger::debug;
use aptos_rest_client::{Account, Response};
use aptos_sdk::move_types::ident_str;
use aptos_sdk::move_types::language_storage::{StructTag, TypeTag};
use aptos_types::{account_address::AccountAddress, chain_id::ChainId};
use futures::future::BoxFuture;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{convert::Infallible, fmt::LowerHex, future::Future, str::FromStr};
use warp::Filter;

/// The year 2000 in milliseconds, as this is the lower limit for Rosetta API implementations
pub const Y2K_MS: u64 = 946713600000;
pub const BLOCKCHAIN: &str = "aptos";

/// Checks the request network matches the server network
pub fn check_network(
    network_identifier: NetworkIdentifier,
    server_context: &RosettaContext,
) -> ApiResult<()> {
    if network_identifier.blockchain == BLOCKCHAIN
        && ChainId::from_str(network_identifier.network.trim())
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
                    debug!("Response: {:?}", serde_json::to_string_pretty(&response));
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
pub fn get_timestamp(timestamp_usecs: u64) -> u64 {
    // note: timestamps are in microseconds, so we convert to milliseconds
    let mut timestamp = timestamp_usecs / 1000;

    // Rosetta doesn't like timestamps before 2000
    if timestamp < Y2K_MS {
        timestamp = Y2K_MS;
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

const DEFAULT_COIN: &str = "APT";
const DEFAULT_DECIMALS: u8 = 8;

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
        module: ident_str!(APTOS_COIN_MODULE).into(),
        name: ident_str!(APTOS_COIN_RESOURCE).into(),
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
            index: Some(block_index),
            hash: Some(_),
        }) => block_index,
        // Lookup by block index
        Some(PartialBlockIdentifier {
            index: Some(block_index),
            hash: None,
        }) => block_index,
        // Lookup by block hash
        Some(PartialBlockIdentifier {
            index: None,
            hash: Some(hash),
        }) => BlockHash::from_str(&hash)?.block_height(server_context.chain_id)?,
        // Lookup latest version
        _ => {
            let response = server_context
                .rest_client()?
                .get_ledger_information()
                .await?;
            let state = response.state();

            state.block_height
        }
    })
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct BlockHash {
    chain_id: ChainId,
    block_height: u64,
}

impl BlockHash {
    pub fn new(chain_id: ChainId, block_height: u64) -> Self {
        BlockHash {
            chain_id,
            block_height,
        }
    }

    pub fn block_height(&self, expected_chain_id: ChainId) -> ApiResult<u64> {
        if expected_chain_id != self.chain_id {
            Err(ApiError::InvalidInput(Some(format!(
                "Invalid chain id in block hash {} expected {}",
                self.chain_id, expected_chain_id
            ))))
        } else {
            Ok(self.block_height)
        }
    }
}

impl FromStr for BlockHash {
    type Err = ApiError;

    fn from_str(str: &str) -> Result<Self, Self::Err> {
        let mut iter = str.split('-');

        let chain_id = if let Some(maybe_chain_id) = iter.next() {
            ChainId::from_str(maybe_chain_id).map_err(|_| {
                ApiError::InvalidInput(Some(format!(
                    "Invalid block hash, chain-id is invalid {}",
                    str
                )))
            })?
        } else {
            return Err(ApiError::InvalidInput(Some(format!(
                "Invalid block hash, missing chain-id or block height {}",
                str
            ))));
        };

        let block_height = if let Some(maybe_block_height) = iter.next() {
            u64::from_str(maybe_block_height).map_err(|_| {
                ApiError::InvalidInput(Some(format!(
                    "Invalid block hash, block height is invalid {}",
                    str
                )))
            })?
        } else {
            return Err(ApiError::InvalidInput(Some(format!(
                "Invalid block hash, missing block height {}",
                str
            ))));
        };

        if iter.next().is_some() {
            Err(ApiError::InvalidInput(Some(format!(
                "Invalid block hash, too many hyphens {}",
                str
            ))))
        } else {
            Ok(BlockHash::new(chain_id, block_height))
        }
    }
}

impl std::fmt::Display for BlockHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.chain_id, self.block_height)
    }
}

pub fn to_hex_lower<T: LowerHex>(obj: &T) -> String {
    format!("{:x}", obj)
}

/// Retrieves the currency from the given parameters
/// TODO: What do do about the type params?
pub fn parse_currency(address: AccountAddress, module: &str, name: &str) -> ApiResult<Currency> {
    match (address, module, name) {
        (AccountAddress::ONE, APTOS_COIN_MODULE, APTOS_COIN_RESOURCE) => Ok(native_coin()),
        _ => Err(ApiError::TransactionParseError(Some(format!(
            "Invalid coin for transfer {}::{}::{}",
            address, module, name
        )))),
    }
}

#[cfg(test)]
mod test {
    use crate::common::BlockHash;
    use aptos_types::chain_id::{ChainId, NamedChain};
    use std::str::FromStr;

    #[test]
    pub fn chain_id_height_check() {
        let block_hash = BlockHash::new(ChainId::test(), 0);
        block_hash
            .block_height(ChainId::test())
            .expect("Matching chain id should work");
        block_hash
            .block_height(ChainId::new(NamedChain::MAINNET.id()))
            .expect_err("Mismatch chain id should not work");
    }

    #[test]
    pub fn chain_id_string_check() {
        let block_hash = BlockHash::new(ChainId::test(), 0);
        let parsed_block_hash =
            BlockHash::from_str(&block_hash.to_string()).expect("Should parse string");
        assert_eq!(block_hash, parsed_block_hash);
    }

    #[test]
    pub fn valid_block_hashes() {
        let valid_block_hashes: Vec<(&str, ChainId, u64)> = vec![
            ("testnet-0", ChainId::new(NamedChain::TESTNET.id()), 0),
            ("mainnet-20", ChainId::new(NamedChain::MAINNET.id()), 20),
            ("5-2", ChainId::new(5), 2),
        ];
        for (str, chain_id, height) in valid_block_hashes {
            let block_hash = BlockHash::from_str(str).expect("Valid block hash");
            assert_eq!(block_hash.block_height, height);
            assert_eq!(block_hash.chain_id, chain_id);
        }
    }

    #[test]
    pub fn invalid_block_hashes() {
        let invalid_block_hashes: Vec<&str> =
            vec!["testnet--1", "testnet", "1", "testnet-1-1", "1-mainnet"];
        for str in invalid_block_hashes {
            BlockHash::from_str(str).expect_err("Invalid block hash");
        }
    }
}
