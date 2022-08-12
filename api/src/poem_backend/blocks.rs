// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::accept_type::AcceptType;
use super::{
    ApiTags, BasicErrorWith404, BasicResponse, BasicResponseStatus, BasicResultWith404,
    InternalError,
};
use crate::context::Context;
use crate::failpoint::fail_point_poem;
use anyhow::Context as AnyhowContext;
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
        let latest_ledger_info = self.context.get_latest_ledger_info_poem()?;
        let latest_version = latest_ledger_info.version();
        let block = self
            .context
            .get_block_by_height(block_height, latest_version, with_transactions)
            .context("Failed to retrieve block by height")
            .map_err(BasicErrorWith404::internal)?;

        BasicResponse::try_from_rust_value((
            block,
            &latest_ledger_info,
            BasicResponseStatus::Ok,
            &accept_type,
        ))
    }

    fn get_by_version(
        &self,
        accept_type: AcceptType,
        version: u64,
        with_transactions: bool,
    ) -> BasicResultWith404<Block> {
        let latest_ledger_info = self.context.get_latest_ledger_info_poem()?;
        let latest_version = latest_ledger_info.version();
        let block = self
            .context
            .get_block_by_version(version, latest_version, with_transactions)
            .context("Failed to retrieve block by height")
            .map_err(BasicErrorWith404::internal)?;

        BasicResponse::try_from_rust_value((
            block,
            &latest_ledger_info,
            BasicResponseStatus::Ok,
            &accept_type,
        ))
    }
}
