// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Rosetta Account API
//!
//! See: [Account API Spec](https://www.rosetta-api.org/docs/AccountApi.html)
//!

use crate::types::{
    AccountBalanceMetadata, ACCOUNT_MODULE, ACCOUNT_RESOURCE, COIN_MODULE, COIN_STORE_RESOURCE,
};
use crate::{
    common::{
        check_network, get_block_index_from_request, handle_request, native_coin, native_coin_tag,
        with_context,
    },
    error::{ApiError, ApiResult},
    types::{AccountBalanceRequest, AccountBalanceResponse, Amount, Currency, CurrencyMetadata},
    RosettaContext,
};
use aptos_logger::{debug, trace};
use aptos_sdk::move_types::language_storage::TypeTag;
use aptos_types::account_address::AccountAddress;
use aptos_types::account_config::{AccountResource, CoinInfoResource, CoinStoreResource};
use once_cell::sync::Lazy;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use std::collections::BTreeMap;
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
    let rest_client = server_context.rest_client()?;

    // Retrieve the block index to read
    let block_height =
        get_block_index_from_request(&server_context, request.block_identifier.clone()).await?;

    // Version to grab is the last entry in the block (balance is at end of block)
    let block_info = server_context
        .block_cache()?
        .get_block_info_by_height(block_height, server_context.chain_id)
        .await?;
    let balance_version = block_info.last_version;

    let (sequence_number, balances) = get_balances(
        &rest_client,
        request.account_identifier.account_address()?,
        balance_version,
    )
    .await?;

    let amounts = convert_balances_to_amounts(
        server_context.coin_cache.clone(),
        request.currencies,
        balances,
        balance_version,
    )
    .await?;

    Ok(AccountBalanceResponse {
        block_identifier: block_info.block_id,
        balances: amounts,
        metadata: AccountBalanceMetadata {
            sequence_number: sequence_number.into(),
        },
    })
}

