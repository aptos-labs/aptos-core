// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::v2::{
    context::{spawn_blocking, V2Context},
    cursor::Cursor,
    error::{ErrorCode, V2Error},
    types::{CursorOnlyParams, V2Response},
};
use aptos_types::account_address::AccountAddress;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Serialize;

/// Lightweight transaction summary for account transaction listings.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TransactionSummary {
    pub version: u64,
    pub hash: String,
    pub sender: String,
}

/// GET /v2/accounts/:address/transactions
#[utoipa::path(
    get,
    path = "/v2/accounts/{address}/transactions",
    tag = "Accounts",
    params(
        ("address" = String, Path, description = "Account address (hex)"),
        CursorOnlyParams,
    ),
    responses(
        (status = 200, description = "Paginated account transaction summaries",
            body = V2Response<Vec<TransactionSummary>>),
        (status = 404, description = "Account not found", body = V2Error),
    )
)]
pub async fn get_account_transactions_handler(
    State(ctx): State<V2Context>,
    Path(address): Path<String>,
    Query(params): Query<CursorOnlyParams>,
) -> Result<Json<V2Response<Vec<TransactionSummary>>>, V2Error> {
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

        let page_size = ctx.v2_config.max_transactions_page_size;

        // Use cursor as start_version for the next page
        let start_version = cursor.as_ref().map(|c| c.as_version()).transpose()?;

        let summaries = ctx
            .inner()
            .db
            .get_account_transaction_summaries(
                address,
                start_version,
                None, // end_version
                page_size as u64,
                ledger_version,
            )
            .map_err(|e| V2Error::internal(e))?;

        let rendered: Vec<TransactionSummary> = summaries
            .iter()
            .map(|s| TransactionSummary {
                version: s.version(),
                hash: s.transaction_hash().to_hex_literal(),
                sender: format!("{}", s.sender()),
            })
            .collect();

        let next_cursor = if summaries.len() as u16 == page_size {
            summaries
                .last()
                .map(|s| Cursor::from_version(s.version() + 1))
        } else {
            None
        };

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
