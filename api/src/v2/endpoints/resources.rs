// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::v2::{
    context::{spawn_blocking, V2Context},
    cursor::Cursor,
    error::{ErrorCode, V2Error},
    types::{LedgerVersionParam, PaginatedLedgerParams, V2Response},
};
use aptos_api_types::{AsConverter, MoveResource};
use aptos_types::account_address::AccountAddress;
use axum::{
    extract::{Path, Query, State},
    Json,
};

/// GET /v2/accounts/:address/resources
#[utoipa::path(
    get,
    path = "/v2/accounts/{address}/resources",
    tag = "Accounts",
    params(
        ("address" = String, Path, description = "Account address (hex)"),
        PaginatedLedgerParams,
    ),
    responses(
        (status = 200, description = "Paginated list of account resources", body = Object),
        (status = 404, description = "Account not found", body = V2Error),
    )
)]
pub async fn get_resources_handler(
    State(ctx): State<V2Context>,
    Path(address): Path<String>,
    Query(params): Query<PaginatedLedgerParams>,
) -> Result<Json<V2Response<Vec<MoveResource>>>, V2Error> {
    let ctx = ctx.clone();
    spawn_blocking(move || {
        let address = parse_address(&address)?;
        let (ledger_info, version, state_view) = ctx.state_view_at(params.ledger_version)?;

        let cursor = params
            .cursor
            .as_ref()
            .map(|c| Cursor::decode(c))
            .transpose()?;

        let (resources, next_cursor) =
            ctx.get_resources_paginated(address, cursor.as_ref(), version)?;

        let converter =
            state_view.as_converter(ctx.inner().db.clone(), ctx.inner().indexer_reader.clone());

        let move_resources: Vec<MoveResource> = resources
            .into_iter()
            .map(|(tag, bytes)| converter.try_into_resource(&tag, &bytes))
            .collect::<anyhow::Result<Vec<_>>>()
            .map_err(V2Error::internal)?;

        let cursor_str = next_cursor.map(|c| c.encode());
        Ok(Json(
            V2Response::new(move_resources, &ledger_info).with_cursor(cursor_str),
        ))
    })
    .await
}

/// GET /v2/accounts/:address/resource/:resource_type
#[utoipa::path(
    get,
    path = "/v2/accounts/{address}/resource/{resource_type}",
    tag = "Accounts",
    params(
        ("address" = String, Path, description = "Account address (hex)"),
        ("resource_type" = String, Path, description = "Move struct tag (e.g. 0x1::account::Account)"),
        LedgerVersionParam,
    ),
    responses(
        (status = 200, description = "Single account resource", body = Object),
        (status = 404, description = "Resource not found", body = V2Error),
    )
)]
pub async fn get_resource_handler(
    State(ctx): State<V2Context>,
    Path((address, resource_type)): Path<(String, String)>,
    Query(params): Query<LedgerVersionParam>,
) -> Result<Json<V2Response<MoveResource>>, V2Error> {
    let ctx = ctx.clone();
    spawn_blocking(move || {
        let address = parse_address(&address)?;
        let tag = parse_struct_tag(&resource_type)?;
        let (ledger_info, _version, state_view) = ctx.state_view_at(params.ledger_version)?;

        let converter =
            state_view.as_converter(ctx.inner().db.clone(), ctx.inner().indexer_reader.clone());

        let api_address: aptos_api_types::Address = address.into();
        let bytes = converter
            .find_resource(&state_view, api_address, &tag)
            .map_err(V2Error::internal)?
            .ok_or_else(|| {
                V2Error::not_found(
                    ErrorCode::ResourceNotFound,
                    format!("Resource {:?} not found at {}", tag, address),
                )
            })?;

        let resource = converter
            .try_into_resource(&tag, &bytes)
            .map_err(V2Error::internal)?;

        Ok(Json(V2Response::new(resource, &ledger_info)))
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

fn parse_struct_tag(s: &str) -> Result<move_core_types::language_storage::StructTag, V2Error> {
    s.parse().map_err(|e| {
        V2Error::bad_request(
            ErrorCode::InvalidInput,
            format!("Invalid struct tag: {}", e),
        )
    })
}
