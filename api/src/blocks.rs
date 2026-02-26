// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    accept_type::AcceptType,
    context::Context,
    response_axum::{AptosErrorResponse, AptosResponse},
};
use aptos_api_types::Block;
use std::sync::Arc;

/// Framework-agnostic business logic for the get block by height endpoint.
pub fn get_block_by_height_inner(
    context: &Arc<Context>,
    accept_type: &AcceptType,
    block_height: u64,
    with_transactions: bool,
) -> Result<AptosResponse<Block>, AptosErrorResponse> {
    let latest_ledger_info = context.get_latest_ledger_info::<AptosErrorResponse>()?;
    let bcs_block = context.get_block_by_height::<AptosErrorResponse>(
        block_height,
        &latest_ledger_info,
        with_transactions,
    )?;

    match accept_type {
        AcceptType::Json => {
            let transactions = if let Some(inner) = bcs_block.transactions {
                Some(
                    context.render_transactions_sequential::<AptosErrorResponse>(
                        &latest_ledger_info,
                        inner,
                        bcs_block.block_timestamp,
                    )?,
                )
            } else {
                None
            };
            let block = Block {
                block_height: bcs_block.block_height.into(),
                block_hash: bcs_block.block_hash.into(),
                block_timestamp: bcs_block.block_timestamp.into(),
                first_version: bcs_block.first_version.into(),
                last_version: bcs_block.last_version.into(),
                transactions,
            };
            AptosResponse::try_from_json(block, &latest_ledger_info)
        },
        AcceptType::Bcs => AptosResponse::try_from_bcs(bcs_block, &latest_ledger_info),
    }
}

/// Framework-agnostic business logic for the get block by version endpoint.
pub fn get_block_by_version_inner(
    context: &Arc<Context>,
    accept_type: &AcceptType,
    version: u64,
    with_transactions: bool,
) -> Result<AptosResponse<Block>, AptosErrorResponse> {
    let latest_ledger_info = context.get_latest_ledger_info::<AptosErrorResponse>()?;
    let bcs_block = context.get_block_by_version::<AptosErrorResponse>(
        version,
        &latest_ledger_info,
        with_transactions,
    )?;

    match accept_type {
        AcceptType::Json => {
            let transactions = if let Some(inner) = bcs_block.transactions {
                Some(
                    context.render_transactions_sequential::<AptosErrorResponse>(
                        &latest_ledger_info,
                        inner,
                        bcs_block.block_timestamp,
                    )?,
                )
            } else {
                None
            };
            let block = Block {
                block_height: bcs_block.block_height.into(),
                block_hash: bcs_block.block_hash.into(),
                block_timestamp: bcs_block.block_timestamp.into(),
                first_version: bcs_block.first_version.into(),
                last_version: bcs_block.last_version.into(),
                transactions,
            };
            AptosResponse::try_from_json(block, &latest_ledger_info)
        },
        AcceptType::Bcs => AptosResponse::try_from_bcs(bcs_block, &latest_ledger_info),
    }
}
