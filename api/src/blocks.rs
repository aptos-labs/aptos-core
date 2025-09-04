// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    accept_type::AcceptType,
    context::{api_spawn_blocking, Context},
    failpoint::fail_point_poem,
    response::{BasicResponse, BasicResponseStatus, BasicResultWith404},
    ApiTags,
};
use velor_api_types::{BcsBlock, Block, LedgerInfo};
use poem_openapi::{
    param::{Path, Query},
    OpenApi,
};
use std::sync::Arc;

/// API for block transactions and information
#[derive(Clone)]
pub struct BlocksApi {
    pub context: Arc<Context>,
}

#[OpenApi]
impl BlocksApi {
    /// Get blocks by height
    ///
    /// This endpoint allows you to get the transactions in a block
    /// and the corresponding block information.
    ///
    /// Transactions are limited by max default transactions size.  If not all transactions
    /// are present, the user will need to query for the rest of the transactions via the
    /// get transactions API.
    ///
    /// If the block is pruned, it will return a 410
    #[oai(
        path = "/blocks/by_height/:block_height",
        method = "get",
        operation_id = "get_block_by_height",
        tag = "ApiTags::Blocks"
    )]
    async fn get_block_by_height(
        &self,
        accept_type: AcceptType,
        /// Block height to lookup.  Starts at 0
        block_height: Path<u64>,
        /// If set to true, include all transactions in the block
        ///
        /// If not provided, no transactions will be retrieved
        with_transactions: Query<Option<bool>>,
    ) -> BasicResultWith404<Block> {
        fail_point_poem("endpoint_get_block_by_height")?;
        self.context
            .check_api_output_enabled("Get block by height", &accept_type)?;
        let api = self.clone();
        api_spawn_blocking(move || {
            api.get_by_height(
                accept_type,
                block_height.0,
                with_transactions.0.unwrap_or_default(),
            )
        })
        .await
    }

    /// Get blocks by version
    ///
    /// This endpoint allows you to get the transactions in a block
    /// and the corresponding block information given a version in the block.
    ///
    /// Transactions are limited by max default transactions size.  If not all transactions
    /// are present, the user will need to query for the rest of the transactions via the
    /// get transactions API.
    ///
    /// If the block has been pruned, it will return a 410
    #[oai(
        path = "/blocks/by_version/:version",
        method = "get",
        operation_id = "get_block_by_version",
        tag = "ApiTags::Blocks"
    )]
    async fn get_block_by_version(
        &self,
        accept_type: AcceptType,
        /// Ledger version to lookup block information for.
        version: Path<u64>,
        /// If set to true, include all transactions in the block
        ///
        /// If not provided, no transactions will be retrieved
        with_transactions: Query<Option<bool>>,
    ) -> BasicResultWith404<Block> {
        fail_point_poem("endpoint_get_block_by_version")?;
        self.context
            .check_api_output_enabled("Get block by version", &accept_type)?;
        let api = self.clone();
        api_spawn_blocking(move || {
            api.get_by_version(
                accept_type,
                version.0,
                with_transactions.0.unwrap_or_default(),
            )
        })
        .await
    }
}

impl BlocksApi {
    fn get_by_height(
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

    fn get_by_version(
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
