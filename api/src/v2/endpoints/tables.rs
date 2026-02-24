// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::v2::{
    context::{spawn_blocking, V2Context},
    error::{ErrorCode, V2Error},
    types::{LedgerVersionParam, V2Response},
};
use aptos_api_types::{AsConverter, MoveValue, TableItemRequest, VerifyInput};
use aptos_types::{account_address::AccountAddress, state_store::table::TableHandle};
use axum::{
    extract::{Path, Query, State},
    Json,
};

/// POST /v2/tables/:table_handle/item -- Get a table item.
///
/// Retrieves a table item by its key, given the table handle.
/// The request body specifies the key_type, value_type, and key.
#[utoipa::path(
    post,
    path = "/v2/tables/{table_handle}/item",
    tag = "Tables",
    params(
        ("table_handle" = String, Path, description = "Table handle (hex)"),
        LedgerVersionParam,
    ),
    request_body(content = Object, description = "TableItemRequest with key_type, value_type, and key"),
    responses(
        (status = 200, description = "Table item value", body = Object),
        (status = 404, description = "Table item not found", body = V2Error),
    )
)]
pub async fn get_table_item_handler(
    State(ctx): State<V2Context>,
    Path(table_handle): Path<String>,
    Query(params): Query<LedgerVersionParam>,
    Json(request): Json<TableItemRequest>,
) -> Result<Json<V2Response<MoveValue>>, V2Error> {
    // Validate the request
    request
        .verify()
        .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, e.to_string()))?;

    let ctx = ctx.clone();
    spawn_blocking(move || {
        let handle_address = parse_address(&table_handle)?;
        let (ledger_info, _version, state_view) = ctx.state_view_at(params.ledger_version)?;

        // Parse key and value types
        let key_type = (&request.key_type).try_into().map_err(|e: anyhow::Error| {
            V2Error::bad_request(ErrorCode::InvalidInput, format!("Invalid key_type: {}", e))
        })?;
        let value_type = (&request.value_type)
            .try_into()
            .map_err(|e: anyhow::Error| {
                V2Error::bad_request(
                    ErrorCode::InvalidInput,
                    format!("Invalid value_type: {}", e),
                )
            })?;

        let converter =
            state_view.as_converter(ctx.inner().db.clone(), ctx.inner().indexer_reader.clone());

        // Convert key to VM value and serialize
        let vm_key = converter
            .try_into_vm_value(&key_type, request.key.clone())
            .map_err(|e| {
                V2Error::bad_request(ErrorCode::InvalidInput, format!("Invalid key: {}", e))
            })?;
        let raw_key = vm_key.undecorate().simple_serialize().ok_or_else(|| {
            V2Error::bad_request(ErrorCode::InvalidInput, "Failed to serialize table key")
        })?;

        // Look up the value
        let state_key = aptos_types::state_store::state_key::StateKey::table_item(
            &TableHandle(handle_address.into()),
            &raw_key,
        );

        use aptos_types::state_store::TStateView;
        let bytes = state_view
            .get_state_value_bytes(&state_key)
            .map_err(V2Error::internal)?
            .ok_or_else(|| {
                V2Error::not_found(
                    ErrorCode::TableItemNotFound,
                    format!("Table item not found for key: {}", request.key),
                )
            })?;

        // Convert value to MoveValue
        let move_value = converter
            .try_into_move_value(&value_type, &bytes)
            .map_err(V2Error::internal)?;

        Ok(Json(V2Response::new(move_value, &ledger_info)))
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
