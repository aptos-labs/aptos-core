// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::{collections::BTreeMap, fmt};

use anyhow::bail;
use aptos_consensus_types::{executed_block::ExecutedBlock, common::Round};
use aptos_types::{ledger_info::LedgerInfoWithSignatures, randomness::{Randomness, RandMetadata}};
use tokio::time::Instant;

use crate::{state_replication::StateComputerCommitCallBackType, experimental::commit_reliable_broadcast::DropGuard};

pub struct OrderedBlocks {
    pub ordered_blocks: Vec<ExecutedBlock>,
    pub ordered_proof: LedgerInfoWithSignatures,
    pub callback: StateComputerCommitCallBackType,
    pub maybe_randomness: Option<Randomness>,
    pub timed_drop_guard: Option<(Instant, DropGuard)>,
}

pub struct RandReadyBlocks {
    pub ordered_blocks: Vec<ExecutedBlock>,
    pub ordered_proof: LedgerInfoWithSignatures,
    pub callback: StateComputerCommitCallBackType,
    pub randomness: Randomness,
}

pub enum BlockQueueItem {
    Ordered(Box<OrderedBlocks>),
    RandReady(Box<RandReadyBlocks>),
}

impl fmt::Debug for BlockQueueItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockQueueItem::Ordered(_) => write!(f, "Ordered "),
            BlockQueueItem::RandReady(_) => write!(f, "RandReady "),
        }
    }
}

impl BlockQueueItem {
    pub fn is_ordered(&self) -> bool {
        match self {
            BlockQueueItem::Ordered(_) => true,
            BlockQueueItem::RandReady(_) => false,
        }
    }

    pub fn round(&self) -> u64 {
        match self {
            BlockQueueItem::Ordered(ordered) => ordered.ordered_blocks.last().unwrap().block().round(),
            BlockQueueItem::RandReady(rand_ready) => rand_ready.ordered_blocks.last().unwrap().block().round(),
        }
    }

    pub fn update_drop_guard(&mut self, drop_guard: DropGuard) {
        match self {
            BlockQueueItem::Ordered(ordered) => ordered.timed_drop_guard = Some((Instant::now(), drop_guard)),
            BlockQueueItem::RandReady(_) => (),
        }    
    }
    
    pub fn rand_metadata(&self) -> RandMetadata {
        let block = match self {
            BlockQueueItem::Ordered(ordered) => ordered.ordered_blocks.last().unwrap().block(),
            BlockQueueItem::RandReady(rand_ready) => rand_ready.ordered_blocks.last().unwrap().block(),
        };
        RandMetadata::new(block.epoch(), block.round(), block.id(), block.timestamp_usecs())
    }
}

// Make it thread safe if needed
pub struct BlockQueue {
    queue: BTreeMap<Round, BlockQueueItem>,
}

impl BlockQueue {
    pub fn new() -> Self {
        Self {
            queue: BTreeMap::new(),
        }
    }

    pub fn queue(&self) -> &BTreeMap<Round, BlockQueueItem> {
        &self.queue
    }

    pub fn push_back(&mut self, item: BlockQueueItem) {
        // if let Some((round, _)) = self.queue.last_key_value() {
        //     // Round numbers must be consecutive after dag
        //     assert!(item.round() == *round + 1);
        // }
        self.queue.insert(item.round(), item);
    }

    // dequeue the rand ready prefix
    pub fn dequeue_rand_ready_prefix(&mut self) -> Vec<RandReadyBlocks> {
        let mut rand_ready_prefix = vec![];
        while let Some((_, item)) = self.queue.pop_first() {
            match item {
                BlockQueueItem::Ordered(_) => {
                    self.queue.insert(item.round(), item);
                    break;
                }
                BlockQueueItem::RandReady(rand_ready) => {
                    rand_ready_prefix.push(*rand_ready);
                }
            }
        }
        rand_ready_prefix
    }

    pub fn get_item_mut(&mut self, round: Round) -> Option<&mut BlockQueueItem> {
        self.queue.get_mut(&round)
    }

    pub fn take_ordered_item(&mut self, round: Round) -> Option<BlockQueueItem> {
        self.queue.remove(&round).filter(|item| item.is_ordered())
    }

    pub fn update_guard(&mut self, round: Round, drop_guard: DropGuard) {
        if let Some(item) = self.get_item_mut(round) {
            item.update_drop_guard(drop_guard);
        }
    }

    pub fn update_randomness(&mut self, round: Round, randomness: Randomness) -> anyhow::Result<()> {
        if let Some(item) = self.take_ordered_item(round) {
            if let BlockQueueItem::Ordered(ordered) = item {
                let rand_ready = RandReadyBlocks {
                    ordered_blocks: ordered.ordered_blocks,
                    ordered_proof: ordered.ordered_proof,
                    callback: ordered.callback,
                    randomness,
                };
                self.queue.insert(round, BlockQueueItem::RandReady(Box::new(rand_ready)));
                return Ok(());
            } else {
                bail!("[BlockQueue] update_decision failed: round {} item is not ordered", round);
            }
        } else {
            bail!("[BlockQueue] update_decision failed: round {} item not found", round);
        }
    }
}
