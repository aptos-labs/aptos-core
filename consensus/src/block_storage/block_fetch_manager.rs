// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use aptos_consensus_types::{
    block::Block, block_retrieval::{
        BlockRetrievalRequest, BlockRetrievalResponse, BlockRetrievalStatus, NUM_PEERS_PER_RETRY,
        NUM_RETRIES, RETRY_INTERVAL_MSEC, RPC_TIMEOUT_MSEC,
    }, common::Author, quorum_cert::QuorumCert, sync_info::SyncInfo,
};
use aptos_types::account_address::AccountAddress;
use aptos_logger::prelude::*;
use anyhow::{bail, Context};
use crate::{block_storage::counters::BLOCK_FETCH_MANAGER_MAIN_LOOP, monitor, network::NetworkSender, logging::{LogEvent, LogSchema}};
use std::{collections::HashMap, cmp::{min, max}, sync::Arc, time::Duration};
use rand::{thread_rng, Rng};
use tokio::time;
use futures::{stream::FuturesUnordered, StreamExt};
use aptos_channels::aptos_channel::{Sender, Receiver};
use lru::LruCache;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BlockFetchContext {
    ProcessRegular(SyncInfo, Author),
    InsertQuorumCert(QuorumCert, Author),
}

#[derive(Debug)]
pub struct BlockFetchRequest {
    initial_block_id: HashValue,
    target_block_id: HashValue,
    preferred_peer: AccountAddress,
    peers: Vec<AccountAddress>,
    num_blocks: u64,
    context: BlockFetchContext,
}

impl BlockFetchRequest {
    pub fn new(
        initial_block_id: HashValue,
        target_block_id: HashValue,
        preferred_peer: AccountAddress,
        peers: Vec<AccountAddress>,
        num_blocks: u64,
        context: BlockFetchContext,
    ) -> Self {
        BlockFetchRequest {
            initial_block_id,
            target_block_id,
            preferred_peer,
            peers,
            num_blocks,
            context,
        }
    }
}

#[derive(Debug)]
pub struct BlockFetchResponse {
    // TODO: Check if using a Vector is better here.
    blocks: HashMap<HashValue, Arc<Block>>,
    context: BlockFetchContext,
}

impl BlockFetchResponse {
    pub fn blocks(&self) -> &HashMap<HashValue, Arc<Block>> {
        &self.blocks
    }

    pub fn context(&self) -> &BlockFetchContext {
        &self.context
    }
}


pub struct BlockFetchManager {
    network: Arc<NetworkSender>,
    // TODO: Consider using a cache
    // TODO: Store the fetched blocks in the block_store for reuse
    block_store: LruCache<HashValue, Arc<Block>>,
    max_blocks_per_request: u64,
    // The key for the queue is (initial_block_id, target_block_id) of the request
    response_sender: Sender<(HashValue, HashValue), BlockFetchResponse>,
}

impl BlockFetchManager {
    pub fn new(network: Arc<NetworkSender>, max_blocks_per_request: u64, response_sender: Sender<(HashValue, HashValue), BlockFetchResponse>) -> Self {
        BlockFetchManager {
            network,
            // TODO: Is storing 50 blocks enough even for recovery manager?
            block_store: LruCache::new(max(max_blocks_per_request as usize, 50)),
            max_blocks_per_request,
            response_sender,
        }
    }

    pub async fn start(
        mut self,
        mut proposal_rx: Receiver<(HashValue, HashValue), BlockFetchRequest>,
    ) {
        loop {
            let _timer = BLOCK_FETCH_MANAGER_MAIN_LOOP.start_timer();

            tokio::select! {
                Some(request) = proposal_rx.next() => monitor!("block_fetch_manager_handle_proposal", {
                    self.handle_fetch_request(request).await,
                })
            }
        }
    }

    async fn handle_fetch_request(&mut self, fetch_request: BlockFetchRequest) {
        let BlockFetchRequest {
            initial_block_id,
            target_block_id,
            preferred_peer,
            peers,
            num_blocks,
            context,
        } = fetch_request;
        let blocks = HashMap::new();
        let mut current_block_id = initial_block_id;
        while current_block_id != target_block_id {
            if let Some(block) = self.block_store.get(&current_block_id) {
                blocks.insert(block.id(), block.clone());
                current_block_id = block.parent_id();
            } else {
                break;
            }
        }
        if blocks.len() == num_blocks as usize {
            let fetch_response = BlockFetchResponse { blocks, context };
            self.response_sender.push(context, fetch_response);
            return;
        } else {
            // TODO: Optimize this. We are fetching between [target_block_id, current_block_id] even if 
            // some of these blocks could be present in block_store.
            let mut peers = peers;
            let peers = self.pick_peers(true, preferred_peer, &mut peers, 2);
            let blocks = self.retrieve_block_for_id(initial_block_id, target_block_id, preferred_peer, peers, num_blocks).await;
            for block in blocks {
                self.block_store.push(block.id(), block);
            }
            match blocks {
                Ok(blocks) => {
                    info!("Successfully fetched blocks: {:?}", blocks);
                    let fetch_response = BlockFetchResponse { blocks, context };
                },
                Err(e) => {
                    error!("Failed to fetch blocks: {:?}", e);
                }
            }
        }
    }

