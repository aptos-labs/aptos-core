// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::{collections::BTreeMap, fmt};

use anyhow::bail;
use aptos_consensus_types::{executed_block::ExecutedBlock, common::Round};
use aptos_types::{ledger_info::LedgerInfoWithSignatures, randomness::{Randomness, RandMetadata}};
use tokio::time::Instant;

use crate::{state_replication::StateComputerCommitCallBackType, experimental::commit_reliable_broadcast::DropGuard};

/// Sent from consensus to RandManager.
/// May contains a randomness if sent from DAG consensus.
pub struct OrderedBlocks {
    pub ordered_blocks: Vec<ExecutedBlock>,
    pub ordered_proof: LedgerInfoWithSignatures,
    pub callback: StateComputerCommitCallBackType,
    pub maybe_randomness: Option<Randomness>,
}

/// Sent from RandManager to BufferManager.
pub struct RandReadyBlocks {
    pub ordered_blocks: Vec<ExecutedBlock>,
    pub ordered_proof: LedgerInfoWithSignatures,
    pub callback: StateComputerCommitCallBackType,
}

pub struct BlockQueueItem {
    pub ordered_blocks: Vec<ExecutedBlock>,
    pub offsets_by_round: BTreeMap<Round, usize>,
    pub ordered_proof: LedgerInfoWithSignatures,
    pub callback: StateComputerCommitCallBackType,
    pub num_undecided_blocks: usize,
    pub timed_drop_guards: Vec<Option<(Instant, DropGuard)>>,
}

impl fmt::Debug for BlockQueueItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BlockQueueItem ")
    }
}

impl BlockQueueItem {
    pub fn num_blocks(&self) -> usize {
        self.ordered_blocks.len()
    }

    pub fn first_round(&self) -> u64 {
        self.ordered_blocks.first().unwrap().block().round()
    }

    pub fn offset(&self, round: Round) -> usize {
        *self.offsets_by_round.get(&round).unwrap()
    }
    pub fn update_drop_guard(&mut self, round: Round, drop_guard: DropGuard) {
        let offset = self.offset(round);
        self.timed_drop_guards[offset] = Some((Instant::now(), drop_guard));
    }

    pub fn rand_metadata(&self, round: Round) -> RandMetadata {
        let block = self.ordered_blocks[self.offset(round)].block();
        RandMetadata::new(block.epoch(), block.round(), block.id(), block.timestamp_usecs())
    }
}

// rand todo: Make it thread safe if needed
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
        self.queue.insert(item.first_round(), item);
    }

    // dequeue the rand ready prefix
    pub fn dequeue_rand_ready_prefix(&mut self) -> Vec<RandReadyBlocks> {
        let mut rand_ready_prefix = vec![];
        while let Some((_starting_round, item)) = self.queue.first_key_value() {
            if item.num_undecided_blocks == 0 {
                let (_, item) = self.queue.pop_first().unwrap();
                let BlockQueueItem {
                    ordered_blocks,
                    ordered_proof,
                    callback,
                    ..
                } = item;
                let rand_ready_blocks = RandReadyBlocks {
                    ordered_blocks,
                    ordered_proof,
                    callback,
                };
                rand_ready_prefix.push(rand_ready_blocks);
            } else {
                break;
            }
        }
        rand_ready_prefix
    }

    /// Return the `BlockQueueItem` that contains the given round, if exists.
    pub fn item_mut(&mut self, round: Round) -> Option<&mut BlockQueueItem> {
        self.queue.range_mut(0..=round).last()
            .map(|(_, item)| item)
            .filter(|item| item.offsets_by_round.contains_key(&round))
    }

    pub fn update_guard(&mut self, round: Round, drop_guard: DropGuard) {
        if let Some(item) = self.item_mut(round) {
            item.update_drop_guard(round, drop_guard);
        }
    }

    pub fn update_randomness(&mut self, round: Round, randomness: Randomness) -> anyhow::Result<()> {
        if let Some(item) = self.item_mut(round) {
            let offset = item.offset(round);
            assert!(item.ordered_blocks[offset].randomness().is_none());
            item.ordered_blocks[offset].set_randomness(randomness);
            item.num_undecided_blocks -= 1;
            Ok(())
        } else {
            bail!("[BlockQueue] update_decision failed: epoch {} round {} item not found", randomness.epoch(), round);
        }
    }
}
