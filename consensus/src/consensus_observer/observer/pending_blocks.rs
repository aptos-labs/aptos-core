// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::{
    common::{
        logging::{LogEntry, LogSchema},
        metrics,
    },
    network::observer_message::OrderedBlock,
    observer::payload_store::BlockPayloadStore,
};
use aptos_config::config::ConsensusObserverConfig;
use aptos_infallible::Mutex;
use aptos_logger::{info, warn};
use aptos_types::block_info::Round;
use std::{
    collections::{btree_map::Entry, BTreeMap},
    sync::Arc,
};

/// A simple struct to hold blocks that are waiting for payloads
pub struct PendingBlockStore {
    // The configuration of the consensus observer
    consensus_observer_config: ConsensusObserverConfig,

    // A map of ordered blocks that are without payloads. The key is
    // the (epoch, round) of the first block in the ordered block.
    blocks_without_payloads: BTreeMap<(u64, Round), OrderedBlock>,
}

impl PendingBlockStore {
    pub fn new(consensus_observer_config: ConsensusObserverConfig) -> Self {
        Self {
            consensus_observer_config,
            blocks_without_payloads: BTreeMap::new(),
        }
    }

    /// Clears all missing blocks from the store
    pub fn clear_missing_blocks(&mut self) {
        self.blocks_without_payloads.clear();
    }

    /// Returns true iff the store contains an entry for the given ordered block
    pub fn existing_pending_block(&self, ordered_block: &OrderedBlock) -> bool {
        // Get the epoch and round of the first block
        let first_block = ordered_block.first_block();
        let first_block_epoch_round = (first_block.epoch(), first_block.round());

        // Check if the block is already in the store
        self.blocks_without_payloads
            .contains_key(&first_block_epoch_round)
    }

