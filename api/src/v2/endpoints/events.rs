// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::v2::{
    context::{spawn_blocking, V2Context},
    cursor::Cursor,
    error::{ErrorCode, V2Error},
    types::{CursorOnlyParams, V2Response},
};
use aptos_api_types::{AsConverter, VersionedEvent};
use aptos_types::{account_address::AccountAddress, event::EventKey};
use axum::{
    extract::{Path, Query, State},
    Json,
};

/// GET /v2/accounts/:address/events/:creation_number
pub async fn get_events_handler(
    State(ctx): State<V2Context>,
    Path((address, creation_number)): Path<(String, u64)>,
    Query(params): Query<CursorOnlyParams>,
) -> Result<Json<V2Response<Vec<VersionedEvent>>>, V2Error> {
    let ctx = ctx.clone();
    spawn_blocking(move || {
        let address = parse_address(&address)?;
        let ledger_info = ctx.ledger_info()?;
        let ledger_version = ledger_info.version();

        let cursor = params
            .cursor
            .as_ref()
            .map(|c| Cursor::decode(c))
            .transpose()?;

        let event_key = EventKey::new(creation_number, address);
        let (events, next_cursor) =
            ctx.get_events_paginated(&event_key, cursor.as_ref(), ledger_version)?;

        let state_view = ctx.inner().latest_state_view().map_err(V2Error::internal)?;
        let converter =
            state_view.as_converter(ctx.inner().db.clone(), ctx.inner().indexer_reader.clone());

        let rendered = converter
            .try_into_versioned_events(&events)
            .map_err(V2Error::internal)?;

        let cursor_str = next_cursor.map(|c| c.encode());
        Ok(Json(
            V2Response::new(rendered, &ledger_info).with_cursor(cursor_str),
        ))
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
