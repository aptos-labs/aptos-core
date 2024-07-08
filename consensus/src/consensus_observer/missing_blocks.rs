// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::{
    logging::{LogEntry, LogSchema},
    metrics,
    network_message::OrderedBlock,
    payload_store::BlockPayloadStore,
};
use aptos_config::config::ConsensusObserverConfig;
use aptos_infallible::Mutex;
use aptos_logger::warn;
use aptos_types::block_info::Round;
use std::{
    collections::{btree_map::Entry, BTreeMap},
    sync::Arc,
};

/// A simple struct to hold blocks that are missing payloads. This is useful to
/// handle out-of-order messages, where payloads are received after ordered blocks.
#[derive(Clone)]
pub struct MissingBlockStore {
    // The configuration of the consensus observer
    consensus_observer_config: ConsensusObserverConfig,

    // A map of ordered blocks that are missing payloads. The key is the
    // (epoch, round) of the first block in the ordered block.
    blocks_missing_payloads: Arc<Mutex<BTreeMap<(u64, Round), OrderedBlock>>>,
}

impl MissingBlockStore {
    pub fn new(consensus_observer_config: ConsensusObserverConfig) -> Self {
        Self {
            consensus_observer_config,
            blocks_missing_payloads: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Clears all missing blocks from the store
    pub fn clear_missing_blocks(&self) {
        self.blocks_missing_payloads.lock().clear();
    }

    /// Inserts a block (with missing payloads) into the store
    pub fn insert_missing_block(&self, ordered_block: OrderedBlock) {
        // Get the epoch and round of the first block
        let first_block = ordered_block.first_block();
        let first_block_epoch_round = (first_block.epoch(), first_block.round());

        // Insert the block into the store using the round of the first block
        match self
            .blocks_missing_payloads
            .lock()
            .entry(first_block_epoch_round)
        {
            Entry::Occupied(_) => {
                // The block is already in the store
                warn!(
                    LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                        "A missing block was already found for the given epoch and round: {:?}",
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
        self.garbage_collect_missing_blocks();
    }

    /// Garbage collects the missing blocks store by removing
    /// the oldest blocks if the store is too large.
    fn garbage_collect_missing_blocks(&self) {
        // Calculate the number of blocks to remove
        let mut blocks_missing_payloads = self.blocks_missing_payloads.lock();
        let num_missing_blocks = blocks_missing_payloads.len() as u64;
        let max_pending_blocks = self.consensus_observer_config.max_num_pending_blocks;
        let num_blocks_to_remove = num_missing_blocks.saturating_sub(max_pending_blocks);

        // Remove the oldest blocks if the store is too large
        for _ in 0..num_blocks_to_remove {
            if let Some((oldest_epoch_round, _)) = blocks_missing_payloads.pop_first() {
                warn!(
                    LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                        "The missing block store is too large: {:?} blocks. Removing the block for the oldest epoch and round: {:?}",
                        num_missing_blocks, oldest_epoch_round
                    ))
                );
            }
        }
    }

    /// Removes and returns the block from the store that is now ready
    /// to be processed (after the new payload has been received).
    pub fn remove_ready_block(
        &self,
        received_payload_epoch: u64,
        received_payload_round: Round,
        block_payload_store: &BlockPayloadStore,
    ) -> Option<OrderedBlock> {
        // Calculate the round at which to split the blocks
        let split_round = received_payload_round.saturating_add(1);

        // Split the missing blocks at the epoch and round
        let mut blocks_missing_payloads = self.blocks_missing_payloads.lock();
        let mut blocks_at_higher_rounds =
            blocks_missing_payloads.split_off(&(received_payload_epoch, split_round));

        // Check if the last block is ready (this should be the only ready block).
        // Any earlier blocks are considered out-of-date and will be dropped.
        let mut ready_block = None;
        if let Some((epoch_and_round, ordered_block)) = blocks_missing_payloads.pop_last() {
            // If all payloads exist for the block, then the block is ready
            if block_payload_store.all_payloads_exist(ordered_block.blocks()) {
                ready_block = Some(ordered_block);
            } else {
                // Otherwise, check if we're still waiting for higher payloads for the block
                if ordered_block.last_block().round() > received_payload_round {
                    blocks_at_higher_rounds.insert(epoch_and_round, ordered_block);
                }
            }
        }

        // Update the missing blocks to only include the blocks at higher rounds
        *blocks_missing_payloads = blocks_at_higher_rounds;

        // Return the ready block (if one exists)
        ready_block
    }

    /// Updates the metrics for the missing blocks
    pub fn update_missing_blocks_metrics(&self) {
        // Update the number of missing blocks
        let blocks_missing_payloads = self.blocks_missing_payloads.lock();
        let num_missing_blocks = blocks_missing_payloads.len() as u64;
        metrics::set_gauge_with_label(
            &metrics::OBSERVER_NUM_PROCESSED_BLOCKS,
            metrics::MISSING_BLOCKS_LABEL,
            num_missing_blocks,
        );

        // Update the highest round for the missing blocks
        let highest_missing_round = blocks_missing_payloads
            .last_key_value()
            .map(|(_, missing_block)| missing_block.last_block().round())
            .unwrap_or(0);
        metrics::set_gauge_with_label(
            &metrics::OBSERVER_PROCESSED_BLOCK_ROUNDS,
            metrics::MISSING_BLOCKS_LABEL,
            highest_missing_round,
        );
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::consensus_observer::network_message::{BlockPayload, BlockTransactionPayload};
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
        // Create a new missing block store
        let max_num_pending_blocks = 10;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            ..ConsensusObserverConfig::default()
        };
        let missing_block_store = MissingBlockStore::new(consensus_observer_config);

        // Insert the maximum number of blocks into the store
        let current_epoch = 0;
        let starting_round = 0;
        let missing_blocks = create_and_add_missing_blocks(
            &missing_block_store,
            max_num_pending_blocks,
            current_epoch,
            starting_round,
            5,
        );

        // Verify that the store is not empty
        verify_missing_blocks(
            &missing_block_store,
            max_num_pending_blocks,
            &missing_blocks,
        );

        // Clear the missing blocks from the store
        missing_block_store.clear_missing_blocks();

        // Verify that the store is now empty
        let blocks_missing_payloads = missing_block_store.blocks_missing_payloads.lock();
        assert!(blocks_missing_payloads.is_empty());
    }

    #[test]
    fn test_insert_missing_block() {
        // Create a new missing block store
        let max_num_pending_blocks = 10;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            ..ConsensusObserverConfig::default()
        };
        let missing_block_store = MissingBlockStore::new(consensus_observer_config);

        // Insert the maximum number of blocks into the store
        let current_epoch = 0;
        let starting_round = 0;
        let missing_blocks = create_and_add_missing_blocks(
            &missing_block_store,
            max_num_pending_blocks,
            current_epoch,
            starting_round,
            5,
        );

        // Verify that all blocks were inserted correctly
        verify_missing_blocks(
            &missing_block_store,
            max_num_pending_blocks,
            &missing_blocks,
        );

        // Insert the maximum number of blocks into the store again
        let starting_round = (max_num_pending_blocks * 100) as Round;
        let missing_blocks = create_and_add_missing_blocks(
            &missing_block_store,
            max_num_pending_blocks,
            current_epoch,
            starting_round,
            5,
        );

        // Verify that all blocks were inserted correctly
        verify_missing_blocks(
            &missing_block_store,
            max_num_pending_blocks,
            &missing_blocks,
        );

        // Insert one more block into the store (for the next epoch)
        let next_epoch = 1;
        let starting_round = 0;
        let new_missing_block =
            create_and_add_missing_blocks(&missing_block_store, 1, next_epoch, starting_round, 5);

        // Verify the new block was inserted correctly
        verify_missing_blocks(
            &missing_block_store,
            max_num_pending_blocks,
            &new_missing_block,
        );
    }

    #[test]
    fn test_garbage_collect_missing_blocks() {
        // Create a new missing block store
        let max_num_pending_blocks = 100;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            ..ConsensusObserverConfig::default()
        };
        let missing_block_store = MissingBlockStore::new(consensus_observer_config);

        // Insert the maximum number of blocks into the store
        let current_epoch = 0;
        let starting_round = 200;
        let mut missing_blocks = create_and_add_missing_blocks(
            &missing_block_store,
            max_num_pending_blocks,
            current_epoch,
            starting_round,
            5,
        );

        // Verify that all blocks were inserted correctly
        verify_missing_blocks(
            &missing_block_store,
            max_num_pending_blocks,
            &missing_blocks,
        );

        // Insert multiple blocks into the store (one at a time) and
        // verify that the oldest block is garbage collected each time.
        for i in 0..20 {
            // Insert one more block into the store
            let starting_round = ((max_num_pending_blocks * 10) + (i * 100)) as Round;
            let new_missing_block = create_and_add_missing_blocks(
                &missing_block_store,
                1,
                current_epoch,
                starting_round,
                5,
            );

            // Verify the new block was inserted correctly
            verify_missing_blocks(
                &missing_block_store,
                max_num_pending_blocks,
                &new_missing_block,
            );

            // Get the round of the oldest block (that was garbage collected)
            let oldest_block = missing_blocks.remove(0);
            let oldest_block_round = oldest_block.first_block().round();

            // Verify that the oldest block was garbage collected
            let blocks_missing_payloads = missing_block_store.blocks_missing_payloads.lock();
            assert!(!blocks_missing_payloads.contains_key(&(current_epoch, oldest_block_round)));
        }

        // Insert multiple blocks into the store (for the next epoch) and
        // verify that the oldest block is garbage collected each time.
        let next_epoch = 1;
        for i in 0..20 {
            // Insert one more block into the store
            let starting_round = i;
            let new_missing_block = create_and_add_missing_blocks(
                &missing_block_store,
                1,
                next_epoch,
                starting_round,
                5,
            );

            // Verify the new block was inserted correctly
            verify_missing_blocks(
                &missing_block_store,
                max_num_pending_blocks,
                &new_missing_block,
            );

            // Get the round of the oldest block (that was garbage collected)
            let oldest_block = missing_blocks.remove(0);
            let oldest_block_round = oldest_block.first_block().round();

            // Verify that the oldest block was garbage collected
            let blocks_missing_payloads = missing_block_store.blocks_missing_payloads.lock();
            assert!(!blocks_missing_payloads.contains_key(&(current_epoch, oldest_block_round)));
        }
    }

