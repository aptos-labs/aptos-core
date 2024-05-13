// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::{block_fetch_manager::{BlockFetchContext, BlockFetchRequest}, BlockReader, BlockStore},
    logging::{LogEvent, LogSchema},
    monitor,
    network::NetworkSender,
};
use anyhow::bail;
use aptos_consensus_types::{
    block::Block,
    block_retrieval::{
        BlockRetrievalRequest, BlockRetrievalResponse, BlockRetrievalStatus, NUM_PEERS_PER_RETRY,
        NUM_RETRIES, RETRY_INTERVAL_MSEC, RPC_TIMEOUT_MSEC,
    },
    common::Author,
    quorum_cert::QuorumCert,
    sync_info::SyncInfo,
};
use aptos_crypto::HashValue;
use aptos_logger::prelude::*;
use aptos_types::account_address::AccountAddress;
use futures::{stream::FuturesUnordered, StreamExt};
use rand::{prelude::*, Rng};
use std::{clone::Clone, cmp::min, sync::Arc, time::Duration};
use tokio::time;
use aptos_channels::aptos_channel::Sender;


pub enum RetrieverResult {
    Blocks(Vec<Block>),
    Fetching,
}

/// Enum to identify how the blocks should be retrieved from the peers
#[derive(Debug, PartialEq, Eq)]
pub enum RetrieverMode {
    // This mode is used by recovery manager. The blocks are fetched in the same thread.
    // The recovery manager stops until the blocks are fetched.
    Synchronous,
    // This mode is used by round manager. The blocks are fetched in a separate thread.
    // The round manager will continue processing the next messages in the queue until
    // the blocks are fetched.
    Asynchronous
}

/// BlockRetriever is used internally to retrieve blocks
pub struct BlockRetriever {
    network: Arc<NetworkSender>,
    preferred_peer: Author,
    validator_addresses: Vec<AccountAddress>,
    max_blocks_to_request: u64,
    retriever_mode: RetrieverMode,
    block_fetch_request_tx: Option<Arc<Sender<(HashValue, HashValue), BlockFetchRequest>>>,
}

impl BlockRetriever {
    pub fn new(
        network: Arc<NetworkSender>,
        preferred_peer: Author,
        validator_addresses: Vec<AccountAddress>,
        max_blocks_to_request: u64,
        retriever_mode: RetrieverMode,
        block_fetch_request_tx: Option<Arc<Sender<(HashValue, HashValue), BlockFetchRequest>>>,
    ) -> Self {
        Self {
            network,
            preferred_peer,
            validator_addresses,
            max_blocks_to_request,
            retriever_mode,
            block_fetch_request_tx,
        }
    }

    pub fn network(&self) -> Arc<NetworkSender> {
        self.network.clone()
    }

    pub fn preferred_peer(&self) -> Author {
        self.preferred_peer
    }
   
    /// Retrieve chain of n blocks for given QC
    pub async fn retrieve_block_for_qc<'a>(
        &'a mut self,
        qc: &'a QuorumCert,
        num_blocks: u64,
        target_block_id: HashValue,
    ) -> anyhow::Result<RetrieverResult> {
        let peers = qc.ledger_info().get_voters(&self.validator_addresses);
        if self.retriever_mode == RetrieverMode::Synchronous {
            retrieve_block_for_id(
                self.network.clone(),
                qc.certified_block().id(),
                target_block_id,
                peers,
                self.preferred_peer,
                num_blocks,
                self.max_blocks_to_request,
            )
            .await
            .map(|blocks| RetrieverResult::Blocks(blocks))
        } else {
            // TODO: Send the request to the block fetch manager
            self.block_fetch_request_tx
                .expect("block_fetch_request_tx cannot be None when retriever mode is set to Asynchronous")
                .push((qc.certified_block().id(), target_block_id), 
                    BlockFetchRequest::new(
                        qc.certified_block().id(), 
                        target_block_id, 
                        self.preferred_peer, 
                        peers,
                        num_blocks,
                        // context
                    )
                );
            Ok(RetrieverResult::Fetching)
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
    network: Arc<NetworkSender>,
    block_id: HashValue,
    target_block_id: HashValue,
    peers: Vec<AccountAddress>,
    preferred_peer: AccountAddress,
    num_blocks: u64,
    max_blocks_to_request: u64,
) -> anyhow::Result<Vec<Block>> {
    info!(
        "Retrieving {} blocks starting from {}",
        num_blocks, block_id
    );
    let mut progress = 0;
    let mut last_block_id = block_id;
    let mut result_blocks: Vec<Block> = vec![];
    let mut retrieve_batch_size = max_blocks_to_request;
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

        let response = retrieve_block_for_id_chunk(
                network.clone(),
                last_block_id,
                target_block_id,
                retrieve_batch_size,
                peers.clone(),
                preferred_peer,
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
    network: Arc<NetworkSender>,
    block_id: HashValue,
    target_block_id: HashValue,
    retrieve_batch_size: u64,
    mut peers: Vec<AccountAddress>,
    preferred_peer: AccountAddress,
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
                        pick_peers(
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
                        let future = network.request_block(
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

fn pick_peer(first_atempt: bool, preferred_peer: AccountAddress, peers: &mut Vec<AccountAddress>) -> AccountAddress {
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
    first_atempt: bool,
    preferred_peer: AccountAddress,
    peers: &mut Vec<AccountAddress>,
    request_num_peers: usize,
) -> Vec<AccountAddress> {
    let mut result = Vec::new();
    while !peers.is_empty() && result.len() < request_num_peers {
        result.push(pick_peer(first_atempt && result.is_empty(), preferred_peer, peers));
    }
    result
}

// /// BlockRetriever is used internally to retrieve blocks
// pub struct BlockRetriever {
//     network: Arc<NetworkSender>,
//     preferred_peer: Author,
//     validator_addresses: Vec<AccountAddress>,
//     max_blocks_to_request: u64,
//     extra_block_store: HashMap<HashValue, Arc<Block>>,
//     fetch_context: BlockFetchContext,
//     block_fetch_request_tx: aptos_channel::Sender<BlockFetchContext, BlockFetchRequest>,
// }

// impl BlockRetriever {
//     pub fn new(
//         network: Arc<NetworkSender>,
//         preferred_peer: Author,
//         validator_addresses: Vec<AccountAddress>,
//         max_blocks_to_request: u64,
//         extra_block_store: HashMap<HashValue, Arc<Block>>,
//         fetch_context: BlockFetchContext,
//         block_fetch_request_tx: aptos_channel::Sender<BlockFetchContext, BlockFetchRequest>,
//     ) -> Self {
//         Self {
//             network,
//             preferred_peer,
//             validator_addresses,
//             max_blocks_to_request,
//             extra_block_store,
//             fetch_context,
//             block_fetch_request_tx
//         }
//     }

//     /// Retrieve chain of n blocks for given QC
//     async fn retrieve_block_for_qc<'a>(
//         &'a mut self,
//         qc: &'a QuorumCert,
//         num_blocks: u64,
//         target_block_id: HashValue,
//     ) -> anyhow::Result<RetrieverResult> {
//         let peers = qc.ledger_info().get_voters(&self.validator_addresses);
//         self.retrieve_block_for_id(
//             qc.certified_block().id(),
//             target_block_id,
//             peers,
//             num_blocks,
//         )
//         .await
//     }
// }
