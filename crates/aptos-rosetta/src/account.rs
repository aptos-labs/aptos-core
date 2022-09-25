// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Rosetta Account API
//!
//! See: [Account API Spec](https://www.rosetta-api.org/docs/AccountApi.html)
//!

use crate::types::*;
use crate::{
    common::{
        check_network, get_block_index_from_request, handle_request, native_coin, native_coin_tag,
        with_context,
    },
    error::{ApiError, ApiResult},
    types::{AccountBalanceRequest, AccountBalanceResponse, Amount, Currency},
    RosettaContext,
};
use aptos_logger::{debug, trace};
use aptos_types::account_address::AccountAddress;
use aptos_types::account_config::{AccountResource, CoinStoreResource};
use aptos_types::stake_pool::StakePool;
use std::collections::HashSet;
use std::str::FromStr;
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
        },
    })
}

/// Retrieve the balances for an account
async fn get_balances(
    rest_client: &aptos_rest_client::Client,
    account: AccountIdentifier,
    version: u64,
    maybe_filter_currencies: Option<Vec<Currency>>,
) -> ApiResult<(u64, Vec<Amount>)> {
    let owner_address = account.account_address()?;

    // Retrieve all account resources
    if let Ok(response) = rest_client
        .get_account_resources_at_version_bcs(owner_address, version)
        .await
    {
        let resources = response.into_inner();
        let mut maybe_sequence_number = None;
        let mut balances = vec![];

        // Iterate through resources, converting balances
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
                    // Only show coins on the base account
                    if account.is_base_account() {
                        let coin_store: CoinStoreResource = bcs::from_bytes(&bytes)?;
                        if let Some(coin_type) = struct_tag.type_params.first() {
                            // Only display supported coins
                            if coin_type == &native_coin_tag() {
                                balances.push(Amount {
                                    value: coin_store.coin().to_string(),
                                    currency: native_coin(),
                                });
                            }
                        }
                    }
                }
                (AccountAddress::ONE, STAKING_CONTRACT_MODULE, STORE_RESOURCE) => {
                    if account.is_base_account() {
                        continue;
                    }

                    let store: Store = bcs::from_bytes(&bytes)?;
                    if account.is_total_stake() {
                        // For total stake, collect all underlying staking contracts and combine
                        let mut total_stake: Option<u64> = None;
                        for (_operator, contract) in store.staking_contracts {
                            if let Ok(response) = rest_client
                                .get_account_resource_bcs::<StakePool>(
                                    contract.pool_address,
                                    &format!(
                                        "{}::{}::{}",
                                        AccountAddress::ONE.to_hex_literal(),
                                        STAKE_MODULE,
                                        STAKE_POOL_RESOURCE
                                    ),
                                )
                                .await
                            {
                                let stake_pool = response.into_inner();

                                // Any stake pools that match, retrieve that.  Then update the total
                                if let Ok(balance) =
                                    get_stake_balance_from_stake_pool(&stake_pool, &account)
                                {
                                    total_stake = Some(
                                        total_stake.unwrap_or_default()
                                            + u64::from_str(&balance.value).unwrap_or_default(),
                                    );
                                }
                            }
                        }

                        if let Some(balance) = total_stake {
                            balances.push(Amount {
                                value: balance.to_string(),
                                currency: native_coin(),
                            })
                        }
                    } else if account.is_operator_stake() {
                        // For operator stake, filter on operator address
                        let operator_address = account.operator_address()?;
                        if let Some(contract) = store.staking_contracts.get(&operator_address) {
                            if let Ok(response) = rest_client
                                .get_account_resource_bcs::<StakePool>(
                                    contract.pool_address,
                                    "0x1::stake::StakePool",
                                )
                                .await
                            {
                                let stake_pool = response.into_inner();

                                // Any stake pools that match, retrieve that.  Then update the total
                                if let Ok(balance) =
                                    get_stake_balance_from_stake_pool(&stake_pool, &account)
                                {
                                    balances.push(balance)
                                }
                            }
                        }
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
        Ok((sequence_number, balances))
    } else {
        Ok((
            0,
            vec![Amount {
                value: 0.to_string(),
                currency: native_coin(),
            }],
        ))
    }
}

/// Retrieves total stake balances from an individual stake pool
fn get_stake_balance_from_stake_pool(
    stake_pool: &StakePool,
    account: &AccountIdentifier,
) -> ApiResult<Amount> {
    // Stake isn't allowed for base accounts
    if account.is_base_account() {
        return Err(ApiError::InvalidInput(Some(
            "Stake pool not supported for base account".to_string(),
        )));
    }

    // If the operator address is different, skip
    if account.is_operator_stake() && account.operator_address()? != stake_pool.operator_address {
        return Err(ApiError::InvalidInput(Some(
            "Stake pool not for matching operator".to_string(),
        )));
    }

    // TODO: Represent inactive, and pending as separate?
    Ok(Amount {
        value: stake_pool.get_total_staked_amount().to_string(),
        currency: native_coin(),
    })
}
