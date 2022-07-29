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
use aptos_logger::sample::SampleRate;
use aptos_logger::sample::Sampling;
use aptos_logger::{debug, sample, trace};
use aptos_rest_client::aptos_api_types::{BlockInfo, HashValue};
use std::cmp::Ordering;
use std::time::Duration;
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
        get_block_index_from_request(&server_context, request.block_identifier).await?;

    let (parent_transaction, block_info, transactions) = get_block_by_index(
        server_context.block_cache()?.as_ref(),
        &rest_client,
        block_index,
    )
    .await?;

    let block = build_block(parent_transaction, block_info, transactions).await?;

    Ok(BlockResponse {
        block: Some(block),
        other_transactions: None,
    })
}

/// Build up the transaction, which should contain the `operations` as the change set
async fn build_block(
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
        txns.push(Transaction::from_transaction(txn).await?)
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

/// Prune the BlockCache every 6 hours
/// TODO: Make configurable
const PRUNE_PERIOD_SECS: u64 = 21600;

/// A cache of [`BlockInfo`] to allow us to keep track of the block boundaries
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
        let genesis_block_info = BlockInfo {
            block_height: 0,
            block_hash: aptos_crypto::HashValue::zero().into(),
            block_timestamp: 0,
            start_version: 0,
            end_version: 0,
            num_transactions: 1,
        };
        blocks.insert(0, genesis_block_info);
        hashes.insert(genesis_block_info.block_hash, 0);
        versions.insert(0, 0);

        // Now insert the first and last blocks
        let state = rest_client.get_ledger_information().await?.into_inner();
        let block_cache = BlockCache {
            blocks: RwLock::new(blocks),
            hashes: RwLock::new(hashes),
            versions: RwLock::new(versions),
            rest_client,
        };
        if let Some(oldest_ledger_version) = state.oldest_ledger_version {
            block_cache.add_block(oldest_ledger_version).await?;
        }
        block_cache.add_block(state.version).await?;

        Ok(block_cache)
    }

    /// Prunes versions older than the oldest version
    pub async fn prune(&self) {
        if let Ok(Some(oldest_version)) = self
            .rest_client
            .get_ledger_information()
            .await
            .map(|response| response.into_inner().oldest_ledger_version)
        {
            // Check first if there's a version older than the oldest version
            {
                let versions = self.versions.read().unwrap();
                if versions.range(..oldest_version).next_back().is_none() {
                    return;
                }
            }

            // Prune based on version
            {
                // Lock all three in order that they're written
                let mut blocks = self.blocks.write().unwrap();
                let mut hashes = self.hashes.write().unwrap();
                let mut versions = self.versions.write().unwrap();

                // Grab all later versions after the oldest version (including it)
                let mut active = versions.split_off(&oldest_version);

                // All remaining versions need to be cleared from the rest of the stores
                for (_, block_index) in versions.iter() {
                    if let Some(info) = blocks.remove(block_index) {
                        hashes.remove(&info.block_hash);
                    }
                }

                // Clear the versions cache and append the active versions
                versions.clear();
                versions.append(&mut active);
            }
        }
    }

    /// Retrieve the block info for the index
    pub async fn get_block_info(&self, block_index: u64) -> ApiResult<BlockInfo> {
        sample!(
            SampleRate::Duration(Duration::from_secs(PRUNE_PERIOD_SECS)),
            self.prune().await
        );

        // If we already have the block info, let's roll with it
        let (closest_below, closest_above) = {
            let map = self.blocks.read().unwrap();
            if let Some(block_info) = map.get(&block_index) {
                return Ok(*block_info);
            }

            // Find the location of the one above and the one below
            let mut closest_below = None;
            let mut closest_above = None;
            for (i, info) in map.iter() {
                if *i < block_index {
                    closest_below = Some(info);
                } else if *i > block_index {
                    closest_above = Some(info);
                    break;
                }
            }

            (closest_below.copied(), closest_above.copied())
        };

        let info = match (closest_below, closest_above) {
            (Some(info), None) => {
                // Search linearly up
                let mut info = info;
                while info.block_height < block_index {
                    info = self.add_block(info.end_version.saturating_add(1)).await?;
                }

                info
            }
            (None, Some(info)) => {
                // Search linearly down, though this will likely be pruned
                let mut info = info;
                while info.block_height > block_index {
                    info = self.add_block(info.start_version.saturating_sub(1)).await?;
                }
                info
            }
            (Some(below), Some(above)) => {
                let mut below = below;
                let mut above = above;

                // Binary search for block
                while below.block_height < above.block_height.saturating_sub(1) {
                    let version = above
                        .start_version
                        .saturating_sub(below.end_version)
                        .saturating_div(2)
                        .saturating_add(below.end_version);
                    let info = self.add_block(version).await?;
                    match info.block_height.cmp(&block_index) {
                        Ordering::Less => below = info,
                        Ordering::Equal => return Ok(info),
                        Ordering::Greater => above = info,
                    }
                }
                return Err(ApiError::AptosError(Some(
                    "Failed to find block".to_string(),
                )));
            }
            _ => unreachable!(
                "Block cache is initialized with blocks, there should always be some other blocks"
            ),
        };

        if info.block_height != block_index {
            return Err(ApiError::AptosError(Some(
                "Failed to find block".to_string(),
            )));
        }
        Ok(info)
    }

    /// Retrieve block info, and add it to the index
    async fn add_block(&self, block_version: u64) -> ApiResult<BlockInfo> {
        let info_response = self.rest_client.get_block_info(block_version).await?;
        let info = info_response.into_inner();

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
            // The only versions that will be used are the start and end versions of a block
            // As those are the only versions that will be returned by ledger info
            map.insert(info.start_version, info.block_height);
            map.insert(info.end_version, info.block_height);
        }
        Ok(info)
    }

    /// Retrieve the block index for the version
    /// TODO: We can search through the versions to find the closest version
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
            return Err(ApiError::AptosError(Some(
                "Can't find block by block hash".to_string(),
            )));
        }
    }
}
