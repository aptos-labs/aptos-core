// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::{ApiError, ApiResult},
    types::{
        Currency, CurrencyMetadata, MetadataRequest, NetworkIdentifier, PartialBlockIdentifier,
        VELOR_COIN_MODULE, VELOR_COIN_RESOURCE,
    },
    RosettaContext,
};
use velor_crypto::{ValidCryptoMaterial, ValidCryptoMaterialStringExt};
use velor_logger::debug;
use velor_rest_client::{Account, Response};
use velor_sdk::move_types::{
    ident_str,
    language_storage::{StructTag, TypeTag},
};
use velor_types::{account_address::AccountAddress, chain_id::ChainId};
use futures::future::BoxFuture;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashSet, convert::Infallible, fmt::LowerHex, future::Future, str::FromStr};
use warp::Filter;

/// The year 2000 in milliseconds, as this is the lower limit for Rosetta API implementations
pub const Y2K_MS: u64 = 946713600000;
pub const BLOCKCHAIN: &str = "velor";

/// Checks the request network matches the server network
///
/// These fields are passed in on every request, and basically prevents non-Velor and matching chain-id
/// requests from going through and messing things up.
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

/// Fills in an empty request for any REST API path that doesn't take any input body
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
                },
                Err(api_error) => {
                    debug!("Error: {:?}", api_error);
                    let status = api_error.status_code();
                    Ok(warp::reply::with_status(
                        warp::reply::json(&api_error.into_error()),
                        status,
                    ))
                },
            }
        };
        Box::pin(fut)
    }
}

/// Retrieves an account's information by its address
pub async fn get_account(
    rest_client: &velor_rest_client::Client,
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

/// Encodes the object into BCS, handling errors
pub fn encode_bcs<T: Serialize>(obj: &T) -> ApiResult<String> {
    let bytes = bcs::to_bytes(obj)?;
    Ok(hex::encode(bytes))
}

/// Decodes the object from BCS, handling errors
pub fn decode_bcs<T: DeserializeOwned>(str: &str, type_name: &'static str) -> ApiResult<T> {
    let bytes = hex::decode(str)?;
    bcs::from_bytes(&bytes).map_err(|_| ApiError::deserialization_failed(type_name))
}

/// Decodes a CryptoMaterial (key, signature, etc.) from Hex
/// TODO: Rename to decode_crypto_material
pub fn decode_key<T: DeserializeOwned + ValidCryptoMaterial>(
    str: &str,
    type_name: &'static str,
) -> ApiResult<T> {
    T::from_encoded_string(str).map_err(|_| ApiError::deserialization_failed(type_name))
}

const APT_SYMBOL: &str = "APT";
const APT_DECIMALS: u8 = 8;

/// Provides the [Currency] for 0x1::velor_coin::VelorCoin aka APT
///
/// Note that 0xA is the address for FA, but it has to be skipped in order to have backwards compatibility
pub fn native_coin() -> Currency {
    Currency {
        symbol: APT_SYMBOL.to_string(),
        decimals: APT_DECIMALS,
        metadata: Some(CurrencyMetadata {
            move_type: Some(native_coin_tag().to_canonical_string()),
            fa_address: None,
        }),
    }
}

/// Provides the [TypeTag] for 0x1::velor_coin::VelorCoin aka APT
pub fn native_coin_tag() -> TypeTag {
    TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ONE,
        module: ident_str!(VELOR_COIN_MODULE).into(),
        name: ident_str!(VELOR_COIN_RESOURCE).into(),
        type_args: vec![],
    }))
}

#[inline]
pub fn is_native_coin(fa_address: AccountAddress) -> bool {
    fa_address == AccountAddress::TEN
}

const USDC_SYMBOL: &str = "USDC";
const USDC_DECIMALS: u8 = 6;
const USDC_ADDRESS: &str = "0xbae207659db88bea0cbead6da0ed00aac12edcdda169e591cd41c94180b46f3b";
const USDC_TESTNET_ADDRESS: &str =
    "0x69091fbab5f7d635ee7ac5098cf0c1efbe31d68fec0f2cd565e8d168daf52832";
