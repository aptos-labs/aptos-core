// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{
        check_network, get_block_index_from_request, get_timestamp, handle_request, with_context,
    },
    error::{ApiError, ApiResult},
    types::{Block, BlockIdentifier, BlockRequest, BlockResponse, Transaction},
    RosettaContext,
};
use aptos_logger::{debug, trace};
use aptos_rest_client::aptos_api_types::{BlockInfo, HashValue};
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};
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

    let rest_client = server_context.rest_client()?;

    // Retrieve by block or by hash, both or neither is not allowed
    let block_index =
        get_block_index_from_request(&server_context, Some(request.block_identifier)).await?;

    let (parent_transaction, block_info, transactions) = get_block_by_index(
        server_context.block_cache()?.as_ref(),
        &rest_client,
        block_index,
    )
    .await?;

    let block = build_block(server_context, parent_transaction, block_info, transactions).await?;

    Ok(BlockResponse {
        block: Some(block),
        other_transactions: None,
    })
}

/// Build up the transaction, which should contain the `operations` as the change set
async fn build_block(
    server_context: RosettaContext,
    parent_block_identifier: BlockIdentifier,
    block_info: BlockInfo,
    transactions: Vec<aptos_rest_client::Transaction>,
) -> ApiResult<Block> {
    // note: timestamps are in microseconds, so we convert to milliseconds
    let timestamp = get_timestamp(block_info);
    let block_identifier = BlockIdentifier::from_block_info(block_info);

    // Convert the transactions and build the block
    let mut txns: Vec<Transaction> = Vec::new();
    for txn in transactions {
        txns.push(
            Transaction::from_transaction(
                server_context.coin_cache.clone(),
                server_context.rest_client()?.as_ref(),
                txn,
            )
            .await?,
        )
    }

    Ok(Block {
        block_identifier,
        parent_block_identifier,
        timestamp,
        transactions: txns,
    })
}

/// Retrieves a block by its index
async fn get_block_by_index(
    block_cache: &BlockCache,
    rest_client: &aptos_rest_client::Client,
    block_index: u64,
) -> ApiResult<(
    BlockIdentifier,
    BlockInfo,
    Vec<aptos_rest_client::Transaction>,
)> {
    // For the genesis block, we populate parent_block_identifier with the
    // same genesis block. Refer to
    // https://www.rosetta-api.org/docs/common_mistakes.html#malformed-genesis-block
    if block_index == 0 {
        let block_info = block_cache.get_block_info(block_index).await?;
        let response = rest_client.get_transaction_by_version(0).await?;
        let txn = response.into_inner();
        Ok((
            BlockIdentifier::from_block_info(block_info),
            block_info,
            vec![txn],
        ))
    } else {
        // Retrieve the previous block's identifier
        let prev_block_info = block_cache.get_block_info(block_index - 1).await?;
        let prev_block = BlockIdentifier::from_block_info(prev_block_info);

        // Retrieve the current block
        let block_info = block_cache.get_block_info(block_index).await?;
        let txns = rest_client
            .get_transactions(
                Some(block_info.start_version),
                Some(block_info.num_transactions),
            )
            .await?
            .into_inner();
        Ok((prev_block, block_info, txns))
    }
}

#[derive(Debug)]
pub struct BlockCache {
    blocks: RwLock<BTreeMap<u64, BlockInfo>>,
    hashes: RwLock<BTreeMap<HashValue, u64>>,
    versions: RwLock<BTreeMap<u64, u64>>,
    rest_client: Arc<aptos_rest_client::Client>,
}

impl BlockCache {
    pub async fn new(rest_client: Arc<aptos_rest_client::Client>) -> ApiResult<Self> {
        let mut blocks = BTreeMap::new();
        let mut hashes = BTreeMap::new();
        let mut versions = BTreeMap::new();
        // Genesis is always index 0
        // TODO: Ensure that this won't fail if it's been pruned
        if let Some(genesis_block_info) = rest_client.get_block_info(0).await?.into_inner() {
            let hash = genesis_block_info.block_hash;
            blocks.insert(0, genesis_block_info);
            hashes.insert(hash, 0);
            versions.insert(0, 0);
        } else {
            return Err(ApiError::BlockIncomplete);
        }
        Ok(BlockCache {
            blocks: RwLock::new(blocks),
            hashes: RwLock::new(hashes),
            versions: RwLock::new(versions),
            rest_client,
        })
    }