/// Lookup currencies and convert them to Rosetta types
async fn convert_balances_to_amounts(
    coin_cache: Arc<CoinCache>,
    maybe_filter_currencies: Option<Vec<Currency>>,
    balances: HashMap<TypeTag, u64>,
    balance_version: u64,
) -> ApiResult<Vec<Amount>> {
    let mut amounts = Vec::new();

    // Lookup coins, and fill in currency codes
    for (coin, balance) in balances {
        if let Some(currency) = coin_cache.get_currency(coin, Some(balance_version)).await? {
            amounts.push(Amount {
                value: balance.to_string(),
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
) -> ApiResult<(u64, HashMap<TypeTag, u64>)> {
    if let Ok(response) = rest_client
        .get_account_resources_at_version_bcs(address, version)
        .await
    {
        let resources = response.into_inner();
        let mut maybe_sequence_number = None;
        let mut balances = HashMap::new();

        for (struct_tag, bytes) in resources {
            match (
                struct_tag.address,
                struct_tag.module.as_str(),
                struct_tag.name.as_str(),
            ) {
                (AccountAddress::ONE, ACCOUNT_MODULE, ACCOUNT_RESOURCE) => {
                    let account: AccountResource = bcs::from_bytes(&bytes)?;
                    maybe_sequence_number = Some(account.sequence_number())
                }
                (AccountAddress::ONE, COIN_MODULE, COIN_STORE_RESOURCE) => {
                    let coin_store: CoinStoreResource = bcs::from_bytes(&bytes)?;
                    if let Some(coin_type) = struct_tag.type_params.first() {
                        balances.insert(coin_type.clone(), coin_store.coin());
                    }
                }
                _ => {}
            }
        }

        let sequence_number = if let Some(sequence_number) = maybe_sequence_number {
            sequence_number
        } else {
            return Err(ApiError::InternalError(Some(
                "Failed to retrieve account sequence number".to_string(),
            )));
        };

        // Retrieve balances
        Ok((sequence_number, balances))
    } else {
        let mut currency_map = HashMap::new();
        currency_map.insert(native_coin_tag(), 0);
        Ok((0, currency_map))
    }
}

/// Only coins supported by Rosetta
///
/// TODO: Allow passing in known supported coins in some sort of config file
pub static SUPPORTED_COINS: Lazy<Vec<TypeTag>> = Lazy::new(|| vec![native_coin_tag()]);

/// A cache for currencies, so we don't have to keep looking up the status of it
#[derive(Debug)]
pub struct CoinCache {
    rest_client: Option<Arc<aptos_rest_client::Client>>,
    currencies: RwLock<BTreeMap<TypeTag, Option<Currency>>>,
}

impl CoinCache {
    pub fn new(rest_client: Option<Arc<aptos_rest_client::Client>>) -> Self {
        Self {
            rest_client,
            currencies: RwLock::new(BTreeMap::new()),
        }
    }

    pub fn get_currency_from_cache(&self, coin: &TypeTag) -> Option<Currency> {
        // Short circuit for the default coin
        if coin == &native_coin_tag() {
            return Some(native_coin());
        }

        let currencies = self
            .currencies
            .read()
            .expect("Can't recover from poisoned lock on coin cache");

        if let Some(currency) = currencies.get(coin) {
            currency.clone()
        } else {
            None
        }
    }

    /// Retrieve a currency and cache it if applicable
    pub async fn get_currency(
        &self,
        coin: TypeTag,
        version: Option<u64>,
    ) -> ApiResult<Option<Currency>> {
        // Short circuit for the default coin
        if coin == native_coin_tag() {
            return Ok(Some(native_coin()));
        }

        // Unsupported coins should be ignored, as symbol may be used as an identifier
        if !SUPPORTED_COINS.contains(&coin) {
            return Ok(None);
        }

        {
            let currencies = self
                .currencies
                .read()
                .expect("Can't recover from poisoned lock on coin cache");
            if let Some(currency) = currencies.get(&coin) {
                return Ok(currency.clone());
            }
        }

        let currency = self
            .get_currency_inner(coin.clone(), version)
            .await
            .map_err(|err| {
                ApiError::CoinTypeFailedToBeFetched(Some(format!(
                    "Failed to retrieve coin type {} {}",
                    coin, err
                )))
            })?;
        self.currencies
            .write()
            .expect("Can't recover from poisoned lock on coin cache")
            .insert(coin, currency.clone());
        Ok(currency)
    }

    /// Pulls currency information from onchain
    pub async fn get_currency_inner(
        &self,
        coin: TypeTag,
        version: Option<u64>,
    ) -> ApiResult<Option<Currency>> {
        let struct_tag = match coin {
            TypeTag::Struct(ref tag) => tag,
            // This is a poorly formed coin, and we'll just skip over it
            _ => return Ok(None),
        };

        // Retrieve the coin type
        const ENCODE_CHARS: &AsciiSet = &CONTROLS.add(b'<').add(b'>');
        let address = struct_tag.address;
        let resource_tag = format!("0x1::coin::CoinInfo<{}>", struct_tag);
        let encoded_resource_tag = utf8_percent_encode(&resource_tag, ENCODE_CHARS).to_string();

        let response = if let Some(version) = version {
            self.rest_client
                .as_ref()
                .expect("Should have rest client to check coin cache")
                .get_account_resource_at_version_bcs::<CoinInfoResource>(
                    address,
                    &encoded_resource_tag,
                    version,
                )
                .await
        } else {
            self.rest_client
                .as_ref()
                .expect("Should have rest client to check coin cache")
                .get_account_resource_bcs::<CoinInfoResource>(address, &encoded_resource_tag)
                .await
        };

        // At this point if we've retrieved it and it's bad, we error out
        let coin_info = response?.into_inner();

        // TODO: The symbol has to come from a trusted list, as the move type is the real indicator
        Ok(Some(Currency {
            symbol: coin_info
                .symbol()
                .unwrap_or_else(|err| format!("Invalid Symbol: {}", err)),
            decimals: coin_info.decimals(),
            metadata: Some(CurrencyMetadata {
                move_type: struct_tag.to_string(),
            }),
        }))
    }
}