pub fn usdc_currency() -> Currency {
    Currency {
        symbol: USDC_SYMBOL.to_string(),
        decimals: USDC_DECIMALS,
        metadata: Some(CurrencyMetadata {
            move_type: None,
            fa_address: Some(USDC_ADDRESS.to_string()),
        }),
    }
}

pub fn usdc_testnet_currency() -> Currency {
    Currency {
        symbol: USDC_SYMBOL.to_string(),
        decimals: USDC_DECIMALS,
        metadata: Some(CurrencyMetadata {
            move_type: None,
            fa_address: Some(USDC_TESTNET_ADDRESS.to_string()),
        }),
    }
}

pub fn find_coin_currency(currencies: &HashSet<Currency>, type_tag: &TypeTag) -> Option<Currency> {
    currencies
        .iter()
        .find(|currency| {
            if let Some(CurrencyMetadata {
                move_type: Some(ref move_type),
                fa_address: _,
            }) = currency.metadata
            {
                move_type == &type_tag.to_canonical_string()
            } else {
                false
            }
        })
        .cloned()
}
pub fn find_fa_currency(
    currencies: &HashSet<Currency>,
    metadata_address: AccountAddress,
) -> Option<Currency> {
    if is_native_coin(metadata_address) {
        Some(native_coin())
    } else {
        let val = currencies
            .iter()
            .find(|currency| {
                if let Some(CurrencyMetadata {
                    move_type: _,
                    fa_address: Some(ref fa_address),
                }) = currency.metadata
                {
                    // TODO: Probably want to cache this
                    AccountAddress::from_str(fa_address)
                        .map(|addr| addr == metadata_address)
                        .unwrap_or(false)
                } else {
                    false
                }
            })
            .cloned();
        val
    }
}

/// Determines which block to pull for the request
///
/// Inputs can give hash, index, or both
pub async fn get_block_index_from_request(
    server_context: &RosettaContext,
    partial_block_identifier: Option<PartialBlockIdentifier>,
) -> ApiResult<u64> {
    Ok(match partial_block_identifier {
        // If Index and hash are provided, we use index, because it's easier to use.
        // Note, we don't handle if they mismatch.
        //
        // This is required.  Rosetta originally only took one or the other, and this failed in
        // integration testing.
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
        },
    })
}

/// BlockHash is not actually the block hash!  This was a hack put in, since we don't actually have
/// [BlockHash] indexable.  Instead, it just returns the combination of [ChainId] and the block_height (aka index).
///
/// The [BlockHash] string format is `chain_id-block_height`
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

    /// Fetch the block height
    ///
    /// We verify the chain_id to ensure it is the correct network
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

    /// Parses `chain_id-block_height`
    fn from_str(str: &str) -> Result<Self, Self::Err> {
        let mut iter = str.split('-');

        // It must start with a chain-id
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

        // Chain id must be followed after a `-` with block height
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

        // Don't allow any more hyphens or characters
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
pub fn parse_coin_currency(
    server_context: &RosettaContext,
    struct_tag: &StructTag,
) -> ApiResult<Currency> {
    if let Some(currency) = server_context.currencies.iter().find(|currency| {
        if let Some(move_type) = currency
            .metadata
            .as_ref()
            .and_then(|inner| inner.move_type.as_ref())
        {
            struct_tag.to_canonical_string() == *move_type
        } else {
            false
        }
    }) {
        Ok(currency.clone())
    } else {
        Err(ApiError::TransactionParseError(Some(format!(
            "Invalid coin for transfer {}",
            struct_tag.to_canonical_string()
        ))))
    }
}

#[cfg(test)]
mod test {
    use crate::common::BlockHash;
    use velor_types::chain_id::{ChainId, NamedChain};
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
