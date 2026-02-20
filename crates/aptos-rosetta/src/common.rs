// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    error::{ApiError, ApiResult},
    types::{
        Currency, CurrencyMetadata, MetadataRequest, NetworkIdentifier, PartialBlockIdentifier,
        APTOS_COIN_MODULE, APTOS_COIN_RESOURCE,
    },
    RosettaContext,
};
use aptos_crypto::{ValidCryptoMaterial, ValidCryptoMaterialStringExt};
use aptos_logger::debug;
use aptos_rest_client::{Account, Response};
use aptos_sdk::move_types::{
    ident_str,
    language_storage::{StructTag, TypeTag},
};
use aptos_types::{account_address::AccountAddress, chain_id::ChainId};
use futures::future::BoxFuture;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashSet, convert::Infallible, fmt::LowerHex, future::Future, str::FromStr};
use warp::Filter;

/// The year 2000 in milliseconds, as this is the lower limit for Rosetta API implementations
pub const Y2K_MS: u64 = 946713600000;
pub const BLOCKCHAIN: &str = "aptos";

/// Checks the request network matches the server network
///
/// These fields are passed in on every request, and basically prevents non-Aptos and matching chain-id
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
       + use<F, R, Req, Resp>
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

/// Provides the [Currency] for 0x1::aptos_coin::AptosCoin aka APT
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

