// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::v2::{
    context::{spawn_blocking, V2Context},
    error::V2Error,
    types::{BlockParams, V2Response},
};
use aptos_api_types::{AsConverter, Block};
use axum::{
    extract::{Path, Query, State},
    Json,
};

/// GET /v2/blocks/:height
pub async fn get_block_by_height_handler(
    State(ctx): State<V2Context>,
    Path(height): Path<u64>,
    Query(params): Query<BlockParams>,
) -> Result<Json<V2Response<Block>>, V2Error> {
    let ctx = ctx.clone();
    spawn_blocking(move || {
        let with_txns = params.with_transactions.unwrap_or(false);
        let (bcs_block, ledger_info) = ctx.get_block_by_height(height, with_txns)?;
        let block = render_block(&ctx, &ledger_info, bcs_block)?;
        Ok(Json(V2Response::new(block, &ledger_info)))
    })
    .await
}

/// GET /v2/blocks/latest
pub async fn get_latest_block_handler(
    State(ctx): State<V2Context>,
) -> Result<Json<V2Response<Block>>, V2Error> {
    let ctx = ctx.clone();
    spawn_blocking(move || {
        let ledger_info = ctx.ledger_info()?;
        let block_height: u64 = ledger_info.block_height.into();
        let (bcs_block, ledger_info) = ctx.get_block_by_height(block_height, false)?;
        let block = render_block(&ctx, &ledger_info, bcs_block)?;
        Ok(Json(V2Response::new(block, &ledger_info)))
    })
    .await
}

fn render_block(
    ctx: &V2Context,
    ledger_info: &aptos_api_types::LedgerInfo,
    bcs_block: aptos_api_types::BcsBlock,
) -> Result<Block, V2Error> {
    let state_view = ctx.inner().latest_state_view().map_err(V2Error::internal)?;
    let converter =
        state_view.as_converter(ctx.inner().db.clone(), ctx.inner().indexer_reader.clone());

    let (block_hash, block_timestamp, first_version, last_version, transactions) = (
        bcs_block.block_hash,
        bcs_block.block_timestamp,
        bcs_block.first_version,
        bcs_block.last_version,
        bcs_block.transactions,
    );

    // Convert transactions if present
    let txns = if let Some(txns_data) = transactions {
        let rendered: Vec<aptos_api_types::Transaction> = txns_data
            .into_iter()
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
            .collect::<Result<Vec<_>, _>>()?;
        Some(rendered)
    } else {
        None
    };

    let block_height: u64 = ledger_info.block_height.into();

    Ok(Block {
        block_height: block_height.into(),
        block_hash: block_hash.into(),
        block_timestamp: block_timestamp.into(),
        first_version: first_version.into(),
        last_version: last_version.into(),
        transactions: txns,
    })
}
