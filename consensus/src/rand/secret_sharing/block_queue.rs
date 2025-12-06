// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    block_storage::tracing::{observe_block, BlockStage},
    pipeline::buffer_manager::OrderedBlocks,
};
use aptos_consensus_types::{common::Round, pipelined_block::PipelinedBlock};
use aptos_reliable_broadcast::DropGuard;
use aptos_types::secret_sharing::SecretShareKey;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};

/// Maintain the ordered blocks received from consensus and corresponding randomness
pub struct QueueItem {
    ordered_blocks: OrderedBlocks,
    offsets_by_round: HashMap<Round, usize>,
    num_undecided_blocks: usize,
    set_undecrypted_blocks: HashSet<Round>,
    broadcast_handle: Option<Vec<DropGuard>>,
}

impl QueueItem {
    pub fn new(
        ordered_blocks: OrderedBlocks,
        broadcast_handle: Option<Vec<DropGuard>>,
        set_undecrypted_blocks: HashSet<Round>,
    ) -> Self {
        let len = ordered_blocks.ordered_blocks.len();
        assert!(len > 0);
        let offsets_by_round: HashMap<Round, usize> = ordered_blocks
            .ordered_blocks
            .iter()
            .enumerate()
            .map(|(idx, b)| (b.round(), idx))
            .collect();
        Self {
            ordered_blocks,
            offsets_by_round,
            num_undecided_blocks: len,
            broadcast_handle,
            set_undecrypted_blocks,
        }
    }

    pub fn num_blocks(&self) -> usize {
        self.blocks().len()
    }

    #[allow(clippy::unwrap_used)]
    pub fn first_round(&self) -> u64 {
        self.blocks().first().unwrap().block().round()
    }

    pub fn offset(&self, round: Round) -> usize {
        *self
            .offsets_by_round
            .get(&round)
            .expect("Round should be in the queue")
    }

    pub fn num_undecided(&self) -> usize {
        self.num_undecided_blocks
    }

    pub fn num_undecrypted(&self) -> usize {
        self.set_undecrypted_blocks.len()
    }

    pub fn set_dec_key(&mut self, round: Round, key: SecretShareKey) {
        let offset = self.offset(round);
        if self.set_undecrypted_blocks.contains(&round) {
            observe_block(
                self.blocks()[offset].timestamp_usecs(),
                BlockStage::SECRET_SHARING_ADD_DECISION,
            );
            let block = &self.blocks_mut()[offset];
            if let Some(tx) = block.pipeline_tx().lock().as_mut() {
                tx.secret_shared_key_tx.take().map(|tx| tx.send(Some(key)));
            }
            self.set_undecrypted_blocks.remove(&round);
        }
    }

    fn blocks(&self) -> &[Arc<PipelinedBlock>] {
        &self.ordered_blocks.ordered_blocks
    }

    fn blocks_mut(&mut self) -> &mut [Arc<PipelinedBlock>] {
        &mut self.ordered_blocks.ordered_blocks
    }

    fn get_block_by_round(&self, round: Round) -> Option<&Arc<PipelinedBlock>> {
        self.blocks().get(self.offset(round))
    }
}

/// Maintain ordered blocks that have pending randomness
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
            observe_block(block.timestamp_usecs(), BlockStage::RAND_ENTER);
        }
        assert!(self.queue.insert(item.first_round(), item).is_none());
    }

    /// Dequeue all ordered blocks prefix that have decryption key
    /// Unwrap is safe because the queue is not empty
    #[allow(clippy::unwrap_used)]
    pub fn dequeue_dec_ready_prefix(&mut self) -> Vec<OrderedBlocks> {
        let mut dec_ready_prefix = vec![];
        while let Some((_starting_round, item)) = self.queue.first_key_value() {
            if item.num_undecrypted() == 0 {
                let (_, item) = self.queue.pop_first().unwrap();
                for block in item.blocks() {
                    observe_block(block.timestamp_usecs(), BlockStage::SECRET_SHARING_READY);
                }
                let QueueItem { ordered_blocks, .. } = item;
                dec_ready_prefix.push(ordered_blocks);
            } else {
                break;
            }
        }
        dec_ready_prefix
    }

    /// Return the `QueueItem` that contains the given round, if exists.
    pub fn item_mut(&mut self, round: Round) -> Option<&mut QueueItem> {
        self.queue
            .range_mut(0..=round)
            .last()
            .map(|(_, item)| item)
            .filter(|item| item.offsets_by_round.contains_key(&round))
    }

    pub fn item(&self, round: Round) -> Option<&QueueItem> {
        self.queue
            .range(0..=round)
            .last()
            .map(|(_, item)| item)
            .filter(|item| item.offsets_by_round.contains_key(&round))
    }

    pub fn get_block_for_round(&self, round: Round) -> Option<&Arc<PipelinedBlock>> {
        self.item(round)
            .and_then(|item| item.get_block_by_round(round))
    }
}
