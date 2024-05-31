// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::counters::BLOCK_RETRIEVAL_LOCAL_FULFILL_COUNT;
use aptos_consensus_types::{block::Block, common::Round};
use aptos_crypto::HashValue;
use aptos_logger::info;
use futures_channel::oneshot;
use std::collections::{BTreeMap, HashMap};

/// A local buffer to hold incoming blocks before it reaches round manager.
/// Which can be used to fulfill block request from local due to out of order messages.
pub struct PendingBlocks {
    blocks_by_hash: HashMap<HashValue, Block>,
    blocks_by_round: BTreeMap<Round, Block>,
    pending_request: Option<(HashValue, oneshot::Sender<Block>)>,
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
        if let Some((id, tx)) = self.pending_request.take() {
            if id == block.id() {
                info!("FulFill block request from incoming block: {}", id);
                BLOCK_RETRIEVAL_LOCAL_FULFILL_COUNT.inc();
                tx.send(block).ok();
            } else {
                self.pending_request = Some((id, tx));
            }
        }
    }

    pub fn insert_request(&mut self, block_id: HashValue, tx: oneshot::Sender<Block>) {
        if let Some(block) = self.blocks_by_hash.get(&block_id) {
            info!("FulFill block request from existing buffer: {}", block_id);
            BLOCK_RETRIEVAL_LOCAL_FULFILL_COUNT.inc();
            tx.send(block.clone()).ok();
        } else {
            info!("Insert block request for: {}", block_id);
            self.pending_request = Some((block_id, tx));
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
