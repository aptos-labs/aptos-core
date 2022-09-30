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
use aptos_logger::{debug, trace, warn};
use aptos_types::account_address::AccountAddress;
use aptos_types::account_config::{AccountResource, CoinStoreResource};
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

    let (sequence_number, operators, balances) = get_balances(
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
        },
    })
}

/// Retrieve the balances for an account
async fn get_balances(
    rest_client: &aptos_rest_client::Client,
    account: AccountIdentifier,
    version: u64,
    maybe_filter_currencies: Option<Vec<Currency>>,
) -> ApiResult<(u64, Option<Vec<AccountAddress>>, Vec<Amount>)> {
    let owner_address = account.account_address()?;

    // Retrieve all account resources
    if let Ok(response) = rest_client
        .get_account_resources_at_version_bcs(owner_address, version)
        .await
    {
        let resources = response.into_inner();
        let mut maybe_sequence_number = None;
        let mut maybe_operators = None;
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
                        maybe_operators = Some(vec![]);
                        for (operator, contract) in store.staking_contracts {
                            // Keep track of operators
                            maybe_operators.as_mut().unwrap().push(operator);
                            match get_total_stake(
                                rest_client,
                                &account,
                                contract.pool_address,
                                version,
                            )
                            .await
                            {
                                Ok(Some(balance)) => {
                                    total_stake = Some(
                                        total_stake.unwrap_or_default()
                                            + u64::from_str(&balance.value).unwrap_or_default(),
                                    );
                                }
                                result => {
                                    warn!(
                                        "Failed to retrieve stake for {}: {:?}",
                                        contract.pool_address, result
                                    )
                                }
                            }
                        }

                        if let Some(balance) = total_stake {
                            balances.push(Amount {
                                value: balance.to_string(),
                                currency: native_coin(),
                            })
                        }
                    } /* TODO: Right now operator stake is not supported
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
        Ok((sequence_number, maybe_operators, balances))
    } else {
        Ok((
            0,
            None,
            vec![Amount {
                value: 0.to_string(),
                currency: native_coin(),
            }],
        ))
    }
}
