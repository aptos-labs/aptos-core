// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::block_store::sync_manager::TargetBlockRetrieval,
    counters::BLOCK_RETRIEVAL_LOCAL_FULFILL_COUNT,
};
use velor_consensus_types::{block::Block, common::Round};
use velor_crypto::HashValue;
use velor_logger::info;
use futures_channel::oneshot;
use std::collections::{BTreeMap, HashMap};

/// A local buffer to hold incoming blocks before it reaches round manager.
/// Which can be used to fulfill block request from local due to out of order messages.
pub struct PendingBlocks {
    blocks_by_hash: HashMap<HashValue, Block>,
    blocks_by_round: BTreeMap<Round, Block>,
    pending_request: Option<(TargetBlockRetrieval, oneshot::Sender<Block>)>,
}

impl PendingBlocks {
    pub fn new() -> Self {
        Self {
            blocks_by_hash: HashMap::new(),
            blocks_by_round: BTreeMap::new(),
            pending_request: None,
        }
    }

    pub fn insert_block(&mut self, block: Block) {
        info!("Pending block inserted: {}", block.id());
        self.blocks_by_hash.insert(block.id(), block.clone());
        self.blocks_by_round.insert(block.round(), block.clone());
        if let Some((target_block_retrieval_payload, tx)) = self.pending_request.take() {
            let is_fulfilled = match target_block_retrieval_payload {
                TargetBlockRetrieval::TargetBlockId(target_block_id) => {
                    target_block_id == block.id()
                },
                TargetBlockRetrieval::TargetRound(target_round) => target_round == block.round(),
            };

            if is_fulfilled {
                info!(
                    "FulFill block request from incoming block: {}",
                    target_block_retrieval_payload
                );
                BLOCK_RETRIEVAL_LOCAL_FULFILL_COUNT.inc();
                tx.send(block).ok();
            } else {
                self.pending_request = Some((target_block_retrieval_payload, tx));
            }
        }
    }

    pub fn insert_request(
        &mut self,
        target_block_retrieval_payload: TargetBlockRetrieval,
        tx: oneshot::Sender<Block>,
    ) {
        match target_block_retrieval_payload {
            TargetBlockRetrieval::TargetBlockId(target_block_id) => {
                if let Some(block) = self.blocks_by_hash.get(&target_block_id) {
                    info!(
                        "FulFill block request from existing buffer: {}",
                        target_block_id
                    );
                    BLOCK_RETRIEVAL_LOCAL_FULFILL_COUNT.inc();
                    tx.send(block.clone()).ok();
                } else {
                    info!("Insert block request for: {}", target_block_id);
                    self.pending_request = Some((target_block_retrieval_payload, tx));
                }
            },
            TargetBlockRetrieval::TargetRound(target_round) => {
                if let Some(block) = self.blocks_by_round.get(&target_round) {
                    info!(
                        "Fulfill block request from existing buffer: {}",
                        target_round
                    );
                    BLOCK_RETRIEVAL_LOCAL_FULFILL_COUNT.inc();
                    tx.send(block.clone()).ok();
                } else {
                    info!("Insert block request for: {}", target_round);
                    self.pending_request = Some((target_block_retrieval_payload, tx));
                }
            },
        }
    }

    pub fn gc(&mut self, round: Round) {
        let mut to_remove = vec![];
        for (r, _) in self.blocks_by_round.range(..=round) {
            to_remove.push(*r);
        }
        for r in to_remove {
            if let Some(block) = self.blocks_by_round.remove(&r) {
                self.blocks_by_hash.remove(&block.id());
            }
        }
    }
}
