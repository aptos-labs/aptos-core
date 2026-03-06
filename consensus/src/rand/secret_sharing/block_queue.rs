// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    block_storage::tracing::{observe_block, BlockStage},
    pipeline::buffer_manager::OrderedBlocks,
};
use aptos_consensus_types::{common::Round, pipelined_block::PipelinedBlock};
use aptos_reliable_broadcast::DropGuard;
use aptos_types::secret_sharing::SecretSharedKey;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};

/// Maintain the ordered blocks received from consensus and corresponding secret shares
pub struct QueueItem {
    ordered_blocks: OrderedBlocks,
    offsets_by_round: HashMap<Round, usize>,
    pending_secret_key_rounds: HashSet<Round>,
    share_requester_handles: Option<Vec<DropGuard>>,
}

impl QueueItem {
    pub fn new(
        ordered_blocks: OrderedBlocks,
        share_requester_handles: Option<Vec<DropGuard>>,
        pending_secret_key_rounds: HashSet<Round>,
    ) -> Self {
        assert!(!ordered_blocks.ordered_blocks.is_empty());
        let offsets_by_round: HashMap<Round, usize> = ordered_blocks
            .ordered_blocks
            .iter()
            .enumerate()
            .map(|(idx, b)| (b.round(), idx))
            .collect();
        Self {
            ordered_blocks,
            offsets_by_round,
            share_requester_handles,
            pending_secret_key_rounds,
        }
    }

    pub fn first_round(&self) -> u64 {
        self.blocks()
            .first()
            .expect("Block vec cannot be empty")
            .block()
            .round()
    }

    pub fn offset(&self, round: Round) -> usize {
        *self
            .offsets_by_round
            .get(&round)
            .expect("Round should be in the queue")
    }

    pub fn is_fully_secret_shared(&self) -> bool {
        self.pending_secret_key_rounds.is_empty()
    }

    pub fn set_secret_shared_key(&mut self, round: Round, key: SecretSharedKey) {
        let offset = self.offset(round);
        // TODO(ibalajiarun): revisit the importance of this hashset
        if self.pending_secret_key_rounds.contains(&round) {
            observe_block(
                self.blocks()[offset].timestamp_usecs(),
                BlockStage::SECRET_SHARING_ADD_DECISION,
            );
            let block = &self.blocks_mut()[offset];
            if let Some(tx) = block.pipeline_tx().lock().as_mut() {
                tx.secret_shared_key_tx.take().map(|tx| tx.send(Some(key)));
            }
            self.pending_secret_key_rounds.remove(&round);
        }
    }

    fn blocks(&self) -> &[Arc<PipelinedBlock>] {
        &self.ordered_blocks.ordered_blocks
    }

    fn blocks_mut(&mut self) -> &mut [Arc<PipelinedBlock>] {
        &mut self.ordered_blocks.ordered_blocks
    }
}

/// Maintain ordered blocks that have pending secret shares
pub struct BlockQueue {
    queue: BTreeMap<Round, QueueItem>,
}

impl BlockQueue {
    pub fn new() -> Self {
        Self {
            queue: BTreeMap::new(),
        }
    }

    pub fn queue(&self) -> &BTreeMap<Round, QueueItem> {
        &self.queue
    }

    pub fn push_back(&mut self, item: QueueItem) {
        for block in item.blocks() {
            observe_block(block.timestamp_usecs(), BlockStage::SECRET_SHARING_ENTER);
        }
        assert!(self.queue.insert(item.first_round(), item).is_none());
    }

    /// Dequeue all ordered blocks prefix that have secret shared key
    pub fn dequeue_ready_prefix(&mut self) -> Vec<OrderedBlocks> {
        let mut ready_prefix = vec![];
        while let Some((_starting_round, item)) = self.queue.first_key_value() {
            if item.is_fully_secret_shared() {
                let (_, item) = self.queue.pop_first().expect("First key must exist");
                for block in item.blocks() {
                    observe_block(block.timestamp_usecs(), BlockStage::SECRET_SHARING_READY);
                }
                let QueueItem { ordered_blocks, .. } = item;
                ready_prefix.push(ordered_blocks);
            } else {
                break;
            }
        }
        ready_prefix
    }

    /// Return the `QueueItem` that contains the given round, if exists.
    pub fn item_mut(&mut self, round: Round) -> Option<&mut QueueItem> {
        self.queue
            .range_mut(0..=round)
            .last()
            .map(|(_, item)| item)
            .filter(|item| item.offsets_by_round.contains_key(&round))
    }
}
