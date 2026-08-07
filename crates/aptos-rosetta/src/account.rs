// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Rosetta Account API
//!
//! See: [Account API Spec](https://docs.cloud.coinbase.com/rosetta/docs/models#accountbalanceresponse)
//!

use crate::{
    common::{check_network, get_block_index_from_request, native_coin, RosettaJson},
    error::{ApiError, ApiResult},
    types::{AccountBalanceRequest, AccountBalanceResponse, Amount, Currency, *},
    RosettaContext,
};
use aptos_logger::{debug, trace, warn};
use aptos_rest_client::{
    aptos_api_types::{AptosError, AptosErrorCode, ViewFunction},
    error::{AptosErrorResponse, RestError},
    Client,
};
use aptos_types::{
    account_address::AccountAddress,
    account_config::{fungible_store::FungibleStoreResource, AccountResource},
};
use axum::{extract::State, routing::post, Json, Router};
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    language_storage::{ModuleId, StructTag, TypeTag},
    parser::parse_type_tag,
};
use serde::de::DeserializeOwned;
use std::{collections::HashSet, str::FromStr};

/// Account routes e.g. balance
pub fn routes() -> Router<RosettaContext> {
    Router::new().route(
        "/account/balance",
        post(
            |State(server_context): State<RosettaContext>,
             RosettaJson(request): RosettaJson<AccountBalanceRequest>| async move {
                account_balance(request, server_context).await.map(Json)
            },
        ),
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
        &server_context,
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
    server_context: &RosettaContext,
    account: AccountIdentifier,
    version: u64,
    maybe_filter_currencies: Option<Vec<Currency>>,
) -> ApiResult<(u64, Option<Vec<AccountAddress>>, Vec<Amount>, u64)> {
    let rest_client = server_context.rest_client()?;
    let owner_address = account.account_address()?;
    let pool_address = account.pool_address()?;

    let mut balances = vec![];
    let mut lockup_expiration: u64 = 0;
    let mut maybe_operators = None;

    // Handle the things that must always happen

    // Retrieve the sequence number
    let sequence_number = get_sequence_number(&rest_client, owner_address, version).await?;

    // Filter currencies to lookup
    let currencies_to_lookup = if let Some(currencies) = maybe_filter_currencies {
        currencies.into_iter().collect()
    } else {
        server_context.currencies.clone()
    };

    // Regular account, FA and Coin
    if account.is_base_account() {
        balances =
            get_base_balances(&rest_client, owner_address, version, currencies_to_lookup).await?;
    } else if let Some(currency) = account.secondary_store_currency() {
        if currencies_to_lookup.contains(currency) {
            balances =
                get_secondary_store_balance(&rest_client, owner_address, version, currency).await?;
        }
    } else if let Some(pool_address) = pool_address {
        // Lookup the delegation pool, if it's provided in the account information
        // Filter appropriately, must have native coin
        if currencies_to_lookup.contains(&native_coin()) {
            (balances, lockup_expiration) =
                get_delegation_info(&rest_client, &account, owner_address, pool_address, version)
                    .await?;
        }
    } else {
        // Retrieve staking information (if it applies)
        // Only non-pool addresses, and non base accounts
        //
        // These are special cases around querying the stake amounts
        // Filter appropriately, must have native coin
        if currencies_to_lookup.contains(&native_coin()) {
            (balances, lockup_expiration, maybe_operators) =
                get_staking_info(&rest_client, &account, owner_address, version).await?;
        }
    }

    Ok((
        sequence_number,
        maybe_operators,
        balances,
        lockup_expiration,
    ))
}

async fn get_secondary_store_balance(
    rest_client: &Client,
    store_address: AccountAddress,
    version: u64,
    currency: &Currency,
) -> ApiResult<Vec<Amount>> {
    match rest_client
        .get_account_resource_at_version_bcs(
            store_address,
            "0x1::fungible_asset::FungibleStore",
            version,
        )
        .await
    {
        Ok(response) => {
            let store: FungibleStoreResource = response.into_inner();
            Ok(vec![secondary_store_amount(&store, currency)?])
        },
        Err(RestError::Api(AptosErrorResponse {
            error:
                AptosError {
                    error_code: AptosErrorCode::AccountNotFound,
                    ..
                },
            ..
        }))
        | Err(RestError::Api(AptosErrorResponse {
            error:
                AptosError {
                    error_code: AptosErrorCode::ResourceNotFound,
                    ..
                },
            ..
        })) => Ok(vec![Amount {
            value: 0.to_string(),
            currency: currency.clone(),
        }]),
        Err(err) => Err(ApiError::InternalError(Some(format!(
            "Failed to retrieve secondary store balance: {}",
            err
        )))),
    }
}

fn secondary_store_amount(store: &FungibleStoreResource, currency: &Currency) -> ApiResult<Amount> {
    let expected_metadata = currency_fa_metadata_address(currency)?.ok_or_else(|| {
        ApiError::InvalidInput(Some(format!(
            "Currency {} does not identify a fungible asset metadata address",
            currency.symbol
        )))
    })?;

    if store.metadata() != expected_metadata {
        return Err(ApiError::InvalidInput(Some(format!(
            "Secondary store metadata {} does not match requested currency metadata {}",
            store.metadata(),
            expected_metadata
        ))));
    }

    Ok(Amount {
        value: store.balance().to_string(),
        currency: currency.clone(),
    })
}

fn currency_fa_metadata_address(currency: &Currency) -> ApiResult<Option<AccountAddress>> {
    if let Some(metadata) = currency.metadata.as_ref() {
        if let Some(fa_address) = metadata.fa_address.as_ref() {
            return AccountAddress::from_str(fa_address)
                .map(Some)
                .map_err(|err| ApiError::InvalidInput(Some(err.to_string())));
        }
    }

    if currency == &native_coin() {
        Ok(Some(AccountAddress::TEN))
    } else {
        Ok(None)
    }
}

async fn get_sequence_number(
    rest_client: &Client,
    owner_address: AccountAddress,
    version: u64,
) -> ApiResult<u64> {
    // Retrieve sequence number
    let sequence_number = match rest_client
        .get_account_resource_at_version_bcs(owner_address, "0x1::account::Account", version)
        .await
    {
        Ok(response) => {
            let account: AccountResource = response.into_inner();
            account.sequence_number()
        },
        Err(RestError::Api(AptosErrorResponse {
            error:
                AptosError {
                    error_code: AptosErrorCode::AccountNotFound,
                    ..
                },
            ..
        }))
        | Err(RestError::Api(AptosErrorResponse {
            error:
                AptosError {
                    error_code: AptosErrorCode::ResourceNotFound,
                    ..
                },
            ..
        })) => {
            // If the account or resource doesn't exist, set the sequence number to 0
            0
        },
        _ => {
            // Any other error we can't retrieve the sequence number
            return Err(ApiError::InternalError(Some(
                "Failed to retrieve account sequence number".to_string(),
            )));
        },
    };

    Ok(sequence_number)
}

async fn get_staking_info(
    rest_client: &Client,
    account: &AccountIdentifier,
    owner_address: AccountAddress,
    version: u64,
) -> ApiResult<(Vec<Amount>, u64, Option<Vec<AccountAddress>>)> {
    let mut balances = vec![];
    let mut lockup_expiration: u64 = 0;
    let mut maybe_operators = None;
    let mut total_balance = 0;
    let mut has_staking = false;

    if let Ok(response) = rest_client
        .get_account_resource_at_version_bcs(owner_address, "0x1::staking_contract::Store", version)
        .await
    {
        let store: Store = response.into_inner();
        maybe_operators = Some(vec![]);
        for (operator, contract) in store.staking_contracts {
            // Keep track of operators
            maybe_operators.as_mut().unwrap().push(operator);
            match get_stake_balances(rest_client, account, contract.pool_address, version).await {
                Ok(Some(balance_result)) => {
                    if let Some(balance) = balance_result.balance {
                        has_staking = true;
                        total_balance += u64::from_str(&balance.value).unwrap_or_default();
                    }
                    // TODO: This seems like it only works if there's only one staking contract (hopefully it stays that way)
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
        if has_staking {
            balances.push(Amount {
                value: total_balance.to_string(),
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
    }

    Ok((balances, lockup_expiration, maybe_operators))
}

async fn get_delegation_info(
    rest_client: &Client,
    account: &AccountIdentifier,
    owner_address: AccountAddress,
    pool_address: AccountAddress,
    version: u64,
) -> ApiResult<(Vec<Amount>, u64)> {
    let mut balances = vec![];
    let mut lockup_expiration: u64 = 0;

    match get_delegation_stake_balances(rest_client, account, owner_address, pool_address, version)
        .await
    {
        Ok(Some(balance_result)) => {
            if let Some(balance) = balance_result.balance {
                balances.push(Amount {
                    value: balance.value,
                    currency: native_coin(),
                });
            }
            lockup_expiration = balance_result.lockup_expiration;
        },
        result => {
            warn!(
                    "Failed to retrieve requested balance for delegator_address: {}, pool_address: {}: {:?}",
                    owner_address, pool_address, result
                )
        },
    }
    Ok((balances, lockup_expiration))
}

async fn get_base_balances(
    rest_client: &Client,
    owner_address: AccountAddress,
    version: u64,
    currencies_to_lookup: HashSet<Currency>,
) -> ApiResult<Vec<Amount>> {
    let mut balances = vec![];

    // Retrieve the fungible asset balances and the coin balances
    for currency in currencies_to_lookup.iter() {
        match *currency {
            // FA only
            Currency {
                metadata:
                    Some(CurrencyMetadata {
                        move_type: None,
                        fa_address: Some(ref fa_address),
                    }),
                ..
            } => {
                let response = view::<Vec<u64>>(
                    rest_client,
                    version,
                    AccountAddress::ONE,
                    ident_str!(PRIMARY_FUNGIBLE_STORE_MODULE),
                    ident_str!(BALANCE_FUNCTION),
                    vec![TypeTag::Struct(Box::new(StructTag {
                        address: AccountAddress::ONE,
                        module: ident_str!(OBJECT_MODULE).into(),
                        name: ident_str!(OBJECT_CORE_RESOURCE).into(),
                        type_args: vec![],
                    }))],
                    vec![
                        bcs::to_bytes(&owner_address).unwrap(),
                        bcs::to_bytes(&AccountAddress::from_str(fa_address).unwrap()).unwrap(),
                    ],
                )
                .await?;
                let fa_balance = response.first().copied().unwrap_or(0);
                balances.push(Amount {
                    value: fa_balance.to_string(),
                    currency: currency.clone(),
                })
            },
            // Coin or Coin and FA combined
            Currency {
                metadata:
                    Some(CurrencyMetadata {
                        move_type: Some(ref coin_type),
                        fa_address: _,
                    }),
                ..
            } => {
                if let Ok(type_tag) = parse_type_tag(coin_type) {
                    let response = view::<Vec<u64>>(
                        rest_client,
                        version,
                        AccountAddress::ONE,
                        ident_str!(COIN_MODULE),
                        ident_str!(BALANCE_FUNCTION),
                        vec![type_tag],
                        vec![bcs::to_bytes(&owner_address)?],
                    )
                    .await?;
                    let coin_balance = response.first().copied().unwrap_or(0);
                    balances.push(Amount {
                        value: coin_balance.to_string(),
                        currency: currency.clone(),
                    })
                }
            },
            _ => {
                // None for both, means we can't look it up anyways / it's invalid
            },
        }
    }

    Ok(balances)
}

pub async fn view<T: DeserializeOwned>(
    rest_client: &Client,
    version: u64,
    address: AccountAddress,
    module: &'static IdentStr,
    function: &'static IdentStr,
    type_args: Vec<TypeTag>,
    args: Vec<Vec<u8>>,
) -> ApiResult<T> {
    Ok(rest_client
        .view_bcs::<T>(
            &ViewFunction {
                module: ModuleId {
                    address,
                    name: module.into(),
                },
                function: function.into(),
                ty_args: type_args,
                args,
            },
            Some(version),
        )
        .await?
        .into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CurrencyMetadata;

    #[test]
    fn test_secondary_store_amount_for_native_coin() {
        let store = FungibleStoreResource::new(AccountAddress::TEN, 50_000_000, false);
        let amount = secondary_store_amount(&store, &native_coin()).unwrap();

        assert_eq!(amount.value, "50000000");
        assert_eq!(amount.currency, native_coin());
    }

    #[test]
    fn test_secondary_store_amount_preserves_fa_currency() {
        let metadata_address =
            AccountAddress::from_str("0x12341234123412341234123412341234").unwrap();
        let currency = Currency {
            symbol: "FUN".to_string(),
            decimals: 2,
            metadata: Some(CurrencyMetadata {
                move_type: None,
                fa_address: Some(metadata_address.to_string()),
            }),
        };
        let store = FungibleStoreResource::new(metadata_address, 123, false);
        let amount = secondary_store_amount(&store, &currency).unwrap();

        assert_eq!(amount.value, "123");
        assert_eq!(amount.currency, currency);
    }

    #[test]
    fn test_secondary_store_amount_rejects_mismatched_currency() {
        let store_metadata =
            AccountAddress::from_str("0x12341234123412341234123412341234").unwrap();
        let requested_metadata =
            AccountAddress::from_str("0x56785678567856785678567856785678").unwrap();
        let currency = Currency {
            symbol: "FUN".to_string(),
            decimals: 2,
            metadata: Some(CurrencyMetadata {
                move_type: None,
                fa_address: Some(requested_metadata.to_string()),
            }),
        };
        let store = FungibleStoreResource::new(store_metadata, 123, false);

        assert!(secondary_store_amount(&store, &currency).is_err());
    }
}