    /// Retrieve n blocks for given block_id from peers
    ///
    /// Returns Result with Vec that if succeeded. This method will
    /// continue until the quorum certificate members all fail to return the missing chain.
    ///
    /// The first attempt of block retrieval will always be sent to preferred_peer to allow the
    /// leader to drive quorum certificate creation The other peers from the quorum certificate
    /// will be randomly tried next.  If all members of the quorum certificate are exhausted, an
    /// error is returned
    async fn retrieve_block_for_id(
        &mut self,
        block_id: HashValue,
        target_block_id: HashValue,
        preferred_peer: AccountAddress,
        peers: Vec<AccountAddress>,
        num_blocks: u64,
    ) -> anyhow::Result<Vec<Block>> {
        info!(
            "Retrieving {} blocks starting from {}",
            num_blocks, block_id
        );
        let mut progress = 0;
        let mut last_block_id = block_id;
        let mut result_blocks: Vec<Block> = vec![];
        let mut retrieve_batch_size = self.max_blocks_per_request;
        if peers.is_empty() {
            bail!("Failed to fetch block {}: no peers available", block_id);
        }
        while progress < num_blocks {
            // in case this is the last retrieval
            retrieve_batch_size = min(retrieve_batch_size, num_blocks - progress);

            info!(
                "Retrieving chunk: {} blocks starting from {}, original start {}",
                retrieve_batch_size, last_block_id, block_id
            );

            let response = self
                .retrieve_block_for_id_chunk(
                    last_block_id,
                    target_block_id,
                    retrieve_batch_size,
                    preferred_peer,
                    peers.clone(),
                )
                .await;
            match response {
                Ok(result) if matches!(result.status(), BlockRetrievalStatus::Succeeded) => {
                    // extend the result blocks
                    let batch = result.blocks().clone();
                    progress += batch.len() as u64;
                    last_block_id = batch.last().unwrap().parent_id();
                    result_blocks.extend(batch);
                },
                Ok(result)
                    if matches!(result.status(), BlockRetrievalStatus::SucceededWithTarget) =>
                {
                    // if we found the target, end the loop
                    let batch = result.blocks().clone();
                    result_blocks.extend(batch);
                    break;
                },
                _e => {
                    bail!(
                        "Failed to fetch block {}, for original start {}",
                        last_block_id,
                        block_id,
                    );
                },
            }
        }
        assert_eq!(result_blocks.last().unwrap().id(), target_block_id);
        Ok(result_blocks)
    }

    async fn retrieve_block_for_id_chunk(
        &mut self,
        block_id: HashValue,
        target_block_id: HashValue,
        retrieve_batch_size: u64,
        preferred_peer: AccountAddress,
        mut peers: Vec<AccountAddress>,
    ) -> anyhow::Result<BlockRetrievalResponse> {
        let mut failed_attempt = 0_u32;
        let mut cur_retry = 0;

        let num_retries = NUM_RETRIES;
        let request_num_peers = NUM_PEERS_PER_RETRY;
        let retry_interval = Duration::from_millis(RETRY_INTERVAL_MSEC);
        let rpc_timeout = Duration::from_millis(RPC_TIMEOUT_MSEC);

        monitor!("retrieve_block_for_id_chunk", {
            let mut interval = time::interval(retry_interval);
            let mut futures = FuturesUnordered::new();
            let request = BlockRetrievalRequest::new_with_target_block_id(
                block_id,
                retrieve_batch_size,
                target_block_id,
            );
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // send batch request to a set of peers of size request_num_peers (or 1 for the first time)
                        let next_peers = if cur_retry < num_retries {
                            let first_atempt = cur_retry == 0;
                            cur_retry += 1;
                            self.pick_peers(
                                first_atempt,
                                preferred_peer,
                                &mut peers,
                                if first_atempt { 1 } else {request_num_peers}
                            )
                        } else {
                            Vec::new()
                        };

                        if next_peers.is_empty() && futures.is_empty() {
                            bail!("Couldn't fetch block")
                        }

                        for peer in next_peers {
                            debug!(
                                LogSchema::new(LogEvent::RetrieveBlock).remote_peer(peer),
                                block_id = block_id,
                                "Fetching {} blocks, retry {}, failed attempts {}",
                                retrieve_batch_size,
                                cur_retry,
                                failed_attempt
                            );
                            let remote_peer = peer;
                            let future = self.network.request_block(
                                request.clone(),
                                peer,
                                rpc_timeout,
                            );
                            futures.push(async move { (remote_peer, future.await) });
                        }
                    }
                    Some((peer, response)) = futures.next() => {
                        match response {
                            Ok(result) => return Ok(result),
                            e => {
                                warn!(
                                    remote_peer = peer,
                                    block_id = block_id,
                                    "{:?}, Failed to fetch block",
                                    e,
                                );
                                failed_attempt += 1;
                            },
                        }
                    },
                }
            }
        })
    }


    fn pick_peer(&self, first_atempt: bool, preferred_peer: AccountAddress, peers: &mut Vec<AccountAddress>) -> AccountAddress {
        assert!(!peers.is_empty(), "pick_peer on empty peer list");

        if first_atempt {
            // remove preferred_peer if its in list of peers
            // (strictly speaking it is not required to be there)
            for i in 0..peers.len() {
                if peers[i] == preferred_peer {
                    peers.remove(i);
                    break;
                }
            }
            return preferred_peer;
        }

        let peer_idx = thread_rng().gen_range(0, peers.len());
        peers.remove(peer_idx)
    }


    fn pick_peers(
        &self,
        first_atempt: bool,
        preferred_peer: AccountAddress,
        peers: &mut Vec<AccountAddress>,
        request_num_peers: usize,
    ) -> Vec<AccountAddress> {
        let mut result = Vec::new();
        while !peers.is_empty() && result.len() < request_num_peers {
            result.push(self.pick_peer(first_atempt && result.is_empty(), preferred_peer, peers));
        }
        result
    }
}