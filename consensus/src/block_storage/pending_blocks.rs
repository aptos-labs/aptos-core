// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::counters::BLOCK_RETRIEVAL_LOCAL_FULFILL_COUNT;
use aptos_consensus_types::{block::Block, common::Round};
use aptos_logger::info;
use futures_channel::oneshot;
use std::collections::BTreeMap;

/// A local buffer to hold incoming blocks before it reaches round manager.
/// Which can be used to fulfill block request from local due to out of order messages.
pub struct PendingBlocks {
    blocks_by_round: BTreeMap<Round, Block>,
    pending_request: Option<(Round, oneshot::Sender<Block>)>,
}

impl PendingBlocks {
    pub fn new() -> Self {
        Self {
            blocks_by_round: BTreeMap::new(),
            pending_request: None,
        }
    }

    pub fn insert_block(&mut self, block: Block) {
        info!("Pending block inserted: {}", block.id());
        self.blocks_by_round.insert(block.round(), block.clone());
        if let Some((round, tx)) = self.pending_request.take() {
            if round == block.round() {
                info!(
                    "FulFill block request from incoming block for round: {}",
                    round
                );
                BLOCK_RETRIEVAL_LOCAL_FULFILL_COUNT.inc();
                tx.send(block).ok();
            } else {
                self.pending_request = Some((round, tx));
            }
        }
    }

    pub fn insert_request(&mut self, block_round: u64, tx: oneshot::Sender<Block>) {
        if let Some(block) = self.blocks_by_round.get(&block_round) {
            info!(
                "Fulfill block request from existing buffer: {}",
                block_round
            );
            BLOCK_RETRIEVAL_LOCAL_FULFILL_COUNT.inc();
            tx.send(block.clone()).ok();
        } else {
            info!("Insert block request for: {}", block_round);
            self.pending_request = Some((block_round, tx));
        }
    }

    pub fn gc(&mut self, round: Round) {
        self.blocks_by_round = self.blocks_by_round.split_off(&round);
    }
}
