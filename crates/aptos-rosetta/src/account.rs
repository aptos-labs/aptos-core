// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Rosetta Account API
//!
//! See: [Account API Spec](https://docs.cloud.coinbase.com/rosetta/docs/models#accountbalanceresponse)
//!

use crate::{
    common::{
        check_network, get_block_index_from_request, handle_request, native_coin, native_coin_tag,
        with_context,
    },
    error::{ApiError, ApiResult},
    types::{AccountBalanceRequest, AccountBalanceResponse, Amount, Currency, *},
    RosettaContext,
};
use aptos_logger::{debug, trace, warn};
use aptos_types::{
    account_address::AccountAddress,
    account_config::{AccountResource, CoinStoreResourceUntyped},
};
use std::{collections::HashSet, str::FromStr};
use warp::Filter;

/// Account routes e.g. balance
pub fn routes(
    server_context: RosettaContext,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
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
    // NOTE: In Rosetta, we always do balances by block here rather than ledger version.
    let block_info = server_context
        .block_cache()?
        .get_block_info_by_height(block_height, server_context.chain_id)
        .await?;
    let balance_version = block_info.last_version;

    // Retrieve all metadata we want to provide as an on-demand lookup
    let (sequence_number, operators, balances, lockup_expiration) = get_balances(
        &rest_client,
        request.account_identifier,
        balance_version,
        request.currencies,
    )
    .await?;

    Ok(AccountBalanceResponse {
        block_identifier: block_info.block_id,
        balances,
        metadata: AccountBalanceMetadata {
            sequence_number: sequence_number.into(),
            operators,
            lockup_expiration_time_utc: aptos_rest_client::aptos_api_types::U64(lockup_expiration),
        },
    })
}

