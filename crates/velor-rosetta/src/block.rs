// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{
        check_network, get_block_index_from_request, get_timestamp, handle_request, with_context,
        BlockHash, Y2K_MS,
    },
    error::ApiResult,
    types::{Block, BlockIdentifier, BlockRequest, BlockResponse, Transaction},
    RosettaContext,
};
use velor_logger::{debug, trace};
use velor_types::chain_id::ChainId;
use std::sync::Arc;
use warp::Filter;

pub fn block_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("block")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(server_context))
        .and_then(handle_request(block))
}

/// Retrieves a block (in this case a single transaction) given it's identifier.
///
/// Our implementation allows for by `index`(block height) or by transaction `hash`.
/// If both are provided, `index` is used
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

    // Retrieve by block index or by hash, neither is not allowed
    let block_index =
        get_block_index_from_request(&server_context, request.block_identifier).await?;

    let (parent_transaction, block) = get_block_by_index(
        server_context.block_cache()?.as_ref(),
        block_index,
        server_context.chain_id,
    )
    .await?;

    // A hack to reduce overhead, if set, it will drop empty transactions (no operations0 from the
    // block to reduce traffic sent
    let keep_empty_transactions = request
        .metadata
        .as_ref()
        .and_then(|inner| inner.keep_empty_transactions)
        .unwrap_or_default();

    // Build the block accordingly from the input data
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
    block: velor_rest_client::velor_api_types::BcsBlock,
    chain_id: ChainId,
    keep_empty_transactions: bool,
) -> ApiResult<Block> {
    // NOTE: timestamps are in microseconds, so we convert to milliseconds for Rosetta
    let timestamp = get_timestamp(block.block_timestamp);
    let block_identifier = BlockIdentifier::from_block(&block, chain_id);

    // Convert the transactions and build the block
    let mut transactions: Vec<Transaction> = Vec::new();
    // TODO: Parallelize these and then sort at end
    if let Some(txns) = block.transactions {
        // Convert transactions to Rosetta format
        for txn in txns {
            let transaction = Transaction::from_transaction(server_context, txn).await?;

            // Skip transactions that don't have any operations, since that's the only thing that's being used by Rosetta
            if keep_empty_transactions || !transaction.operations.is_empty() {
                transactions.push(transaction)
            }
        }
    }

    // Ensure the transactions are sorted in order, this is required by Rosetta
    // NOTE: sorting may be pretty expensive, depending on the size of the block
    transactions.sort_by(|first, second| first.metadata.version.0.cmp(&second.metadata.version.0));

    Ok(Block {
        block_identifier,
        parent_block_identifier,
        timestamp,
        transactions,
    })
}

/// Retrieves a block by its index (block height)
async fn get_block_by_index(
    block_cache: &BlockRetriever,
    block_height: u64,
    chain_id: ChainId,
) -> ApiResult<(
    BlockIdentifier,
    velor_rest_client::velor_api_types::BcsBlock,
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

/// Abbreviated information about a Block without transactions
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
        block: &velor_rest_client::velor_api_types::BcsBlock,
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
    page_size: u16,
    rest_client: Arc<velor_rest_client::Client>,
}

impl BlockRetriever {
    pub fn new(page_size: u16, rest_client: Arc<velor_rest_client::Client>) -> Self {
        BlockRetriever {
            page_size,
            rest_client,
        }
    }

    /// Retrieves block abbreviated info by height
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

    /// Retrieves the block by height
    pub async fn get_block_by_height(
        &self,
        height: u64,
        with_transactions: bool,
    ) -> ApiResult<velor_rest_client::velor_api_types::BcsBlock> {
        // If we request transactions, we have to provide the page size, it ideally is bigger than
        // the maximum block size.  If not, transactions will be missed.
        if with_transactions {
            Ok(self
                .rest_client
                .get_full_block_by_height_bcs(height, self.page_size)
                .await?
                .into_inner())
        } else {
            Ok(self
                .rest_client
                .get_block_by_height_bcs(height, false)
                .await?
                .into_inner())
        }
    }
}