    /// Retrieve the block info for the index
    ///
    /// TODO: Improve parallelism and performance
    pub async fn get_block_info(&self, block_index: u64) -> ApiResult<BlockInfo> {
        // If we already have the block info, let's roll with it
        let (closest_known_block, closest_block_info): (u64, BlockInfo) = {
            let map = self.blocks.read().unwrap();
            if let Some(block_info) = map.get(&block_index) {
                return Ok(*block_info);
            }

            // There will always be an index less than the index, since it starts with 0 set
            let (index, block_info) = map.iter().rev().find(|(i, _)| **i < block_index).unwrap();
            (*index, *block_info)
        };

        // Go through the blocks, and add them into the cache
        let mut running_version = closest_block_info.end_version + 1;
        for i in (closest_known_block + 1)..=block_index {
            let info = self.add_block(running_version).await?;

            // Increment to the next block
            running_version = info.end_version + 1;

            // If it's the end condition, let's return the info
            if i == block_index {
                return Ok(info);
            }
        }

        // If for some reason the block doesn't get found, retry with block incomplete
        Err(ApiError::BlockIncomplete)
    }

    /// Retrieve block info, and add it to the index
    async fn add_block(&self, block_version: u64) -> ApiResult<BlockInfo> {
        let info_response = self.rest_client.get_block_info(block_version).await?;
        let info = if let Some(info) = info_response.into_inner() {
            info
        } else {
            // If we can't find the boundaries, provide a retriable error since the block isn't ready
            return Err(ApiError::BlockIncomplete);
        };

        // Add into the cache (keeping the write lock short)
        let info = {
            let mut map = self.blocks.write().unwrap();
            map.insert(info.block_height, info);
            info
        };

        // Write hash to index mapping
        {
            let mut map = self.hashes.write().unwrap();
            map.insert(info.block_hash, info.block_height);
        }

        // Write version to index mapping
        {
            let mut map = self.versions.write().unwrap();
            map.insert(block_version, info.block_height);
        }
        Ok(info)
    }

    /// Retrieve the block index for the version
    ///
    /// TODO: Improve parallelism and performance
    pub async fn get_block_index_by_version(&self, version: u64) -> ApiResult<u64> {
        // If we already have the version, let's roll with it
        if let Some(index) = self.versions.read().unwrap().get(&version) {
            return Ok(*index);
        }

        // Lookup block info by version
        Ok(self.add_block(version).await?.block_height)
    }

    pub async fn get_block_info_by_version(&self, version: u64) -> ApiResult<BlockInfo> {
        // If we already have the version, let's roll with it
        let maybe_index = { self.versions.read().unwrap().get(&version).copied() };

        if let Some(index) = maybe_index {
            if let Some(info) = self.blocks.read().unwrap().get(&index) {
                return Ok(*info);
            }
        }

        // Lookup block info by version
        self.add_block(version).await
    }

    /// Retrieve the block info for the hash
    ///
    /// This is particularly bad, since there's no index on this value.  It can only be derived
    /// from the cache, otherwise it needs to fail immediately.  This cache will need to be saved
    /// somewhere for these purposes.
    ///
    /// We could use the BlockMetadata transaction's hash rather than the block hash as a hack,
    /// and that is always indexed
    ///
    /// TODO: Improve reliability
    pub async fn get_block_index_by_hash(&self, hash: &HashValue) -> ApiResult<u64> {
        if let Some(version) = self.hashes.read().unwrap().get(hash) {
            Ok(*version)
        } else {
            // If for some reason the block doesn't get found, retry with block incomplete
            Err(ApiError::BlockIncomplete)
        }
    }
}
