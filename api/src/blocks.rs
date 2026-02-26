// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    accept_type::AcceptType,
    context::Context,
    response::{BasicResponse, BasicResponseStatus, BasicResultWith404},
    response_axum::{AptosErrorResponse, AptosResponse},
};
use aptos_api_types::{BcsBlock, Block, LedgerInfo};
use std::sync::Arc;

/// API for block transactions and information
#[derive(Clone)]
pub struct BlocksApi {
    pub context: Arc<Context>,
}

impl BlocksApi {
    pub(crate) fn get_by_height(
        &self,
        accept_type: AcceptType,
        block_height: u64,
        with_transactions: bool,
    ) -> BasicResultWith404<Block> {
        let latest_ledger_info = self.context.get_latest_ledger_info()?;
        let bcs_block = self.context.get_block_by_height(
            block_height,
            &latest_ledger_info,
            with_transactions,
        )?;

        self.render_bcs_block(&accept_type, latest_ledger_info, bcs_block)
    }

    pub(crate) fn get_by_version(
        &self,
        accept_type: AcceptType,
        version: u64,
        with_transactions: bool,
    ) -> BasicResultWith404<Block> {
        let latest_ledger_info = self.context.get_latest_ledger_info()?;
        let bcs_block =
            self.context
                .get_block_by_version(version, &latest_ledger_info, with_transactions)?;

        self.render_bcs_block(&accept_type, latest_ledger_info, bcs_block)
    }

    /// Renders a [`BcsBlock`] into a [`Block`] if it's a JSON accept type
    fn render_bcs_block(
        &self,
        accept_type: &AcceptType,
        latest_ledger_info: LedgerInfo,
        bcs_block: BcsBlock,
    ) -> BasicResultWith404<Block> {
        match accept_type {
            AcceptType::Json => {
                let transactions = if let Some(inner) = bcs_block.transactions {
                    Some(self.context.render_transactions_sequential(
                        &latest_ledger_info,
                        inner,
                        bcs_block.block_timestamp,
                    )?)
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
                BasicResponse::try_from_json((block, &latest_ledger_info, BasicResponseStatus::Ok))
            },
            AcceptType::Bcs => BasicResponse::try_from_bcs((
                bcs_block,
                &latest_ledger_info,
                BasicResponseStatus::Ok,
            )),
        }
    }
}

/// Framework-agnostic business logic for the get block by height endpoint.
/// Called by the Axum handler directly, bypassing the Poem bridge.
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
/// Called by the Axum handler directly, bypassing the Poem bridge.
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