/// Provides the [TypeTag] for 0x1::aptos_coin::AptosCoin aka APT
pub fn native_coin_tag() -> TypeTag {
    TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ONE,
        module: ident_str!(APTOS_COIN_MODULE).into(),
        name: ident_str!(APTOS_COIN_RESOURCE).into(),
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
        currencies
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
            .cloned()
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
    use super::*;
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

    // ---- Timestamp Tests ----

    #[test]
    fn test_get_timestamp_normal() {
        // 1_700_000_000_000 ms = some time in 2023, way after Y2K
        let usecs = 1_700_000_000_000_000u64; // microseconds
        let result = get_timestamp(usecs);
        assert_eq!(result, 1_700_000_000_000u64);
    }

    #[test]
    fn test_get_timestamp_pre_y2k_clamped() {
        // timestamp 0 (genesis) should clamp to Y2K
        assert_eq!(get_timestamp(0), Y2K_MS);
        // A very early timestamp should also clamp
        assert_eq!(get_timestamp(1000), Y2K_MS);
    }

    #[test]
    fn test_get_timestamp_exactly_y2k() {
        // Exactly Y2K in microseconds
        let y2k_usecs = Y2K_MS * 1000;
        assert_eq!(get_timestamp(y2k_usecs), Y2K_MS);
    }

    #[test]
    fn test_get_timestamp_just_after_y2k() {
        let y2k_usecs = Y2K_MS * 1000 + 1000; // 1ms after Y2K
        assert_eq!(get_timestamp(y2k_usecs), Y2K_MS + 1);
    }

    // ---- strip_hex_prefix Tests ----

    #[test]
    fn test_strip_hex_prefix_with_prefix() {
        assert_eq!(strip_hex_prefix("0xabcdef"), "abcdef");
        assert_eq!(strip_hex_prefix("0x1"), "1");
        assert_eq!(strip_hex_prefix("0x"), "");
    }

    #[test]
    fn test_strip_hex_prefix_without_prefix() {
        assert_eq!(strip_hex_prefix("abcdef"), "abcdef");
        assert_eq!(strip_hex_prefix(""), "");
        assert_eq!(strip_hex_prefix("1234"), "1234");
    }

    // ---- BCS encode/decode Tests ----

    #[test]
    fn test_encode_decode_bcs_roundtrip() {
        let value: u64 = 42;
        let encoded = encode_bcs(&value).expect("Should encode");
        let decoded: u64 = decode_bcs(&encoded, "u64").expect("Should decode");
        assert_eq!(value, decoded);
    }

    #[test]
    fn test_decode_bcs_invalid() {
        let result = decode_bcs::<u64>("not_valid_hex", "u64");
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_bcs_complex_type() {
        let value = AccountAddress::ONE;
        let encoded = encode_bcs(&value).expect("Should encode address");
        let decoded: AccountAddress = decode_bcs(&encoded, "AccountAddress").expect("Should decode");
        assert_eq!(value, decoded);
    }

    // ---- Currency Tests ----

    #[test]
    fn test_native_coin() {
        let coin = native_coin();
        assert_eq!(coin.symbol, "APT");
        assert_eq!(coin.decimals, 8);
        assert!(coin.metadata.is_some());
        let metadata = coin.metadata.unwrap();
        assert!(metadata.move_type.is_some());
        assert!(metadata.fa_address.is_none());
        assert!(metadata.move_type.unwrap().contains("AptosCoin"));
    }

    #[test]
    fn test_native_coin_tag() {
        let tag = native_coin_tag();
        match tag {
            TypeTag::Struct(s) => {
                assert_eq!(s.address, AccountAddress::ONE);
                assert_eq!(s.module.as_str(), "aptos_coin");
                assert_eq!(s.name.as_str(), "AptosCoin");
            },
            _ => panic!("Expected struct tag"),
        }
    }

    #[test]
    fn test_is_native_coin() {
        assert!(is_native_coin(AccountAddress::TEN));
        assert!(!is_native_coin(AccountAddress::ONE));
        assert!(!is_native_coin(AccountAddress::ZERO));
    }

    #[test]
    fn test_usdc_currency() {
        let usdc = usdc_currency();
        assert_eq!(usdc.symbol, "USDC");
        assert_eq!(usdc.decimals, 6);
        let metadata = usdc.metadata.unwrap();
        assert!(metadata.move_type.is_none());
        assert!(metadata.fa_address.is_some());
    }

    #[test]
    fn test_usdc_testnet_currency() {
        let usdc = usdc_testnet_currency();
        assert_eq!(usdc.symbol, "USDC");
        assert_eq!(usdc.decimals, 6);
        let metadata = usdc.metadata.unwrap();
        assert!(metadata.move_type.is_none());
        assert!(metadata.fa_address.is_some());
        // Testnet address should differ from mainnet
        assert_ne!(
            usdc_testnet_currency().metadata.unwrap().fa_address,
            usdc_currency().metadata.unwrap().fa_address
        );
    }

    #[test]
    fn test_find_coin_currency_match() {
        let mut currencies = HashSet::new();
        currencies.insert(native_coin());
        let tag = native_coin_tag();
        let result = find_coin_currency(&currencies, &tag);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), native_coin());
    }

    #[test]
    fn test_find_coin_currency_no_match() {
        let currencies = HashSet::new();
        let tag = native_coin_tag();
        let result = find_coin_currency(&currencies, &tag);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_fa_currency_native() {
        let currencies = HashSet::new();
        let result = find_fa_currency(&currencies, AccountAddress::TEN);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), native_coin());
    }

    #[test]
    fn test_find_fa_currency_usdc() {
        let mut currencies = HashSet::new();
        currencies.insert(usdc_currency());
        let addr =
            AccountAddress::from_str(USDC_ADDRESS).expect("Valid address");
        let result = find_fa_currency(&currencies, addr);
        assert!(result.is_some());
        assert_eq!(result.unwrap().symbol, "USDC");
    }

    #[test]
    fn test_find_fa_currency_unknown() {
        let currencies = HashSet::new();
        let result = find_fa_currency(&currencies, AccountAddress::ONE);
        assert!(result.is_none());
    }

    // ---- check_network Tests ----

    #[tokio::test]
    async fn test_check_network_valid() {
        let context = RosettaContext::new(None, ChainId::test(), None, HashSet::new()).await;
        let network_id = NetworkIdentifier {
            blockchain: BLOCKCHAIN.to_string(),
            network: ChainId::test().to_string(),
        };
        assert!(check_network(network_id, &context).is_ok());
    }

    #[tokio::test]
    async fn test_check_network_wrong_blockchain() {
        let context = RosettaContext::new(None, ChainId::test(), None, HashSet::new()).await;
        let network_id = NetworkIdentifier {
            blockchain: "bitcoin".to_string(),
            network: ChainId::test().to_string(),
        };
        assert!(check_network(network_id, &context).is_err());
    }

    #[tokio::test]
    async fn test_check_network_wrong_chain_id() {
        let context = RosettaContext::new(None, ChainId::test(), None, HashSet::new()).await;
        let network_id = NetworkIdentifier {
            blockchain: BLOCKCHAIN.to_string(),
            network: "mainnet".to_string(),
        };
        assert!(check_network(network_id, &context).is_err());
    }

    #[tokio::test]
    async fn test_check_network_invalid_chain_id() {
        let context = RosettaContext::new(None, ChainId::test(), None, HashSet::new()).await;
        let network_id = NetworkIdentifier {
            blockchain: BLOCKCHAIN.to_string(),
            network: "not_a_chain".to_string(),
        };
        assert!(check_network(network_id, &context).is_err());
    }

    // ---- to_hex_lower Tests ----

    #[test]
    fn test_to_hex_lower() {
        let addr = AccountAddress::ONE;
        let hex = to_hex_lower(&addr);
        assert!(hex.contains('1'));
        assert!(!hex.contains('A')); // Should be lowercase
    }

    // ---- parse_coin_currency Tests ----

    #[tokio::test]
    async fn test_parse_coin_currency_valid() {
        let context = RosettaContext::new(None, ChainId::test(), None, HashSet::new()).await;
        let struct_tag = StructTag {
            address: AccountAddress::ONE,
            module: ident_str!("aptos_coin").into(),
            name: ident_str!("AptosCoin").into(),
            type_args: vec![],
        };
        let result = parse_coin_currency(&context, &struct_tag);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().symbol, "APT");
    }

    #[tokio::test]
    async fn test_parse_coin_currency_invalid() {
        let context = RosettaContext::new(None, ChainId::test(), None, HashSet::new()).await;
        let struct_tag = StructTag {
            address: AccountAddress::ONE,
            module: ident_str!("unknown_coin").into(),
            name: ident_str!("UnknownCoin").into(),
            type_args: vec![],
        };
        let result = parse_coin_currency(&context, &struct_tag);
        assert!(result.is_err());
    }

    // ---- BlockHash Display Tests ----

    #[test]
    fn test_block_hash_display() {
        let hash = BlockHash::new(ChainId::test(), 42);
        let display = hash.to_string();
        assert!(display.contains("42"));
    }

    #[test]
    fn test_block_hash_roundtrip_mainnet() {
        let hash = BlockHash::new(ChainId::new(NamedChain::MAINNET.id()), 999);
        let s = hash.to_string();
        let parsed = BlockHash::from_str(&s).expect("Should parse");
        assert_eq!(parsed, hash);
    }

    // ---- RosettaContext Tests ----

    #[tokio::test]
    async fn test_rosetta_context_always_includes_apt() {
        let context = RosettaContext::new(None, ChainId::test(), None, HashSet::new()).await;
        assert!(context.currencies.contains(&native_coin()));
    }

    #[tokio::test]
    async fn test_rosetta_context_mainnet_includes_usdc() {
        let mainnet_id = ChainId::new(NamedChain::MAINNET.id());
        let context = RosettaContext::new(None, mainnet_id, None, HashSet::new()).await;
        assert!(context.currencies.contains(&native_coin()));
        assert!(context.currencies.contains(&usdc_currency()));
    }

    #[tokio::test]
    async fn test_rosetta_context_testnet_includes_testnet_usdc() {
        let testnet_id = ChainId::new(NamedChain::TESTNET.id());
        let context = RosettaContext::new(None, testnet_id, None, HashSet::new()).await;
        assert!(context.currencies.contains(&native_coin()));
        assert!(context.currencies.contains(&usdc_testnet_currency()));
    }

    #[tokio::test]
    async fn test_rosetta_context_offline_rest_client() {
        let context = RosettaContext::new(None, ChainId::test(), None, HashSet::new()).await;
        assert!(context.rest_client().is_err());
    }

    #[tokio::test]
    async fn test_rosetta_context_offline_block_cache() {
        let context = RosettaContext::new(None, ChainId::test(), None, HashSet::new()).await;
        assert!(context.block_cache().is_err());
    }
}
