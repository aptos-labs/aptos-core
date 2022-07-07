// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Rosetta Account API
//!
//! See: [Account API Spec](https://www.rosetta-api.org/docs/AccountApi.html)
//!

use crate::{
    block::block_index_to_version,
    common::{check_network, get_block_index_from_request, handle_request, with_context},
    error::{ApiError, ApiResult},
    types::{
        coin_identifier, coin_store_identifier, AccountBalanceRequest, AccountBalanceResponse,
        Amount, BlockIdentifier, Currency,
    },
    RosettaContext,
};
use aptos_logger::{debug, trace};
use aptos_rest_client::{aptos::Balance, aptos_api_types::U64};
use aptos_sdk::move_types::language_storage::TypeTag;
use aptos_types::account_address::AccountAddress;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};
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
    let balance_version =
        block_index_to_version(block_size, block_index.saturating_add(1)).saturating_sub(1);

    let balances = get_balances(
        rest_client,
        request.account_identifier.account_address()?,
        balance_version,
    )
    .await?;

    let amounts = convert_balances_to_amounts(
        rest_client,
        server_context.coin_cache.clone(),
        request.currencies,
        balances,
        balance_version,
    )
    .await?;

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

/// Lookup currencies and convert them to Rosetta types
async fn convert_balances_to_amounts(
    rest_client: &aptos_rest_client::Client,
    coin_cache: Arc<CoinCache>,
    maybe_filter_currencies: Option<Vec<Currency>>,
    balances: HashMap<TypeTag, Balance>,
    balance_version: u64,
) -> ApiResult<Vec<Amount>> {
    let mut amounts = Vec::new();

    // Lookup coins, and fill in currency codes
    for (coin, balance) in balances {
        if let Some(currency) = coin_cache
            .get_currency(rest_client, coin, Some(balance_version))
            .await?
        {
            amounts.push(Amount {
                value: balance.coin.value.0.to_string(),
                currency,
            });
        }
    }

    // Filter based on requested currencies
    if let Some(currencies) = maybe_filter_currencies {
        let mut currencies: HashSet<Currency> = currencies.into_iter().collect();
        // Remove extra currencies not requested
        amounts = amounts
            .into_iter()
            .filter(|amount| currencies.contains(&amount.currency))
            .collect();

        // Zero out currencies that weren't in the account yet
        for amount in amounts.iter() {
            currencies.remove(&amount.currency);
        }
        for currency in currencies {
            amounts.push(Amount {
                value: 0.to_string(),
                currency,
            });
        }
    }

    Ok(amounts)
}

/// Retrieve the balances for an account
async fn get_balances(
    rest_client: &aptos_rest_client::Client,
    address: AccountAddress,
    version: u64,
) -> ApiResult<HashMap<TypeTag, Balance>> {
    // TODO: Handle non not found errors
    if let Ok(response) = rest_client
        .get_account_resources_at_version(address, version)
        .await
    {
        // Retrieve balances
        Ok(response
            .inner()
            .iter()
            .filter(|resource| {
                resource.resource_type.address == AccountAddress::ONE
                    && resource.resource_type.module == coin_identifier()
                    && resource.resource_type.name == coin_store_identifier()
            })
            .filter_map(|resource| {
                // Currency must have a type
                if let Some(coin_type) = resource.resource_type.type_params.first() {
                    match serde_json::from_value::<Balance>(resource.data.clone()) {
                        Ok(resource) => Some((coin_type.clone(), resource)),
                        Err(_) => None,
                    }
                } else {
                    // Skip currencies that don't match
                    None
                }
            })
            .collect())
    } else {
        Ok(HashMap::new())
    }
}

/// A cache for currencies, so we don't have to keep looking up the status of it
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
        version: Option<u64>,
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
        version: Option<u64>,
    ) -> ApiResult<Option<Currency>> {
        /// Type for deserializing coin info
        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct CoinInfo {
            name: String,
            symbol: String,
            decimals: U64,
        }

        let struct_tag = match coin {
            TypeTag::Struct(ref tag) => tag,
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

        let response = if let Some(version) = version {
            rest_client
                .get_account_resource_at_version(address, &encoded_resource_tag, version)
                .await?
        } else {
            rest_client
                .get_account_resource(address, &encoded_resource_tag)
                .await?
        };

        // At this point if we've retrieved it and it's bad, we error out
        if let Some(resource) = response.into_inner() {
            let coin_info = serde_json::from_value::<CoinInfo>(resource.data).map_err(|_| {
                ApiError::DeserializationFailed(Some(format!(
                    "CoinInfo failed to deserialize for {}",
                    coin
                )))
            })?;

            Ok(Some(Currency {
                symbol: coin_info.symbol,
                decimals: coin_info.decimals.0,
            }))
        } else {
            Err(ApiError::DeserializationFailed(Some(format!(
                "Currency {} not found",
                coin
            ))))
        }
    }
}
