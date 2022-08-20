// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::accept_type::AcceptType;
use crate::context::Context;
use crate::failpoint::fail_point_poem;
use crate::response::BasicResultWith404;
use crate::ApiTags;
use aptos_api_types::Block;
use poem_openapi::param::{Path, Query};
use poem_openapi::OpenApi;
use std::sync::Arc;

pub struct BlocksApi {
    pub context: Arc<Context>,
}

#[OpenApi]
impl BlocksApi {
    /// Get blocks by height
    ///
    /// This endpoint allows you to get the transactions in a block
    /// and the corresponding block information.
    #[oai(
        path = "/blocks/by_height/:block_height",
        method = "get",
        operation_id = "get_block_by_height",
        tag = "ApiTags::Blocks"
    )]
    async fn get_block_by_height(
        &self,
        accept_type: AcceptType,
        block_height: Path<u64>,
        with_transactions: Query<Option<bool>>,
    ) -> BasicResultWith404<Block> {
        fail_point_poem("endpoint_get_block_by_height")?;
        self.get_by_height(
            accept_type,
            block_height.0,
            with_transactions.0.unwrap_or_default(),
        )
    }

    /// Get blocks by version
    ///
    /// This endpoint allows you to get the transactions in a block
    /// and the corresponding block information given a version in the block.
    #[oai(
        path = "/blocks/by_version/:version",
        method = "get",
        operation_id = "get_block_by_version",
        tag = "ApiTags::Blocks"
    )]
    async fn get_block_by_version(
        &self,
        accept_type: AcceptType,
        version: Path<u64>,
        with_transactions: Query<Option<bool>>,
    ) -> BasicResultWith404<Block> {
        fail_point_poem("endpoint_get_block_by_version")?;
        self.get_by_version(
            accept_type,
            version.0,
            with_transactions.0.unwrap_or_default(),
        )
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
        self.context.get_block_by_height(
            &accept_type,
            block_height,
            latest_ledger_info,
            with_transactions,
        )
    }

    fn get_by_version(
        &self,
        accept_type: AcceptType,
        version: u64,
        with_transactions: bool,
    ) -> BasicResultWith404<Block> {
        let latest_ledger_info = self.context.get_latest_ledger_info()?;
        self.context.get_block_by_version(
            &accept_type,
            version,
            latest_ledger_info,
            with_transactions,
        )
    }
}