    /// Inserts a block (without payloads) into the store
    pub fn insert_pending_block(&mut self, ordered_block: OrderedBlock) {
        // Get the epoch and round of the first block
        let first_block = ordered_block.first_block();
        let first_block_epoch_round = (first_block.epoch(), first_block.round());

        // Insert the block into the store using the round of the first block
        match self.blocks_without_payloads.entry(first_block_epoch_round) {
            Entry::Occupied(_) => {
                // The block is already in the store
                warn!(
                    LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                        "A pending block was already found for the given epoch and round: {:?}",
                        first_block_epoch_round
                    ))
                );
            },
            Entry::Vacant(entry) => {
                // Insert the block into the store
                entry.insert(ordered_block);
            },
        }

        // Perform garbage collection if the store is too large
        self.garbage_collect_pending_blocks();
    }

    /// Garbage collects the pending blocks store by removing
    /// the oldest blocks if the store is too large.
    fn garbage_collect_pending_blocks(&mut self) {
        // Calculate the number of blocks to remove
        let num_pending_blocks = self.blocks_without_payloads.len() as u64;
        let max_pending_blocks = self.consensus_observer_config.max_num_pending_blocks;
        let num_blocks_to_remove = num_pending_blocks.saturating_sub(max_pending_blocks);

        // Remove the oldest blocks if the store is too large
        for _ in 0..num_blocks_to_remove {
            if let Some((oldest_epoch_round, _)) = self.blocks_without_payloads.pop_first() {
                warn!(
                    LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                        "The pending block store is too large: {:?} blocks. Removing the block for the oldest epoch and round: {:?}",
                        num_pending_blocks, oldest_epoch_round
                    ))
                );
            }
        }
    }

    /// Removes and returns the block from the store that is now ready
    /// to be processed (after the new payload has been received).
    pub fn remove_ready_block(
        &mut self,
        received_payload_epoch: u64,
        received_payload_round: Round,
        block_payload_store: Arc<Mutex<BlockPayloadStore>>,
    ) -> Option<OrderedBlock> {
        // Calculate the round at which to split the blocks
        let split_round = received_payload_round.saturating_add(1);

        // Split the blocks at the epoch and round
        let mut blocks_at_higher_rounds = self
            .blocks_without_payloads
            .split_off(&(received_payload_epoch, split_round));

        // Check if the last block is ready (this should be the only ready block).
        // Any earlier blocks are considered out-of-date and will be dropped.
        let mut ready_block = None;
        if let Some((epoch_and_round, ordered_block)) = self.blocks_without_payloads.pop_last() {
            // If all payloads exist for the block, then the block is ready
            if block_payload_store
                .lock()
                .all_payloads_exist(ordered_block.blocks())
            {
                ready_block = Some(ordered_block);
            } else {
                // Otherwise, check if we're still waiting for higher payloads for the block
                if ordered_block.last_block().round() > received_payload_round {
                    blocks_at_higher_rounds.insert(epoch_and_round, ordered_block);
                }
            }
        }

        // Check if any out-of-date blocks were dropped
        if !self.blocks_without_payloads.is_empty() {
            info!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Dropped {:?} out-of-date pending blocks before epoch and round: {:?}",
                    self.blocks_without_payloads.len(),
                    (received_payload_epoch, received_payload_round)
                ))
            );
        }

        // Update the pending blocks to only include the blocks at higher rounds
        self.blocks_without_payloads = blocks_at_higher_rounds;

        // Return the ready block (if one exists)
        ready_block
    }

    /// Updates the metrics for the pending blocks
    pub fn update_pending_blocks_metrics(&self) {
        // Update the number of pending block entries
        let num_entries = self.blocks_without_payloads.len() as u64;
        metrics::set_gauge_with_label(
            &metrics::OBSERVER_NUM_PROCESSED_BLOCKS,
            metrics::PENDING_BLOCK_ENTRIES_LABEL,
            num_entries,
        );

        // Update the total number of pending blocks
        let num_pending_blocks = self
            .blocks_without_payloads
            .values()
            .map(|block| block.blocks().len() as u64)
            .sum();
        metrics::set_gauge_with_label(
            &metrics::OBSERVER_NUM_PROCESSED_BLOCKS,
            metrics::PENDING_BLOCKS_LABEL,
            num_pending_blocks,
        );

        // Update the highest round for the pending blocks
        let highest_pending_round = self
            .blocks_without_payloads
            .last_key_value()
            .map(|(_, pending_block)| pending_block.last_block().round())
            .unwrap_or(0);
        metrics::set_gauge_with_label(
            &metrics::OBSERVER_PROCESSED_BLOCK_ROUNDS,
            metrics::PENDING_BLOCKS_LABEL,
            highest_pending_round,
        );
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::consensus_observer::{
        network::observer_message::{BlockPayload, BlockTransactionPayload},
        observer::payload_store::BlockPayloadStore,
    };
    use aptos_consensus_types::{
        block::Block,
        block_data::{BlockData, BlockType},
        pipelined_block::PipelinedBlock,
        quorum_cert::QuorumCert,
    };
    use aptos_crypto::HashValue;
    use aptos_types::{
        aggregate_signature::AggregateSignature,
        block_info::BlockInfo,
        ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    };
    use rand::Rng;

    #[test]
    fn test_clear_missing_blocks() {
        // Create a new pending block store
        let max_num_pending_blocks = 10;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            ..ConsensusObserverConfig::default()
        };
        let pending_block_store = Arc::new(Mutex::new(PendingBlockStore::new(
            consensus_observer_config,
        )));

        // Insert the maximum number of blocks into the store
        let current_epoch = 0;
        let starting_round = 0;
        let missing_blocks = create_and_add_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks,
            current_epoch,
            starting_round,
            5,
        );

        // Verify that the store is not empty
        verify_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks,
            &missing_blocks,
        );

        // Clear the missing blocks from the store
        pending_block_store.lock().clear_missing_blocks();

        // Verify that the store is now empty
        assert!(pending_block_store
            .lock()
            .blocks_without_payloads
            .is_empty());
    }

    #[test]
    fn test_existing_pending_block() {
        // Create a new pending block store
        let max_num_pending_blocks = 10;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            ..ConsensusObserverConfig::default()
        };
        let pending_block_store = Arc::new(Mutex::new(PendingBlockStore::new(
            ConsensusObserverConfig::default(),
        )));

        // Insert the maximum number of blocks into the store
        let current_epoch = 10;
        let starting_round = 100;
        let pending_blocks = create_and_add_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks,
            current_epoch,
            starting_round,
            5,
        );

        // Verify that all blocks were inserted correctly
        for pending_block in &pending_blocks {
            assert!(pending_block_store
                .lock()
                .existing_pending_block(pending_block));
        }

        // Create a new block payload store and insert payloads for the second block
        let block_payload_store = Arc::new(Mutex::new(BlockPayloadStore::new(
            consensus_observer_config,
        )));
        let second_block = pending_blocks[1].clone();
        insert_payloads_for_ordered_block(block_payload_store.clone(), &second_block);

        // Remove the second block (which is now ready)
        let payload_round = second_block.first_block().round();
        let ready_block = pending_block_store.lock().remove_ready_block(
            current_epoch,
            payload_round,
            block_payload_store.clone(),
        );
        assert_eq!(ready_block, Some(second_block));

        // Verify that the first and second blocks were removed
        verify_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks - 2,
            &pending_blocks[2..].to_vec(),
        );

        // Verify that the first and second blocks are no longer in the store
        for pending_block in &pending_blocks[..2] {
            assert!(!pending_block_store
                .lock()
                .existing_pending_block(pending_block));
        }
    }

    #[test]
    fn test_insert_pending_block() {
        // Create a new pending block store
        let max_num_pending_blocks = 10;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            ..ConsensusObserverConfig::default()
        };
        let pending_block_store = Arc::new(Mutex::new(PendingBlockStore::new(
            consensus_observer_config,
        )));

        // Insert the maximum number of blocks into the store
        let current_epoch = 0;
        let starting_round = 0;
        let pending_blocks = create_and_add_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks,
            current_epoch,
            starting_round,
            5,
        );

        // Verify that all blocks were inserted correctly
        verify_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks,
            &pending_blocks,
        );

        // Insert the maximum number of blocks into the store again
        let starting_round = (max_num_pending_blocks * 100) as Round;
        let pending_blocks = create_and_add_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks,
            current_epoch,
            starting_round,
            5,
        );

        // Verify that all blocks were inserted correctly
        verify_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks,
            &pending_blocks,
        );

        // Insert one more block into the store (for the next epoch)
        let next_epoch = 1;
        let starting_round = 0;
        let new_pending_block = create_and_add_pending_blocks(
            pending_block_store.clone(),
            1,
            next_epoch,
            starting_round,
            5,
        );

        // Verify the new block was inserted correctly
        verify_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks,
            &new_pending_block,
        );
    }

    #[test]
    fn test_garbage_collect_pending_blocks() {
        // Create a new pending block store
        let max_num_pending_blocks = 100;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            ..ConsensusObserverConfig::default()
        };
        let pending_block_store = Arc::new(Mutex::new(PendingBlockStore::new(
            consensus_observer_config,
        )));

        // Insert the maximum number of blocks into the store
        let current_epoch = 0;
        let starting_round = 200;
        let mut pending_blocks = create_and_add_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks,
            current_epoch,
            starting_round,
            5,
        );

        // Verify that all blocks were inserted correctly
        verify_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks,
            &pending_blocks,
        );

        // Insert multiple blocks into the store (one at a time) and
        // verify that the oldest block is garbage collected each time.
        for i in 0..20 {
            // Insert one more block into the store
            let starting_round = ((max_num_pending_blocks * 10) + (i * 100)) as Round;
            let new_pending_block = create_and_add_pending_blocks(
                pending_block_store.clone(),
                1,
                current_epoch,
                starting_round,
                5,
            );

            // Verify the new block was inserted correctly
            verify_pending_blocks(
                pending_block_store.clone(),
                max_num_pending_blocks,
                &new_pending_block,
            );

            // Get the round of the oldest block (that was garbage collected)
            let oldest_block = pending_blocks.remove(0);
            let oldest_block_round = oldest_block.first_block().round();

            // Verify that the oldest block was garbage collected
            let blocks_without_payloads =
                pending_block_store.lock().blocks_without_payloads.clone();
            assert!(!blocks_without_payloads.contains_key(&(current_epoch, oldest_block_round)));
        }

        // Insert multiple blocks into the store (for the next epoch) and
        // verify that the oldest block is garbage collected each time.
        let next_epoch = 1;
        for i in 0..20 {
            // Insert one more block into the store
            let starting_round = i;
            let new_pending_block = create_and_add_pending_blocks(
                pending_block_store.clone(),
                1,
                next_epoch,
                starting_round,
                5,
            );

            // Verify the new block was inserted correctly
            verify_pending_blocks(
                pending_block_store.clone(),
                max_num_pending_blocks,
                &new_pending_block,
            );

            // Get the round of the oldest block (that was garbage collected)
            let oldest_block = pending_blocks.remove(0);
            let oldest_block_round = oldest_block.first_block().round();

            // Verify that the oldest block was garbage collected
            let blocks_without_payloads =
                pending_block_store.lock().blocks_without_payloads.clone();
            assert!(!blocks_without_payloads.contains_key(&(current_epoch, oldest_block_round)));
        }
    }

    #[test]
    fn test_remove_ready_block_multiple_blocks() {
        // Create a new pending block store
        let max_num_pending_blocks = 40;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            ..ConsensusObserverConfig::default()
        };
        let pending_block_store = Arc::new(Mutex::new(PendingBlockStore::new(
            consensus_observer_config,
        )));

        // Insert the maximum number of blocks into the store
        let current_epoch = 0;
        let starting_round = 0;
        let pending_blocks = create_and_add_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks,
            current_epoch,
            starting_round,
            5,
        );

        // Create a new block payload store and insert payloads for the second block
        let block_payload_store = Arc::new(Mutex::new(BlockPayloadStore::new(
            consensus_observer_config,
        )));
        let second_block = pending_blocks[1].clone();
        insert_payloads_for_ordered_block(block_payload_store.clone(), &second_block);

        // Remove the second block (which is now ready)
        let payload_round = second_block.first_block().round();
        let ready_block = pending_block_store.lock().remove_ready_block(
            current_epoch,
            payload_round,
            block_payload_store.clone(),
        );
        assert_eq!(ready_block, Some(second_block));

        // Verify that the first and second blocks were removed
        verify_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks - 2,
            &pending_blocks[2..].to_vec(),
        );

        // Insert payloads for the last block
        let last_block = pending_blocks.last().unwrap().clone();
        insert_payloads_for_ordered_block(block_payload_store.clone(), &last_block);

        // Remove the last block (which is now ready)
        let payload_round = last_block.first_block().round();
        let ready_block = pending_block_store.lock().remove_ready_block(
            current_epoch,
            payload_round,
            block_payload_store.clone(),
        );

        // Verify that the last block was removed
        assert_eq!(ready_block, Some(last_block));

        // Verify that the store is empty
        verify_pending_blocks(pending_block_store.clone(), 0, &vec![]);
    }

    #[test]
    fn test_remove_ready_block_multiple_blocks_missing() {
        // Create a new pending block store
        let max_num_pending_blocks = 10;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            ..ConsensusObserverConfig::default()
        };
        let pending_block_store = Arc::new(Mutex::new(PendingBlockStore::new(
            consensus_observer_config,
        )));

        // Insert the maximum number of blocks into the store
        let current_epoch = 10;
        let starting_round = 100;
        let pending_blocks = create_and_add_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks,
            current_epoch,
            starting_round,
            5,
        );

        // Create an empty block payload store
        let block_payload_store = Arc::new(Mutex::new(BlockPayloadStore::new(
            consensus_observer_config,
        )));

        // Incrementally insert and process each payload for the first block
        let first_block = pending_blocks.first().unwrap().clone();
        for block in first_block.blocks().clone() {
            // Insert the block
            let block_payload =
                BlockPayload::new(block.block_info(), BlockTransactionPayload::empty());
            block_payload_store
                .lock()
                .insert_block_payload(block_payload, true);

            // Attempt to remove the block (which might not be ready)
            let payload_round = block.round();
            let ready_block = pending_block_store.lock().remove_ready_block(
                current_epoch,
                payload_round,
                block_payload_store.clone(),
            );

            // If the block is ready, verify that it was removed.
            // Otherwise, verify that the block still remains.
            if payload_round == first_block.last_block().round() {
                // The block should be ready
                assert_eq!(ready_block, Some(first_block.clone()));

                // Verify that the block was removed
                verify_pending_blocks(
                    pending_block_store.clone(),
                    max_num_pending_blocks - 1,
                    &pending_blocks[1..].to_vec(),
                );
            } else {
                // The block should not be ready
                assert!(ready_block.is_none());

                // Verify that the block still remains
                verify_pending_blocks(
                    pending_block_store.clone(),
                    max_num_pending_blocks,
                    &pending_blocks,
                );
            }
        }

        // Incrementally insert and process payloads for the last block (except one)
        let last_block = pending_blocks.last().unwrap().clone();
        for block in last_block.blocks().clone() {
            // Insert the block only if this is not the first block
            let payload_round = block.round();
            if payload_round != last_block.first_block().round() {
                let block_payload =
                    BlockPayload::new(block.block_info(), BlockTransactionPayload::empty());
                block_payload_store
                    .lock()
                    .insert_block_payload(block_payload, true);
            }

            // Attempt to remove the block (which might not be ready)
            let ready_block = pending_block_store.lock().remove_ready_block(
                current_epoch,
                payload_round,
                block_payload_store.clone(),
            );

            // The block should not be ready
            assert!(ready_block.is_none());

            // Verify that the block still remains or has been removed on the last insert
            if payload_round == last_block.last_block().round() {
                verify_pending_blocks(pending_block_store.clone(), 0, &vec![]);
            } else {
                verify_pending_blocks(pending_block_store.clone(), 1, &vec![last_block.clone()]);
            }
        }

        // Verify that the store is now empty
        verify_pending_blocks(pending_block_store.clone(), 0, &vec![]);
    }

    #[test]
    fn test_remove_ready_block_singular_blocks() {
        // Create a new pending block store
        let max_num_pending_blocks = 10;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            ..ConsensusObserverConfig::default()
        };
        let pending_block_store = Arc::new(Mutex::new(PendingBlockStore::new(
            consensus_observer_config,
        )));

        // Insert the maximum number of blocks into the store
        let current_epoch = 0;
        let starting_round = 0;
        let pending_blocks = create_and_add_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks,
            current_epoch,
            starting_round,
            1,
        );

        // Create a new block payload store and insert payloads for the first block
        let block_payload_store = Arc::new(Mutex::new(BlockPayloadStore::new(
            consensus_observer_config,
        )));
        let first_block = pending_blocks.first().unwrap().clone();
        insert_payloads_for_ordered_block(block_payload_store.clone(), &first_block);

        // Remove the first block (which is now ready)
        let payload_round = first_block.first_block().round();
        let ready_block = pending_block_store.lock().remove_ready_block(
            current_epoch,
            payload_round,
            block_payload_store.clone(),
        );
        assert_eq!(ready_block, Some(first_block));

        // Verify that the first block was removed
        verify_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks - 1,
            &pending_blocks[1..].to_vec(),
        );

        // Insert payloads for the second block
        let second_block = pending_blocks[1].clone();
        insert_payloads_for_ordered_block(block_payload_store.clone(), &second_block);

        // Remove the second block (which is now ready)
        let payload_round = second_block.first_block().round();
        let ready_block = pending_block_store.lock().remove_ready_block(
            current_epoch,
            payload_round,
            block_payload_store.clone(),
        );
        assert_eq!(ready_block, Some(second_block));

        // Verify that the first and second blocks were removed
        verify_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks - 2,
            &pending_blocks[2..].to_vec(),
        );

        // Insert payloads for the last block
        let last_block = pending_blocks.last().unwrap().clone();
        insert_payloads_for_ordered_block(block_payload_store.clone(), &last_block);

        // Remove the last block (which is now ready)
        let payload_round = last_block.first_block().round();
        let ready_block = pending_block_store.lock().remove_ready_block(
            current_epoch,
            payload_round,
            block_payload_store.clone(),
        );

        // Verify that the last block was removed
        assert_eq!(ready_block, Some(last_block));

        // Verify that the store is empty
        verify_pending_blocks(pending_block_store.clone(), 0, &vec![]);
    }

    #[test]
    fn test_remove_ready_block_singular_blocks_missing() {
        // Create a new pending block store
        let max_num_pending_blocks = 100;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            ..ConsensusObserverConfig::default()
        };
        let pending_block_store = Arc::new(Mutex::new(PendingBlockStore::new(
            consensus_observer_config,
        )));

        // Insert the maximum number of blocks into the store
        let current_epoch = 10;
        let starting_round = 100;
        let pending_blocks = create_and_add_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks,
            current_epoch,
            starting_round,
            1,
        );

        // Create an empty block payload store
        let block_payload_store = Arc::new(Mutex::new(BlockPayloadStore::new(
            consensus_observer_config,
        )));

        // Remove the third block (which is not ready)
        let third_block = pending_blocks[2].clone();
        let third_block_round = third_block.first_block().round();
        let ready_block = pending_block_store.lock().remove_ready_block(
            current_epoch,
            third_block_round,
            block_payload_store.clone(),
        );
        assert!(ready_block.is_none());

        // Verify that the first three blocks were removed
        verify_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks - 3,
            &pending_blocks[3..].to_vec(),
        );

        // Remove the last block (which is not ready)
        let last_block = pending_blocks.last().unwrap().clone();
        let last_block_round = last_block.first_block().round();
        let ready_block = pending_block_store.lock().remove_ready_block(
            current_epoch,
            last_block_round,
            block_payload_store.clone(),
        );
        assert!(ready_block.is_none());

        // Verify that the store is now empty
        verify_pending_blocks(pending_block_store.clone(), 0, &vec![]);
    }

    /// Creates and adds the specified number of blocks to the pending block store
    fn create_and_add_pending_blocks(
        pending_block_store: Arc<Mutex<PendingBlockStore>>,
        num_pending_blocks: usize,
        epoch: u64,
        starting_round: Round,
        max_pipelined_blocks: u64,
    ) -> Vec<OrderedBlock> {
        let mut pending_blocks = vec![];
        for i in 0..num_pending_blocks {
            // Create the pipelined blocks
            let num_pipelined_blocks = rand::thread_rng().gen_range(1, max_pipelined_blocks + 1);
            let mut pipelined_blocks = vec![];
            for j in 0..num_pipelined_blocks {
                // Calculate the block round
                let round = starting_round + ((i as Round) * max_pipelined_blocks) + j; // Ensure gaps between blocks

                // Create a new block info
                let block_info = BlockInfo::new(
                    epoch,
                    round,
                    HashValue::random(),
                    HashValue::random(),
                    round,
                    i as u64,
                    None,
                );

                // Create the pipelined block
                let block_data = BlockData::new_for_testing(
                    block_info.epoch(),
                    block_info.round(),
                    block_info.timestamp_usecs(),
                    QuorumCert::dummy(),
                    BlockType::Genesis,
                );
                let block = Block::new_for_testing(block_info.id(), block_data, None);
                let pipelined_block = Arc::new(PipelinedBlock::new_ordered(block));

                // Add the pipelined block to the list
                pipelined_blocks.push(pipelined_block);
            }

            // Create an ordered block
            let ordered_proof = LedgerInfoWithSignatures::new(
                LedgerInfo::new(
                    BlockInfo::random_with_epoch(epoch, starting_round),
                    HashValue::random(),
                ),
                AggregateSignature::empty(),
            );
            let ordered_block = OrderedBlock::new(pipelined_blocks, ordered_proof.clone());

            // Insert the ordered block into the pending block store
            pending_block_store
                .lock()
                .insert_pending_block(ordered_block.clone());

            // Add the ordered block to the pending blocks
            pending_blocks.push(ordered_block);
        }

        pending_blocks
    }

    /// Inserts payloads into the payload store for the ordered block
    fn insert_payloads_for_ordered_block(
        block_payload_store: Arc<Mutex<BlockPayloadStore>>,
        ordered_block: &OrderedBlock,
    ) {
        for block in ordered_block.blocks() {
            let block_payload =
                BlockPayload::new(block.block_info(), BlockTransactionPayload::empty());
            block_payload_store
                .lock()
                .insert_block_payload(block_payload, true);
        }
    }

    /// Verifies that the pending block store contains the expected blocks
    fn verify_pending_blocks(
        pending_block_store: Arc<Mutex<PendingBlockStore>>,
        num_expected_blocks: usize,
        pending_blocks: &Vec<OrderedBlock>,
    ) {
        // Check the number of pending blocks
        assert_eq!(
            pending_block_store.lock().blocks_without_payloads.len(),
            num_expected_blocks
        );

        // Check that all pending blocks are in the store
        for pending_block in pending_blocks {
            let first_block = pending_block.first_block();
            assert_eq!(
                pending_block_store
                    .lock()
                    .blocks_without_payloads
                    .get(&(first_block.epoch(), first_block.round()))
                    .unwrap(),
                pending_block
            );
        }
    }
}
