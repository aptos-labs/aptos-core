// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Account balance endpoint.
//!
//! Supports both legacy coin balances (`0x1::aptos_coin::AptosCoin`) and
//! fungible asset balances (by metadata address). For coins with a paired
//! fungible asset, the balance is the sum of both.

use crate::v2::{
    context::{spawn_blocking, V2Context},
    error::{ErrorCode, V2Error},
    types::{LedgerVersionParam, V2Response},
};
use aptos_api_types::AssetType;
use aptos_sdk::types::{get_paired_fa_metadata_address, get_paired_fa_primary_store_address};
use aptos_types::{
    account_address::AccountAddress,
    account_config::{
        CoinStoreResourceUntyped, ConcurrentFungibleBalanceResource, FungibleStoreResource,
        ObjectGroupResource,
    },
    state_store::{state_key::StateKey, TStateView},
};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use move_core_types::{language_storage::StructTag, move_resource::MoveStructType};
use serde::Serialize;

/// Balance response.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct BalanceResponse {
    /// The total balance (coin + fungible asset).
    pub balance: u64,
}

/// GET /v2/accounts/:address/balance/:asset_type
///
/// Returns the balance of a specific asset for the given account.
/// The `asset_type` can be:
/// - A Move struct tag for a coin type (e.g., `0x1::aptos_coin::AptosCoin`)
/// - A hex address for a fungible asset metadata object
///
/// For coins with a paired fungible asset, the returned balance is the sum of
/// the legacy coin balance and the fungible asset balance.
#[utoipa::path(
    get,
    path = "/v2/accounts/{address}/balance/{asset_type}",
    tag = "Accounts",
    params(
        ("address" = String, Path, description = "Account address (hex)"),
        ("asset_type" = String, Path, description = "Coin struct tag (e.g. 0x1::aptos_coin::AptosCoin) or FA metadata address"),
        LedgerVersionParam,
    ),
    responses(
        (status = 200, description = "Account balance", body = Object),
        (status = 404, description = "Account or asset not found", body = V2Error),
    )
)]
pub async fn get_balance_handler(
    State(ctx): State<V2Context>,
    Path((address, asset_type_str)): Path<(String, String)>,
    Query(params): Query<LedgerVersionParam>,
) -> Result<Json<V2Response<BalanceResponse>>, V2Error> {
    let ctx = ctx.clone();
    spawn_blocking(move || {
        let address = parse_address(&address)?;
        let asset_type: AssetType = asset_type_str.parse().map_err(|e: anyhow::Error| {
            V2Error::bad_request(
                ErrorCode::InvalidInput,
                format!("Invalid asset type '{}': {}", asset_type_str, e),
            )
        })?;

        let (ledger_info, _version, state_view) = ctx.state_view_at(params.ledger_version)?;

        // Resolve coin balance and FA metadata address
        let (fa_metadata_address, mut balance) = match &asset_type {
            AssetType::Coin(move_struct_tag) => {
                // Read CoinStore<T> resource
                let coin_store_tag = format!("0x1::coin::CoinStore<{}>", move_struct_tag)
                    .parse::<StructTag>()
                    .map_err(V2Error::internal)?;

                let state_key =
                    StateKey::resource(&address, &coin_store_tag).map_err(V2Error::internal)?;

                let coin_balance = match state_view
                    .get_state_value_bytes(&state_key)
                    .map_err(V2Error::internal)?
                {
                    Some(bytes) => {
                        let coin_store: CoinStoreResourceUntyped =
                            bcs::from_bytes(&bytes).map_err(V2Error::internal)?;
                        coin_store.coin()
                    },
                    None => 0,
                };

                (
                    get_paired_fa_metadata_address(move_struct_tag),
                    coin_balance,
                )
            },
            AssetType::FungibleAsset(fa_address) => ((*fa_address).into(), 0u64),
        };

        // Read fungible asset balance from primary store
        let primary_store_address =
            get_paired_fa_primary_store_address(address, fa_metadata_address);
        let group_key =
            StateKey::resource_group(&primary_store_address, &ObjectGroupResource::struct_tag());

        if let Some(data_blob) = state_view
            .get_state_value_bytes(&group_key)
            .map_err(V2Error::internal)?
        {
            if let Ok(object_group) = bcs::from_bytes::<ObjectGroupResource>(&data_blob) {
                if let Some(fa_store_bytes) =
                    object_group.group.get(&FungibleStoreResource::struct_tag())
                {
                    let fa_store: FungibleStoreResource =
                        bcs::from_bytes(fa_store_bytes).map_err(V2Error::internal)?;

                    if fa_store.balance != 0 {
                        balance += fa_store.balance();
                    } else if let Some(concurrent_bytes) = object_group
                        .group
                        .get(&ConcurrentFungibleBalanceResource::struct_tag())
                    {
                        let concurrent: ConcurrentFungibleBalanceResource =
                            bcs::from_bytes(concurrent_bytes).map_err(V2Error::internal)?;
                        balance += concurrent.balance();
                    }
                }
            }
        }

        Ok(Json(V2Response::new(
            BalanceResponse { balance },
            &ledger_info,
        )))
    })
    .await
}

fn parse_address(s: &str) -> Result<AccountAddress, V2Error> {
    AccountAddress::from_hex_literal(s)
        .or_else(|_| AccountAddress::from_hex(s))
        .map_err(|e| {
            V2Error::bad_request(ErrorCode::InvalidInput, format!("Invalid address: {}", e))
        })
}
