// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::v2::{
    context::{spawn_blocking, V2Context},
    error::{ErrorCode, V2Error},
    types::{LedgerVersionParam, V2Response},
};
use aptos_api_types::AccountData;
use aptos_types::{
    account_address::AccountAddress, account_config::AccountResource, on_chain_config::FeatureFlag,
};
use axum::{
    extract::{Path, Query, State},
    Json,
};

/// GET /v2/accounts/:address -- Get account info (sequence number + auth key).
#[utoipa::path(
    get,
    path = "/v2/accounts/{address}",
    tag = "Accounts",
    params(
        ("address" = String, Path, description = "Account address (hex)"),
        LedgerVersionParam,
    ),
    responses(
        (status = 200, description = "Account info", body = Object),
        (status = 404, description = "Account not found", body = V2Error),
    )
)]
pub async fn get_account_handler(
    State(ctx): State<V2Context>,
    Path(address): Path<String>,
    Query(params): Query<LedgerVersionParam>,
) -> Result<Json<V2Response<AccountData>>, V2Error> {
    let ctx = ctx.clone();
    spawn_blocking(move || {
        let address = parse_address(&address)?;
        let (ledger_info, version, state_view) = ctx.state_view_at(params.ledger_version)?;

        // Try to read the AccountResource
        let state_key = aptos_types::state_store::state_key::StateKey::resource_typed::<
            AccountResource,
        >(&address)
        .map_err(V2Error::internal)?;

        use aptos_types::state_store::TStateView;
        let account_resource_bytes = state_view
            .get_state_value_bytes(&state_key)
            .map_err(V2Error::internal)?;

        let account_data = match account_resource_bytes {
            Some(bytes) => {
                let account_resource: AccountResource =
                    bcs::from_bytes(&bytes).map_err(V2Error::internal)?;
                AccountData::from(account_resource)
            },
            None => {
                // Check if stateless accounts are enabled
                let stateless_enabled = ctx
                    .inner()
                    .feature_enabled(FeatureFlag::DEFAULT_ACCOUNT_RESOURCE)
                    .unwrap_or(false);

                if stateless_enabled {
                    let default = AccountResource::new_stateless(address);
                    AccountData::from(default)
                } else {
                    return Err(V2Error::not_found(
                        ErrorCode::AccountNotFound,
                        format!("Account {} not found at version {}", address, version),
                    ));
                }
            },
        };

        Ok(Json(V2Response::new(account_data, &ledger_info)))
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