/// Retrieve the balances for an account
#[allow(clippy::manual_retain)]
async fn get_balances(
    rest_client: &aptos_rest_client::Client,
    account: AccountIdentifier,
    version: u64,
    maybe_filter_currencies: Option<Vec<Currency>>,
) -> ApiResult<(u64, Option<Vec<AccountAddress>>, Vec<Amount>, u64)> {
    let owner_address = account.account_address()?;
    let pool_address = account.pool_address()?;

    let mut balances = vec![];
    let mut lockup_expiration: u64 = 0;
    let mut total_requested_balance: Option<u64> = None;

    // Lookup the delegation pool, if it's provided in the account information
    if pool_address.is_some() {
        match get_delegation_stake_balances(
            rest_client,
            &account,
            owner_address,
            pool_address.unwrap(),
            version,
        )
        .await
        {
            Ok(Some(balance_result)) => {
                if let Some(balance) = balance_result.balance {
                    total_requested_balance = Some(
                        total_requested_balance.unwrap_or_default()
                            + u64::from_str(&balance.value).unwrap_or_default(),
                    );
                }
                lockup_expiration = balance_result.lockup_expiration;
                if let Some(balance) = total_requested_balance {
                    balances.push(Amount {
                        value: balance.to_string(),
                        currency: native_coin(),
                    })
                }
            },
            result => {
                warn!(
                    "Failed to retrieve requested balance for delegator_address: {}, pool_address: {}: {:?}",
                    owner_address, pool_address.unwrap(), result
                )
            },
        }
    }

    // Retrieve all account resources
    // TODO: This will need to change for FungibleAssets, will need to lookup on a list of known FAs
    if let Ok(response) = rest_client
        .get_account_resources_at_version_bcs(owner_address, version)
        .await
    {
        let resources = response.into_inner();
        let mut maybe_sequence_number = None;
        let mut maybe_operators = None;

        // Iterate through resources, converting balances
        for (struct_tag, bytes) in resources {
            match (
                struct_tag.address,
                struct_tag.module.as_str(),
                struct_tag.name.as_str(),
            ) {
                // Retrieve the sequence number from the account resource
                // TODO: Make a separate call for this
                (AccountAddress::ONE, ACCOUNT_MODULE, ACCOUNT_RESOURCE) => {
                    let account: AccountResource = bcs::from_bytes(&bytes)?;
                    maybe_sequence_number = Some(account.sequence_number())
                },
                // Parse all associated coin stores
                // TODO: This would need to be expanded to support other coin stores
                (AccountAddress::ONE, COIN_MODULE, COIN_STORE_RESOURCE) => {
                    // Only show coins on the base account
                    if account.is_base_account() {
                        let coin_store: CoinStoreResourceUntyped = bcs::from_bytes(&bytes)?;
                        if let Some(coin_type) = struct_tag.type_args.first() {
                            // Only display supported coins
                            if coin_type == &native_coin_tag() {
                                balances.push(Amount {
                                    value: coin_store.coin().to_string(),
                                    currency: native_coin(),
                                });
                            }
                        }
                    }
                },
                // Parse all staking contract data to know the underlying balances of the pools
                (AccountAddress::ONE, STAKING_CONTRACT_MODULE, STORE_RESOURCE) => {
                    if account.is_base_account() || pool_address.is_some() {
                        continue;
                    }

                    let store: Store = bcs::from_bytes(&bytes)?;
                    maybe_operators = Some(vec![]);
                    for (operator, contract) in store.staking_contracts {
                        // Keep track of operators
                        maybe_operators.as_mut().unwrap().push(operator);
                        match get_stake_balances(
                            rest_client,
                            &account,
                            contract.pool_address,
                            version,
                        )
                        .await
                        {
                            Ok(Some(balance_result)) => {
                                if let Some(balance) = balance_result.balance {
                                    total_requested_balance = Some(
                                        total_requested_balance.unwrap_or_default()
                                            + u64::from_str(&balance.value).unwrap_or_default(),
                                    );
                                }
                                lockup_expiration = balance_result.lockup_expiration;
                            },
                            result => {
                                warn!(
                                    "Failed to retrieve requested balance for account: {}, address: {}: {:?}",
                                    owner_address, contract.pool_address, result
                                )
                            },
                        }
                    }
                    if let Some(balance) = total_requested_balance {
                        balances.push(Amount {
                            value: balance.to_string(),
                            currency: native_coin(),
                        })
                    }

                    /* TODO: Right now operator stake is not supported
                    else if account.is_operator_stake() {
                        // For operator stake, filter on operator address
                        let operator_address = account.operator_address()?;
                        if let Some(contract) = store.staking_contracts.get(&operator_address) {
                            balances.push(get_total_stake(
                                rest_client,
                                &account,
                                contract.pool_address,
                                version,
                            ).await?);
                        }
                    }*/
                },
                _ => {},
            }
        }

        // Retrieves the sequence number accordingly
        // TODO: Sequence number should be 0 if it isn't retrieved probably
        let sequence_number = if let Some(sequence_number) = maybe_sequence_number {
            sequence_number
        } else {
            return Err(ApiError::InternalError(Some(
                "Failed to retrieve account sequence number".to_string(),
            )));
        };

        // Filter based on requested currencies
        if let Some(currencies) = maybe_filter_currencies {
            let mut currencies: HashSet<Currency> = currencies.into_iter().collect();
            // Remove extra currencies not requested
            balances = balances
                .into_iter()
                .filter(|balance| currencies.contains(&balance.currency))
                .collect();

            for balance in balances.iter() {
                currencies.remove(&balance.currency);
            }

            for currency in currencies {
                balances.push(Amount {
                    value: 0.to_string(),
                    currency,
                });
            }
        }

        // Retrieve balances
        Ok((
            sequence_number,
            maybe_operators,
            balances,
            lockup_expiration,
        ))
    } else {
        // If it fails, we return 0
        // TODO: This should probably be fixed to check if the account exists.  Then if the account doesn't exist, return empty balance, otherwise error
        Ok((
            0,
            None,
            vec![Amount {
                value: 0.to_string(),
                currency: native_coin(),
            }],
            0,
        ))
    }
}