    #[test]
    fn test_remove_ready_block_multiple_blocks() {
        // Create a new missing block store
        let max_num_pending_blocks = 40;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            ..ConsensusObserverConfig::default()
        };
        let missing_block_store = MissingBlockStore::new(consensus_observer_config);

        // Insert the maximum number of blocks into the store
        let current_epoch = 0;
        let starting_round = 0;
        let missing_blocks = create_and_add_missing_blocks(
            &missing_block_store,
            max_num_pending_blocks,
            current_epoch,
            starting_round,
            5,
        );

        // Create a new block payload store and insert payloads for the second block
        let mut block_payload_store = BlockPayloadStore::new(consensus_observer_config);
        let second_block = missing_blocks[1].clone();
        insert_payloads_for_ordered_block(&mut block_payload_store, &second_block);

        // Remove the second block (which is now ready)
        let payload_round = second_block.first_block().round();
        let ready_block = missing_block_store.remove_ready_block(
            current_epoch,
            payload_round,
            &block_payload_store,
        );
        assert_eq!(ready_block, Some(second_block));

        // Verify that the first and second blocks were removed
        verify_missing_blocks(
            &missing_block_store,
            max_num_pending_blocks - 2,
            &missing_blocks[2..].to_vec(),
        );

        // Insert payloads for the last block
        let last_block = missing_blocks.last().unwrap().clone();
        insert_payloads_for_ordered_block(&mut block_payload_store, &last_block);

