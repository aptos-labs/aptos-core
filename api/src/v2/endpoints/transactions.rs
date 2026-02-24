// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::v2::{
    context::{spawn_blocking, V2Context},
    cursor::Cursor,
    error::{ErrorCode, V2Error},
    extractors::BcsOnly,
    types::{CursorOnlyParams, V2Response},
};
use aptos_api_types::{AsConverter, Transaction, TransactionOnChainData};
use aptos_crypto::HashValue;
use aptos_types::transaction::SignedTransaction;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Serialize;
use std::time::Duration;

/// GET /v2/transactions -- paginated list of committed transactions.
#[utoipa::path(
    get,
    path = "/v2/transactions",
    tag = "Transactions",
    params(CursorOnlyParams),
    responses(
        (status = 200, description = "Paginated list of transactions", body = Object),
        (status = 500, description = "Internal error", body = V2Error),
    )
)]
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
#[utoipa::path(
    get,
    path = "/v2/transactions/{hash}",
    tag = "Transactions",
    params(("hash" = String, Path, description = "Transaction hash (0x-prefixed hex)")),
    responses(
        (status = 200, description = "Transaction details", body = Object),
        (status = 404, description = "Transaction not found", body = V2Error),
    )
)]
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

/// POST /v2/transactions -- submit a signed transaction (BCS only).
#[utoipa::path(
    post,
    path = "/v2/transactions",
    tag = "Transactions",
    request_body(content = Vec<u8>, content_type = "application/x-bcs",
        description = "BCS-encoded signed transaction with version envelope"),
    responses(
        (status = 200, description = "Transaction accepted", body = V2Response<SubmitResult>),
        (status = 422, description = "Rejected by mempool", body = V2Error),
    )
)]
pub async fn submit_transaction_handler(
    State(ctx): State<V2Context>,
    BcsOnly(versioned): BcsOnly<SignedTransaction>,
) -> Result<Json<V2Response<SubmitResult>>, V2Error> {
    let txn = versioned.into_inner();
    let hash = txn.committed_hash();

    let (mempool_status, vm_status_opt) = ctx
        .inner()
        .submit_transaction(txn)
        .await
        .map_err(V2Error::internal)?;

    use aptos_types::mempool_status::MempoolStatusCode;
    if mempool_status.code == MempoolStatusCode::Accepted {
        let ledger_info = ctx.ledger_info()?;
        Ok(Json(V2Response::new(
            SubmitResult {
                hash: hash.to_hex_literal(),
                status: "accepted".to_string(),
            },
            &ledger_info,
        )))
    } else {
        let msg = vm_status_opt
            .map(|s| format!("{:?}: {:?}", mempool_status.code, s))
            .unwrap_or_else(|| format!("{:?}", mempool_status.code));
        Err(V2Error::bad_request(ErrorCode::MempoolRejected, msg))
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SubmitResult {
    pub hash: String,
    pub status: String,
}

/// GET /v2/transactions/:hash/wait -- poll until the transaction is committed or timeout.
#[utoipa::path(
    get,
    path = "/v2/transactions/{hash}/wait",
    tag = "Transactions",
    params(("hash" = String, Path, description = "Transaction hash (0x-prefixed hex)")),
    responses(
        (status = 200, description = "Transaction committed", body = Object),
        (status = 404, description = "Transaction not found within timeout", body = V2Error),
    )
)]
pub async fn wait_transaction_handler(
    State(ctx): State<V2Context>,
    Path(hash): Path<String>,
) -> Result<Json<V2Response<Transaction>>, V2Error> {
    let hash_value = parse_hash(&hash)?;
    let timeout_ms = ctx.v2_config.wait_by_hash_timeout_ms;
    let poll_interval_ms = ctx.v2_config.wait_by_hash_poll_interval_ms;

    let start_time = std::time::Instant::now();

    loop {
        let ledger_info = ctx.ledger_info()?;
        let version = ledger_info.version();

        let ctx_clone = ctx.clone();
        let result = spawn_blocking(move || {
            ctx_clone
                .inner()
                .get_transaction_by_hash(hash_value, version)
                .map_err(V2Error::internal)
        })
        .await?;

        match result {
            Some(txn_data) => {
                let ctx_clone = ctx.clone();
                let txn =
                    spawn_blocking(move || render_single_transaction(&ctx_clone, txn_data)).await?;
                let ledger_info = ctx.ledger_info()?;
                return Ok(Json(V2Response::new(txn, &ledger_info)));
            },
            None => {
                if (start_time.elapsed().as_millis() as u64) >= timeout_ms {
                    return Err(V2Error::not_found(
                        ErrorCode::TransactionNotFound,
                        format!(
                            "Transaction {} not found within {}ms timeout",
                            hash, timeout_ms
                        ),
                    ));
                }
                tokio::time::sleep(Duration::from_millis(poll_interval_ms)).await;
            },
        }
    }
}

/// GET /v2/transactions/by_version/:version
#[utoipa::path(
    get,
    path = "/v2/transactions/by_version/{version}",
    tag = "Transactions",
    params(("version" = u64, Path, description = "Transaction version number")),
    responses(
        (status = 200, description = "Transaction details", body = Object),
        (status = 404, description = "Transaction not found", body = V2Error),
        (status = 410, description = "Version pruned", body = V2Error),
    )
)]
pub async fn get_transaction_by_version_handler(
    State(ctx): State<V2Context>,
    Path(version): Path<u64>,
) -> Result<Json<V2Response<Transaction>>, V2Error> {
    let ctx = ctx.clone();
    spawn_blocking(move || {
        let ledger_info = ctx.ledger_info()?;
        let ledger_version = ledger_info.version();

        if version > ledger_version {
            return Err(V2Error::not_found(
                ErrorCode::VersionNotFound,
                format!(
                    "Transaction version {} not found (latest: {})",
                    version, ledger_version
                ),
            ));
        }

        let oldest = ledger_info.oldest_version();
        if version < oldest {
            return Err(V2Error::gone(
                ErrorCode::VersionPruned,
                format!(
                    "Transaction version {} has been pruned (oldest: {})",
                    version, oldest
                ),
            ));
        }

        let txn_data = ctx
            .inner()
            .get_transaction_by_version(version, ledger_version)
            .map_err(V2Error::internal)?;

        let txn = render_single_transaction(&ctx, txn_data)?;
        Ok(Json(V2Response::new(txn, &ledger_info)))
    })
    .await
}

#[allow(clippy::result_large_err)]
fn parse_hash(s: &str) -> Result<HashValue, V2Error> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    HashValue::from_hex(s).map_err(|e| {
        V2Error::bad_request(
            ErrorCode::InvalidInput,
            format!("Invalid transaction hash: {}", e),
        )
    })
}
