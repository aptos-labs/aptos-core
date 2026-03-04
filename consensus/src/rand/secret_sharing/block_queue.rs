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
    share_requester_handles: Vec<DropGuard>,
}

impl QueueItem {
    pub fn new(ordered_blocks: OrderedBlocks) -> Self {
        assert!(!ordered_blocks.ordered_blocks.is_empty());
        let offsets_by_round: HashMap<Round, usize> = ordered_blocks
            .ordered_blocks
            .iter()
            .enumerate()
            .map(|(idx, b)| (b.round(), idx))
            .collect();
        let pending_secret_key_rounds: HashSet<Round> = offsets_by_round.keys().copied().collect();
        Self {
            ordered_blocks,
            offsets_by_round,
            share_requester_handles: Vec::new(),
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

    pub fn push_share_requester_handle(&mut self, handle: DropGuard) {
        self.share_requester_handles.push(handle);
    }

    pub fn set_secret_shared_key(&mut self, round: Round, key: SecretSharedKey) {
        let offset = self.offset(round);
        // Guard against setting a key for an already-resolved round.
        if self.pending_secret_key_rounds.contains(&round) {
            observe_block(
                self.blocks()[offset].timestamp_usecs(),
                BlockStage::SECRET_SHARING_ADD_DECISION,
            );
            let block = &self.blocks_mut()[offset];
            block.set_decryption_key(key);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rand::{
        rand_gen::test_utils::create_ordered_blocks,
        secret_sharing::test_utils::{create_metadata, create_secret_shared_key, TestContext},
    };

    /// Helper: mark all rounds in a QueueItem as secret-shared so it becomes ready.
    fn mark_all_ready(ctx: &TestContext, item: &mut QueueItem) {
        let rounds: Vec<Round> = item.pending_secret_key_rounds.iter().copied().collect();
        for round in rounds {
            let metadata = create_metadata(ctx.epoch, round);
            let key = create_secret_shared_key(ctx, &metadata);
            item.set_secret_shared_key(round, key);
        }
    }

    #[test]
    fn test_queue_item_basic() {
        let blocks = create_ordered_blocks(vec![1, 2, 3]);
        let item = QueueItem::new(blocks);

        assert_eq!(item.first_round(), 1);
        assert_eq!(item.offset(1), 0);
        assert_eq!(item.offset(2), 1);
        assert_eq!(item.offset(3), 2);
        // All block rounds are pending secret sharing
        assert!(!item.is_fully_secret_shared());
        assert_eq!(item.pending_secret_key_rounds.len(), 3);
    }

    #[test]
    fn test_queue_item_pending_rounds_match_blocks() {
        // Verify that new() populates pending_secret_key_rounds from block rounds
        let blocks = create_ordered_blocks(vec![5, 6]);
        let item = QueueItem::new(blocks);
        assert_eq!(item.pending_secret_key_rounds, HashSet::from([5, 6]));
        assert!(!item.is_fully_secret_shared());
    }

    #[test]
    fn test_queue_item_set_secret_shared_key() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let blocks = create_ordered_blocks(vec![10]);
        let mut item = QueueItem::new(blocks);

        assert!(!item.is_fully_secret_shared());

        let metadata = create_metadata(ctx.epoch, 10);
        let key = create_secret_shared_key(&ctx, &metadata);
        item.set_secret_shared_key(10, key);

        assert!(item.is_fully_secret_shared());
    }

    #[test]
    fn test_block_queue_push_and_dequeue() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let mut queue = BlockQueue::new();

        // Push an item and mark all rounds ready
        let blocks = create_ordered_blocks(vec![1, 2]);
        let mut item = QueueItem::new(blocks);
        mark_all_ready(&ctx, &mut item);
        queue.push_back(item);

        let ready = queue.dequeue_ready_prefix();
        assert_eq!(ready.len(), 1);
        assert!(queue.queue().is_empty());
    }

    #[test]
    fn test_block_queue_dequeue_prefix_only() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let mut queue = BlockQueue::new();

        // First item: ready
        let blocks1 = create_ordered_blocks(vec![1, 2]);
        let mut item1 = QueueItem::new(blocks1);
        mark_all_ready(&ctx, &mut item1);
        queue.push_back(item1);

        // Second item: NOT ready (pending round 3)
        let blocks2 = create_ordered_blocks(vec![3]);
        let item2 = QueueItem::new(blocks2);
        queue.push_back(item2);

        // Third item: ready
        let blocks3 = create_ordered_blocks(vec![5, 6]);
        let mut item3 = QueueItem::new(blocks3);
        mark_all_ready(&ctx, &mut item3);
        queue.push_back(item3);

        // Only first item should dequeue (second blocks third)
        let ready = queue.dequeue_ready_prefix();
        assert_eq!(ready.len(), 1);
        assert_eq!(queue.queue().len(), 2);

        // Mark second item as ready
        let metadata = create_metadata(ctx.epoch, 3);
        let key = create_secret_shared_key(&ctx, &metadata);
        queue.item_mut(3).unwrap().set_secret_shared_key(3, key);

        // Now both remaining items should dequeue
        let ready = queue.dequeue_ready_prefix();
        assert_eq!(ready.len(), 2);
        assert!(queue.queue().is_empty());
    }

    #[test]
    fn test_block_queue_item_mut() {
        let mut queue = BlockQueue::new();

        let blocks = create_ordered_blocks(vec![10, 11, 12]);
        let item = QueueItem::new(blocks);
        queue.push_back(item);

        // Finds correct item by round
        assert!(queue.item_mut(10).is_some());
        assert!(queue.item_mut(11).is_some());
        assert!(queue.item_mut(12).is_some());

        // None for gap / non-existent rounds
        assert!(queue.item_mut(5).is_none());
        assert!(queue.item_mut(13).is_none());
    }
}
