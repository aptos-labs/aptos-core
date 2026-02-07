// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::v2::{
    context::{spawn_blocking, V2Context},
    cursor::Cursor,
    error::{ErrorCode, V2Error},
    types::{LedgerVersionParam, PaginatedLedgerParams, V2Response},
};
use aptos_api_types::MoveModuleBytecode;
use aptos_types::account_address::AccountAddress;
use axum::{
    extract::{Path, Query, State},
    Json,
};

/// GET /v2/accounts/:address/modules
pub async fn get_modules_handler(
    State(ctx): State<V2Context>,
    Path(address): Path<String>,
    Query(params): Query<PaginatedLedgerParams>,
) -> Result<Json<V2Response<Vec<MoveModuleBytecode>>>, V2Error> {
    let ctx = ctx.clone();
    spawn_blocking(move || {
        let address = parse_address(&address)?;
        let (ledger_info, version, _state_view) = ctx.state_view_at(params.ledger_version)?;

        let cursor = params
            .cursor
            .as_ref()
            .map(|c| Cursor::decode(c))
            .transpose()?;

        let (modules, next_cursor) =
            ctx.get_modules_paginated(address, cursor.as_ref(), version)?;

        let mut move_modules = Vec::with_capacity(modules.len());
        for (_module_id, bytes) in modules {
            let m = MoveModuleBytecode::new(bytes)
                .try_parse_abi()
                .map_err(|e| V2Error::internal(e))?;
            move_modules.push(m);
        }

        let cursor_str = next_cursor.map(|c| c.encode());
        Ok(Json(
            V2Response::new(move_modules, &ledger_info).with_cursor(cursor_str),
        ))
    })
    .await
}

/// GET /v2/accounts/:address/module/:module_name
pub async fn get_module_handler(
    State(ctx): State<V2Context>,
    Path((address, module_name)): Path<(String, String)>,
    Query(params): Query<LedgerVersionParam>,
) -> Result<Json<V2Response<MoveModuleBytecode>>, V2Error> {
    let ctx = ctx.clone();
    spawn_blocking(move || {
        let address = parse_address(&address)?;
        let (ledger_info, _version, state_view) = ctx.state_view_at(params.ledger_version)?;

        let module_id = move_core_types::language_storage::ModuleId::new(
            address,
            move_core_types::identifier::Identifier::new(module_name.clone()).map_err(|e| {
                V2Error::bad_request(
                    ErrorCode::InvalidInput,
                    format!("Invalid module name: {}", e),
                )
            })?,
        );

        let state_key = aptos_types::state_store::state_key::StateKey::module_id(&module_id);

        use aptos_types::state_store::TStateView;
        let module_bytes = state_view
            .get_state_value_bytes(&state_key)
            .map_err(V2Error::internal)?
            .ok_or_else(|| {
                V2Error::not_found(
                    ErrorCode::ModuleNotFound,
                    format!("Module {} not found at {}", module_name, address),
                )
            })?;

        let module = MoveModuleBytecode::new(module_bytes.to_vec())
            .try_parse_abi()
            .map_err(V2Error::internal)?;

        Ok(Json(V2Response::new(module, &ledger_info)))
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
