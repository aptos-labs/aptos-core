// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::v2::{
    context::{spawn_blocking, V2Context},
    cursor::Cursor,
    error::{ErrorCode, V2Error},
    types::{CursorOnlyParams, V2Response},
};
use aptos_api_types::{AsConverter, Transaction, TransactionOnChainData};
use aptos_crypto::HashValue;
use axum::{
    extract::{Path, Query, State},
    Json,
};

/// GET /v2/transactions -- paginated list of committed transactions.
pub async fn list_transactions_handler(
    State(ctx): State<V2Context>,
    Query(params): Query<CursorOnlyParams>,
) -> Result<Json<V2Response<Vec<Transaction>>>, V2Error> {
    let ctx = ctx.clone();
    spawn_blocking(move || {
        let ledger_info = ctx.ledger_info()?;
        let ledger_version = ledger_info.version();

        let cursor = params
            .cursor
            .as_ref()
            .map(|c| Cursor::decode(c))
            .transpose()?;

        let (txns, next_cursor) =
            ctx.get_transactions_paginated(cursor.as_ref(), ledger_version)?;

        let rendered = render_transactions(&ctx, &ledger_info, txns)?;
        let cursor_str = next_cursor.map(|c| c.encode());

        Ok(Json(
            V2Response::new(rendered, &ledger_info).with_cursor(cursor_str),
        ))
    })
    .await
}

/// GET /v2/transactions/:hash
pub async fn get_transaction_handler(
    State(ctx): State<V2Context>,
    Path(hash): Path<String>,
) -> Result<Json<V2Response<Transaction>>, V2Error> {
    let ctx = ctx.clone();
    spawn_blocking(move || {
        let hash = parse_hash(&hash)?;
        let ledger_info = ctx.ledger_info()?;
        let version = ledger_info.version();

        match ctx
            .inner()
            .get_transaction_by_hash(hash, version)
            .map_err(V2Error::internal)?
        {
            Some(txn_data) => {
                let txn = render_single_transaction(&ctx, txn_data)?;
                Ok(Json(V2Response::new(txn, &ledger_info)))
            },
            None => Err(V2Error::not_found(
                ErrorCode::TransactionNotFound,
                format!("Transaction {} not found", hash),
            )),
        }
    })
    .await
}

// --- Internal helpers ---

fn render_transactions(
    ctx: &V2Context,
    _ledger_info: &aptos_api_types::LedgerInfo,
    txns: Vec<TransactionOnChainData>,
) -> Result<Vec<Transaction>, V2Error> {
    let state_view = ctx.inner().latest_state_view().map_err(V2Error::internal)?;
    let converter =
        state_view.as_converter(ctx.inner().db.clone(), ctx.inner().indexer_reader.clone());

    txns.into_iter()
        .map(|txn_data| {
            let timestamp = ctx
                .inner()
                .db
                .get_block_timestamp(txn_data.version)
                .map_err(V2Error::internal)?;
            converter
                .try_into_onchain_transaction(timestamp, txn_data)
                .map_err(V2Error::internal)
        })
        .collect()
}

fn render_single_transaction(
    ctx: &V2Context,
    txn_data: TransactionOnChainData,
) -> Result<Transaction, V2Error> {
    let timestamp = ctx
        .inner()
        .db
        .get_block_timestamp(txn_data.version)
        .map_err(V2Error::internal)?;
    let state_view = ctx.inner().latest_state_view().map_err(V2Error::internal)?;
    let converter =
        state_view.as_converter(ctx.inner().db.clone(), ctx.inner().indexer_reader.clone());
    converter
        .try_into_onchain_transaction(timestamp, txn_data)
        .map_err(V2Error::internal)
}

fn parse_hash(s: &str) -> Result<HashValue, V2Error> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    HashValue::from_hex(s).map_err(|e| {
        V2Error::bad_request(
            ErrorCode::InvalidInput,
            format!("Invalid transaction hash: {}", e),
        )
    })
}