        // Remove the last block (which is now ready)
        let payload_round = last_block.first_block().round();
        let ready_block = missing_block_store.remove_ready_block(
            current_epoch,
            payload_round,
            &block_payload_store,
        );

        // Verify that the last block was removed
        assert_eq!(ready_block, Some(last_block));

        // Verify that the store is empty
        verify_missing_blocks(&missing_block_store, 0, &vec![]);
    }

    #[test]
    fn test_remove_ready_block_multiple_blocks_missing() {
        // Create a new missing block store
        let max_num_pending_blocks = 10;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            ..ConsensusObserverConfig::default()
        };
        let missing_block_store = MissingBlockStore::new(consensus_observer_config);

        // Insert the maximum number of blocks into the store
        let current_epoch = 10;
        let starting_round = 100;
        let missing_blocks = create_and_add_missing_blocks(
            &missing_block_store,
            max_num_pending_blocks,
            current_epoch,
            starting_round,
            5,
        );

        // Create an empty block payload store
        let mut block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Incrementally insert and process each payload for the first block
        let first_block = missing_blocks.first().unwrap().clone();
        for block in first_block.blocks().clone() {
            // Insert the block
            let block_payload =
                BlockPayload::new(block.block_info(), BlockTransactionPayload::empty());
            block_payload_store.insert_block_payload(block_payload, true);

            // Attempt to remove the block (which might not be ready)
            let payload_round = block.round();
            let ready_block = missing_block_store.remove_ready_block(
                current_epoch,
                payload_round,
                &block_payload_store,
            );

            // If the block is ready, verify that it was removed.
            // Otherwise, verify that the block still remains.
            if payload_round == first_block.last_block().round() {
                // The block should be ready
                assert_eq!(ready_block, Some(first_block.clone()));

                // Verify that the block was removed
                verify_missing_blocks(
                    &missing_block_store,
                    max_num_pending_blocks - 1,
                    &missing_blocks[1..].to_vec(),
                );
            } else {
                // The block should not be ready
                assert!(ready_block.is_none());

                // Verify that the block still remains
                verify_missing_blocks(
                    &missing_block_store,
                    max_num_pending_blocks,
                    &missing_blocks,
                );
            }
        }

        // Incrementally insert and process payloads for the last block (except one)
        let last_block = missing_blocks.last().unwrap().clone();
        for block in last_block.blocks().clone() {
            // Insert the block only if this is not the first block
            let payload_round = block.round();
            if payload_round != last_block.first_block().round() {
                let block_payload =
                    BlockPayload::new(block.block_info(), BlockTransactionPayload::empty());
                block_payload_store.insert_block_payload(block_payload, true);
            }

            // Attempt to remove the block (which might not be ready)
            let ready_block = missing_block_store.remove_ready_block(
                current_epoch,
                payload_round,
                &block_payload_store,
            );

            // The block should not be ready
            assert!(ready_block.is_none());

            // Verify that the block still remains or has been removed on the last insert
            if payload_round == last_block.last_block().round() {
                verify_missing_blocks(&missing_block_store, 0, &vec![]);
            } else {
                verify_missing_blocks(&missing_block_store, 1, &vec![last_block.clone()]);
            }
        }

        // Verify that the store is now empty
        verify_missing_blocks(&missing_block_store, 0, &vec![]);
    }

    #[test]
    fn test_remove_ready_block_singular_blocks() {
        // Create a new missing block store
        let max_num_pending_blocks = 10;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            ..ConsensusObserverConfig::default()
        };
        let missing_block_store = MissingBlockStore::new(consensus_observer_config);

        // Insert the maximum number of blocks into the store
        let current_epoch = 0;
        let starting_round = 0;
        let missing_blocks = create_and_add_missing_blocks(
            &missing_block_store,
            max_num_pending_blocks,
            current_epoch,
            starting_round,
            1,
        );

        // Create a new block payload store and insert payloads for the first block
        let mut block_payload_store = BlockPayloadStore::new(consensus_observer_config);
        let first_block = missing_blocks.first().unwrap().clone();
        insert_payloads_for_ordered_block(&mut block_payload_store, &first_block);

        // Remove the first block (which is now ready)
        let payload_round = first_block.first_block().round();
        let ready_block = missing_block_store.remove_ready_block(
            current_epoch,
            payload_round,
            &block_payload_store,
        );
        assert_eq!(ready_block, Some(first_block));

        // Verify that the first block was removed
        verify_missing_blocks(
            &missing_block_store,
            max_num_pending_blocks - 1,
            &missing_blocks[1..].to_vec(),
        );

        // Insert payloads for the second block
        let second_block = missing_blocks[1].clone();
        insert_payloads_for_ordered_block(&mut block_payload_store, &second_block);

        // Remove the second block (which is now ready)
        let payload_round = second_block.first_block().round();
        let ready_block = missing_block_store.remove_ready_block(
            current_epoch,
            payload_round,
            &block_payload_store,
        );
        assert_eq!(ready_block, Some(second_block));

        // Verify that the first and second blocks were removed
        verify_missing_blocks(
            &missing_block_store,
            max_num_pending_blocks - 2,
            &missing_blocks[2..].to_vec(),
        );

        // Insert payloads for the last block
        let last_block = missing_blocks.last().unwrap().clone();
        insert_payloads_for_ordered_block(&mut block_payload_store, &last_block);

        // Remove the last block (which is now ready)
        let payload_round = last_block.first_block().round();
        let ready_block = missing_block_store.remove_ready_block(
            current_epoch,
            payload_round,
            &block_payload_store,
        );

        // Verify that the last block was removed
        assert_eq!(ready_block, Some(last_block));

        // Verify that the store is empty
        verify_missing_blocks(&missing_block_store, 0, &vec![]);
    }

    #[test]
    fn test_remove_ready_block_singular_blocks_missing() {
        // Create a new missing block store
        let max_num_pending_blocks = 100;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            ..ConsensusObserverConfig::default()
        };
        let missing_block_store = MissingBlockStore::new(consensus_observer_config);

        // Insert the maximum number of blocks into the store
        let current_epoch = 10;
        let starting_round = 100;
        let missing_blocks = create_and_add_missing_blocks(
            &missing_block_store,
            max_num_pending_blocks,
            current_epoch,
            starting_round,
            1,
        );

        // Create an empty block payload store
        let block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Remove the third block (which is not ready)
        let third_block = missing_blocks[2].clone();
        let third_block_round = third_block.first_block().round();
        let ready_block = missing_block_store.remove_ready_block(
            current_epoch,
            third_block_round,
            &block_payload_store,
        );
        assert!(ready_block.is_none());

        // Verify that the first three blocks were removed
        verify_missing_blocks(
            &missing_block_store,
            max_num_pending_blocks - 3,
            &missing_blocks[3..].to_vec(),
        );

        // Remove the last block (which is not ready)
        let last_block = missing_blocks.last().unwrap().clone();
        let last_block_round = last_block.first_block().round();
        let ready_block = missing_block_store.remove_ready_block(
            current_epoch,
            last_block_round,
            &block_payload_store,
        );
        assert!(ready_block.is_none());

        // Verify that the store is now empty
        verify_missing_blocks(&missing_block_store, 0, &vec![]);
    }

    /// Creates and adds the specified number of blocks to the missing block store
    fn create_and_add_missing_blocks(
        missing_block_store: &MissingBlockStore,
        num_missing_blocks: usize,
        epoch: u64,
        starting_round: Round,
        max_pipelined_blocks: u64,
    ) -> Vec<OrderedBlock> {
        let mut missing_blocks = vec![];
        for i in 0..num_missing_blocks {
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

            // Insert the ordered block into the missing block store
            missing_block_store.insert_missing_block(ordered_block.clone());

            // Add the ordered block to the missing blocks
            missing_blocks.push(ordered_block);
        }

        missing_blocks
    }

    /// Inserts payloads into the payload store for the ordered block
    fn insert_payloads_for_ordered_block(
        block_payload_store: &mut BlockPayloadStore,
        ordered_block: &OrderedBlock,
    ) {
        for block in ordered_block.blocks() {
            let block_payload =
                BlockPayload::new(block.block_info(), BlockTransactionPayload::empty());
            block_payload_store.insert_block_payload(block_payload, true);
        }
    }

    /// Verifies that the missing block store contains the expected blocks
    fn verify_missing_blocks(
        missing_block_store: &MissingBlockStore,
        num_expected_blocks: usize,
        missing_blocks: &Vec<OrderedBlock>,
    ) {
        // Check the number of missing blocks
        let blocks_missing_payloads = missing_block_store.blocks_missing_payloads.lock();
        assert_eq!(blocks_missing_payloads.len(), num_expected_blocks);

        // Check that all missing blocks are in the store
        for missing_block in missing_blocks {
            let first_block = missing_block.first_block();
            assert_eq!(
                blocks_missing_payloads
                    .get(&(first_block.epoch(), first_block.round()))
                    .unwrap(),
                missing_block
            );
        }
    }
}
