// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Rosetta Account API
//!
//! See: [Account API Spec](https://www.rosetta-api.org/docs/AccountApi.html)
//!

use crate::{
    block::{block_index_to_version, version_to_block_index},
    common::{check_network, handle_request, with_context},
    error::{ApiError, ApiResult},
    types::{
        AccountBalanceRequest, AccountBalanceResponse, Amount, BlockIdentifier, Currency,
        PartialBlockIdentifier,
    },
    RosettaContext,
};
use aptos_crypto::HashValue;
use aptos_logger::{debug, trace};
use aptos_rest_client::{aptos::Balance, aptos_api_types::U64};
use aptos_sdk::move_types::{identifier::Identifier, language_storage::TypeTag};
use aptos_types::account_address::AccountAddress;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr, sync::RwLock};
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
    let block_size = server_context.block_size;
    let rest_client = server_context.rest_client()?;

    // Retrieve the block index to read
    let block_index =
        get_block_index_from_request(rest_client, request.block_identifier.clone(), block_size)
            .await?;

    // Version to grab is the last entry in the block (balance is at end of block)
    let block_version = block_index_to_version(block_size, block_index);
    let balance_version = block_index_to_version(block_size, block_index + 1) - 1;

    let balances = get_balances(
        rest_client,
        request.account_identifier.account_address()?,
        balance_version,
    )
    .await?;

    let mut amounts = vec![];

    // Lookup coins, and fill in currency codes
    for (coin, balance) in balances {
        if let Some(currency) = server_context
            .coin_cache
            .get_currency(rest_client, coin, balance_version)
            .await?
        {
            amounts.push(Amount {
                value: balance.coin.value.0.to_string(),
                currency,
            });
        }
    }

    // Get the block identifier
    let response = rest_client
        .get_transaction_by_version(block_version)
        .await?;
    let block_identifier = BlockIdentifier::from_transaction(block_size, response.inner())?;

    Ok(AccountBalanceResponse {
        block_identifier,
        balances: amounts,
    })
}

async fn get_block_index_from_request(
    rest_client: &aptos_rest_client::Client,
    partial_block_identifier: Option<PartialBlockIdentifier>,
    block_size: u64,
) -> ApiResult<u64> {
    Ok(match partial_block_identifier {
        Some(PartialBlockIdentifier {
            index: Some(_),
            hash: Some(_),
        }) => {
            return Err(ApiError::HistoricBalancesUnsupported);
        }
        Some(PartialBlockIdentifier {
            index: Some(block_index),
            hash: None,
        }) => block_index,
        Some(PartialBlockIdentifier {
            index: None,
            hash: Some(hash),
        }) => {
            if hash == BlockIdentifier::genesis_txn().hash {
                0
            } else {
                // Lookup by hash doesn't work since we're faking blocks, need to verify that it's a
                // block
                let response = rest_client
                    .get_transaction(
                        HashValue::from_str(&hash)
                            .map_err(|err| ApiError::DeserializationFailed(err.to_string()))?,
                    )
                    .await?;
                let version = response.inner().version();

                if let Some(version) = version {
                    let block_index = version_to_block_index(block_size, version);
                    // If it's not the beginning of a block, then it's invalid
                    if version != block_index_to_version(block_size, block_index) {
                        return Err(ApiError::BadBlockRequest);
                    }

                    block_index
                } else {
                    // If the transaction is pending, it's incomplete
                    return Err(ApiError::BlockIncomplete);
                }
            }
        }
        _ => {
            let response = rest_client.get_ledger_information().await?;
            let state = response.state();
            block_index_to_version(
                block_size,
                version_to_block_index(block_size, state.version) - 1,
            )
        }
    })
}

async fn get_balances(
    rest_client: &aptos_rest_client::Client,
    address: AccountAddress,
    version: u64,
) -> ApiResult<HashMap<TypeTag, Balance>> {
    let response = rest_client
        .get_account_resources_at_version(address, version)
        .await?;
    let resources = response.inner();
    let coin_identifier = Identifier::new("Coin").unwrap();
    let coinstore_identifier = Identifier::new("CoinStore").unwrap();

    // Retrieve balances
    Ok(resources
        .iter()
        .filter(|resource| {
            resource.resource_type.address == AccountAddress::ONE
                && resource.resource_type.module == coin_identifier
                && resource.resource_type.name == coinstore_identifier
        })
        .filter_map(|resource| {
            // Coin must have a type
            let coin = resource.resource_type.type_params.first().unwrap();
            let resource = serde_json::from_value::<Balance>(resource.data.clone());
            match resource {
                Ok(resource) => Some((coin.clone(), resource)),
                Err(_) => None,
            }
        })
        .collect())
}

#[derive(Debug)]
pub struct CoinCache {
    currencies: RwLock<HashMap<TypeTag, Option<Currency>>>,
}

impl CoinCache {
    pub fn new() -> Self {
        Self {
            currencies: RwLock::new(HashMap::new()),
        }
    }

    /// Retrieve a currency and cache it if applicable
    pub async fn get_currency(
        &self,
        rest_client: &aptos_rest_client::Client,
        coin: TypeTag,
        version: u64,
    ) -> ApiResult<Option<Currency>> {
        {
            let currencies = self.currencies.read().unwrap();
            if let Some(currency) = currencies.get(&coin) {
                return Ok(currency.clone());
            }
        }

        let currency = self
            .get_currency_inner(rest_client, coin.clone(), version)
            .await?;
        self.currencies
            .write()
            .unwrap()
            .insert(coin, currency.clone());
        Ok(currency)
    }

    /// Pulls currency information from onchain
    pub async fn get_currency_inner(
        &self,
        rest_client: &aptos_rest_client::Client,
        coin: TypeTag,
        version: u64,
    ) -> ApiResult<Option<Currency>> {
        /// Type for deserializing coin info
        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct CoinInfo {
            name: String,
            symbol: String,
            decimals: U64,
        }

        let struct_tag = match coin {
            TypeTag::Struct(tag) => tag,
            // This is a poorly formed coin, and we'll just skip over it
            _ => return Ok(None),
        };

        // Nested types are not supported for now
        if !struct_tag.type_params.is_empty() {
            return Ok(None);
        }

        // Retrieve the coin type
        const ENCODE_CHARS: &AsciiSet = &CONTROLS.add(b'<').add(b'>');
        let address = struct_tag.address;
        let resource_tag = format!("0x1::Coin::CoinInfo<{}>", struct_tag);
        let encoded_resource_tag = utf8_percent_encode(&resource_tag, ENCODE_CHARS).to_string();
        println!("Get: {}", encoded_resource_tag);
        let response = rest_client
            .get_account_resource_at_version(address, &encoded_resource_tag, version)
            .await?;

        // At this point if we've retrieved it and it's bad, we error out
        if let Some(resource) = response.into_inner() {
            let coin_info =
                serde_json::from_value::<CoinInfo>(resource.data).map_err(|_| ApiError::BadCoin)?;

            Ok(Some(Currency {
                symbol: coin_info.symbol,
                decimals: coin_info.decimals.0,
            }))
        } else {
            Err(ApiError::BadCoin)
        }
    }
}
