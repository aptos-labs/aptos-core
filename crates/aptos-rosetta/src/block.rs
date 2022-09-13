// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::{BlockHash, Y2K_MS};
use crate::{
    common::{
        check_network, get_block_index_from_request, get_timestamp, handle_request, with_context,
    },
    error::ApiResult,
    types::{Block, BlockIdentifier, BlockRequest, BlockResponse, Transaction},
    ApiError, RosettaContext,
};
use aptos_logger::{debug, trace};
use aptos_rest_client::aptos_api_types::BcsBlock;
use aptos_types::chain_id::ChainId;
use std::sync::Arc;
use warp::Filter;

pub fn block_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("block")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(block))
}

/// Retrieves a block (in this case a single transaction) given it's identifier.
///
/// Our implementation allows for by `index`, which is the ledger `version` or by
/// transaction `hash`.
///
/// [API Spec](https://www.rosetta-api.org/docs/BlockApi.html#block)
async fn block(request: BlockRequest, server_context: RosettaContext) -> ApiResult<BlockResponse> {
    debug!("/block");
    trace!(
        request = ?request,
        server_context = ?server_context,
        "/block",
    );

    check_network(request.network_identifier, &server_context)?;

    // Retrieve by block or by hash, both or neither is not allowed
    let block_index =
        get_block_index_from_request(&server_context, request.block_identifier).await?;

    let (parent_transaction, block) = get_block_by_index(
        server_context.block_cache()?.as_ref(),
        block_index,
        server_context.chain_id,
    )
    .await?;

    let keep_empty_transactions = request
        .metadata
        .as_ref()
        .and_then(|inner| inner.keep_empty_transactions)
        .unwrap_or_default();
    let block = build_block(
        &server_context,
        parent_transaction,
        block,
        server_context.chain_id,
        keep_empty_transactions,
    )
    .await?;

    Ok(BlockResponse { block })
}

/// Build up the transaction, which should contain the `operations` as the change set
async fn build_block(
    server_context: &RosettaContext,
    parent_block_identifier: BlockIdentifier,
    block: aptos_rest_client::aptos_api_types::BcsBlock,
    chain_id: ChainId,
    keep_empty_transactions: bool,
) -> ApiResult<Block> {
    // note: timestamps are in microseconds, so we convert to milliseconds
    let timestamp = get_timestamp(block.block_timestamp);
    let block_identifier = BlockIdentifier::from_block(&block, chain_id);

    // Convert the transactions and build the block
    let mut transactions: Vec<Transaction> = Vec::new();
    // TODO: Parallelize these and then sort at end
    if let Some(txns) = block.transactions {
        for txn in txns {
            let transaction =
                Transaction::from_transaction(server_context.coin_cache.clone(), txn).await?;
            if keep_empty_transactions || !transaction.operations.is_empty() {
                transactions.push(transaction)
            }
        }
    }

    // Ensure the transactions are sorted in order
    transactions.sort_by(|first, second| first.metadata.version.0.cmp(&second.metadata.version.0));

    Ok(Block {
        block_identifier,
        parent_block_identifier,
        timestamp,
        transactions,
    })
}

/// Retrieves a block by its index
async fn get_block_by_index(
    block_cache: &BlockRetriever,
    block_height: u64,
    chain_id: ChainId,
) -> ApiResult<(
    BlockIdentifier,
    aptos_rest_client::aptos_api_types::BcsBlock,
)> {
    let block = block_cache.get_block_by_height(block_height, true).await?;

    // For the genesis block, we populate parent_block_identifier with the
    // same genesis block. Refer to
    // https://www.rosetta-api.org/docs/common_mistakes.html#malformed-genesis-block
    if block_height == 0 {
        Ok((BlockIdentifier::from_block(&block, chain_id), block))
    } else {
        // Retrieve the previous block's identifier
        let prev_block = block_cache
            .get_block_by_height(block_height - 1, false)
            .await?;
        let prev_block_id = BlockIdentifier::from_block(&prev_block, chain_id);

        // Retrieve the current block
        Ok((prev_block_id, block))
    }
}

#[derive(Clone, Debug)]
pub struct BlockInfo {
    /// Block identifier (block hash & block height)
    pub block_id: BlockIdentifier,
    /// Milliseconds timestamp
    pub timestamp: u64,
    /// Last version in block for getting state
    pub last_version: u64,
}

impl BlockInfo {
    pub fn from_block(
        block: &aptos_rest_client::aptos_api_types::BcsBlock,
        chain_id: ChainId,
    ) -> BlockInfo {
        BlockInfo {
            block_id: BlockIdentifier::from_block(block, chain_id),
            timestamp: get_timestamp(block.block_timestamp),
            last_version: block.last_version,
        }
    }
}

/// A cache of [`BlockInfo`] to allow us to keep track of the block boundaries
#[derive(Debug)]
pub struct BlockRetriever {
    rest_client: Arc<aptos_rest_client::Client>,
    block_size: Option<u16>,
}

impl BlockRetriever {
    pub fn new(rest_client: Arc<aptos_rest_client::Client>, block_size: Option<u16>) -> Self {
        BlockRetriever {
            rest_client,
            block_size,
        }
    }

    pub async fn get_block_info_by_height(
        &self,
        height: u64,
        chain_id: ChainId,
    ) -> ApiResult<BlockInfo> {
        // Genesis block is hardcoded
        if height == 0 {
            return Ok(BlockInfo {
                block_id: BlockIdentifier {
                    index: 0,
                    hash: BlockHash::new(chain_id, 0).to_string(),
                },
                timestamp: Y2K_MS,
                last_version: 0,
            });
        }

        let block = self.get_block_by_height(height, false).await?;
        Ok(BlockInfo::from_block(&block, chain_id))
    }

    pub async fn get_block_by_height(
        &self,
        height: u64,
        with_transactions: bool,
    ) -> ApiResult<aptos_rest_client::aptos_api_types::BcsBlock> {
        // Let's do some magic if there's a block size
        if let Some(block_size) = self.block_size {
            // Synthetic blocks have some problems around block time, and other pieces, but
            // they are a tradeoff of performance when blocks are mostly empty
            let first_version = block_size as u64 * height;
            let last_version = (block_size as u64 * (height + 1)) - 1;
            let mut block = self
                .rest_client
                .get_block_by_version_bcs(last_version, false)
                .await?
                .into_inner();

            // We have to hack the block into the correct bounds
            // Hash doesn't matter and timestamp is the last transaction's block
            block.first_version = first_version;
            block.last_version = last_version;
            block.block_height = height;

            // If we need the transactions, append them to the block info
            if with_transactions {
                let transactions = self
                    .rest_client
                    .get_transactions_bcs(Some(first_version), Some(block_size))
                    .await?
                    .into_inner();
                block.transactions = Some(transactions);
            }
            Ok(block)
        } else {
            Ok(self
                .rest_client
                .get_block_by_height_bcs(height, with_transactions)
                .await?
                .into_inner())
        }
    }
}
