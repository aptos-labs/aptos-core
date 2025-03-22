// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensus_observer::{
        common::{
            logging::{LogEntry, LogSchema},
            metrics,
        },
        network::observer_message::OrderedBlock,
        observer::{
            execution_pool, execution_pool::ObservedOrderedBlock, payload_store::BlockPayloadStore,
        },
    },
    util::BlockStorage,
};
use aptos_config::{config::ConsensusObserverConfig, network_id::PeerNetworkId};
use aptos_consensus_types::pipelined_block::PipelinedBlock;
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_logger::{error, info, warn};
use aptos_types::{block_info::Round, ledger_info::LedgerInfoWithSignatures};
use std::{
    collections::{btree_map::Entry, BTreeMap},
    sync::Arc,
    time::Instant,
};

/// A simple struct that holds a pending block with relevant metadata
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingBlockWithMetadata {
    peer_network_id: PeerNetworkId, // The peer network ID of the block sender
    block_receipt_time: Instant,    // The time the block was received
    observed_ordered_block: ObservedOrderedBlock, // The observed ordered block
}

impl PendingBlockWithMetadata {
    pub fn new_with_arc(
        peer_network_id: PeerNetworkId,
        block_receipt_time: Instant,
        observed_ordered_block: ObservedOrderedBlock,
    ) -> Arc<Self> {
        let pending_block_with_metadata = Self {
            peer_network_id,
            block_receipt_time,
            observed_ordered_block,
        };
        Arc::new(pending_block_with_metadata)
    }

    /// Unpacks the block with metadata into its components.
    /// Note: this will copy/clone all components.
    pub fn unpack(&self) -> (PeerNetworkId, Instant, ObservedOrderedBlock) {
        (
            self.peer_network_id,
            self.block_receipt_time,
            self.observed_ordered_block.clone(),
        )
    }

    /// Returns a reference to the observed ordered block
    pub fn observed_ordered_block(&self) -> &ObservedOrderedBlock {
        &self.observed_ordered_block
    }

    /// Returns a reference to the ordered block
    pub fn ordered_block(&self) -> &OrderedBlock {
        self.observed_ordered_block.ordered_block()
    }
}

/// A simple struct to hold pending blocks with metadata
pub struct PendingBlockStore {
    // The configuration of the consensus observer
    consensus_observer_config: ConsensusObserverConfig,

    // A map of pending blocks with metadata. The key is the
    // (epoch, round) of the first block in the ordered block.
    pending_blocks: BTreeMap<(u64, Round), Arc<PendingBlockWithMetadata>>,

    // A map of pending blocks with metadata. The key is the
    // hash of the first block in the ordered block.
    // Note: this is the same as pending_blocks, but with a different key.
    pending_blocks_by_hash: BTreeMap<HashValue, Arc<PendingBlockWithMetadata>>,
}

impl PendingBlockStore {
    pub fn new(consensus_observer_config: ConsensusObserverConfig) -> Self {
        Self {
            consensus_observer_config,
            pending_blocks: BTreeMap::new(),
            pending_blocks_by_hash: BTreeMap::new(),
        }
    }

    /// Clears all pending blocks from the store
    pub fn clear_pending_blocks(&mut self) {
        self.pending_blocks.clear();
        self.pending_blocks_by_hash.clear();
    }

    /// Returns true iff the store contains an entry for the given ordered block
    pub fn existing_pending_block(&self, ordered_block: &OrderedBlock) -> bool {
        // Get the epoch and round of the first block
        let first_block = ordered_block.first_block();
        let first_block_epoch_round = (first_block.epoch(), first_block.round());

        // Check if the block is already in the store by epoch and round
        self.pending_blocks.contains_key(&first_block_epoch_round)
    }

    #[cfg(test)]
    /// Returns all pending blocks in the store. This is only used for testing.
    pub fn get_pending_blocks(&self) -> Vec<Arc<PendingBlockWithMetadata>> {
        self.pending_blocks.values().cloned().collect()
    }

    /// Returns the pending block with the given hash (if it exists)
    pub fn get_pending_block_by_hash(
        &self,
        block_hash: HashValue,
    ) -> Option<Arc<PendingBlockWithMetadata>> {
        self.pending_blocks_by_hash.get(&block_hash).cloned()
    }

    /// Inserts a pending block into the store
    pub fn insert_pending_block(&mut self, pending_block: Arc<PendingBlockWithMetadata>) {
        // Verify that both stores have the same number of entries.
        // If not, log an error as this should never happen.
        let num_pending_blocks = self.pending_blocks.len();
        let num_pending_blocks_by_hash = self.pending_blocks_by_hash.len();
        if num_pending_blocks != num_pending_blocks_by_hash {
            error!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "The pending block stores have different numbers of entries: {} and {} (by hash)",
                    num_pending_blocks, num_pending_blocks_by_hash
                ))
            );
        }

        // Verify that the number of payloads doesn't exceed the maximum
        let max_num_pending_blocks = self.consensus_observer_config.max_num_pending_blocks as usize;
        if num_pending_blocks >= max_num_pending_blocks {
            warn!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Exceeded the maximum number of pending blocks: {:?}. Dropping block: {:?}!",
                    max_num_pending_blocks,
                    pending_block.ordered_block().first_block().block_info(),
                ))
            );
            return; // Drop the block if we've exceeded the maximum
        }

        // Get the first block in the ordered blocks
        let first_block = pending_block.ordered_block().first_block();

        // Insert the block into the store using the epoch round of the first block
        let first_block_epoch_round = (first_block.epoch(), first_block.round());
        match self.pending_blocks.entry(first_block_epoch_round) {
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
                entry.insert(pending_block.clone());
            },
        }

        // Insert the block into the hash store using the hash of the first block
        let first_block_hash = first_block.id();
        match self.pending_blocks_by_hash.entry(first_block_hash) {
            Entry::Occupied(_) => {
                // The block is already in the hash store
                warn!(
                    LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                        "A pending block was already found for the given block hash: {:?}",
                        first_block_hash
                    ))
                );
            },
            Entry::Vacant(entry) => {
                // Insert the block into the hash store
                entry.insert(pending_block);
            },
        }
    }

    /// Removes the pending blocks for the given commit ledger info. If
    /// the execution pool window size is None, all blocks up to (and
    /// including) the epoch and round of the commit will be removed.
    /// Otherwise, a buffer of blocks preceding the commit will be retained
    /// (to ensure we have enough blocks to satisfy the execution window).
    pub fn remove_blocks_for_commit(
        &mut self,
        commit_ledger_info: &LedgerInfoWithSignatures,
        execution_pool_window_size: Option<u64>,
    ) {
        // Determine the epoch and round to split off
        let window_buffer_multiplier = self
            .consensus_observer_config
            .observer_block_window_buffer_multiplier;
        let (split_off_epoch, split_off_round) =
            execution_pool::calculate_epoch_round_split_for_commit(
                commit_ledger_info,
                execution_pool_window_size,
                window_buffer_multiplier,
            );

        // Split the blocks at the epoch and round and identify the blocks to retain
        let blocks_to_retain = self
            .pending_blocks
            .split_off(&(split_off_epoch, split_off_round));

        // Remove the old blocks from the hash store
        for pending_block in self.pending_blocks.values() {
            let first_block = pending_block.ordered_block().first_block();
            if self
                .pending_blocks_by_hash
                .remove(&first_block.id())
                .is_none()
            {
                error!(
                    LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                        "Failed to remove pending block by hash for block: {:?}",
                        first_block.block_info()
                    ))
                );
            }
        }

        // Update the pending block store with the blocks to retain
        self.pending_blocks = blocks_to_retain;
    }

    #[cfg(test)]
    /// Removes the pending block from the pending block store (only used for testing)
    pub fn remove_pending_block(&mut self, ordered_block: &OrderedBlock) {
        // Get the epoch, round and hash of the first block
        let first_block = ordered_block.first_block();
        let first_block_epoch_round = (first_block.epoch(), first_block.round());
        let first_block_hash = first_block.id();

        // Remove the block from both stores
        self.pending_blocks
            .remove(&first_block_epoch_round)
            .unwrap();
        self.pending_blocks_by_hash
            .remove(&first_block_hash)
            .unwrap();
    }

    /// Removes and returns the block from the store that is now ready
    /// to be processed (after the new payload has been received).
    // TODO: identify how this will work with execution pool blocks!
    pub fn remove_ready_block(
        &mut self,
        received_payload_epoch: u64,
        received_payload_round: Round,
        block_payload_store: Arc<Mutex<BlockPayloadStore>>,
    ) -> Option<Arc<PendingBlockWithMetadata>> {
        // Calculate the round at which to split the blocks
        let split_round = received_payload_round.saturating_add(1);

        // Split the blocks at the epoch and round
        let mut blocks_at_higher_rounds = self
            .pending_blocks
            .split_off(&(received_payload_epoch, split_round));

        // Check if the last block is ready (this should be the only ready block).
        // Any earlier blocks are considered out-of-date and will be dropped.
        let mut ready_block = None;
        if let Some((epoch_and_round, pending_block)) = self.pending_blocks.pop_last() {
            // If all payloads exist for the block, then the block is ready
            if block_payload_store
                .lock()
                .all_payloads_exist(pending_block.ordered_block().blocks())
            {
                ready_block = Some(pending_block);
            } else {
                // Otherwise, check if we're still waiting for higher payloads for the block
                let last_pending_block_round = pending_block.ordered_block().last_block().round();
                if last_pending_block_round > received_payload_round {
                    blocks_at_higher_rounds.insert(epoch_and_round, pending_block);
                }
            }
        }

        // Check if any out-of-date blocks are going to be dropped
        if !self.pending_blocks.is_empty() {
            info!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Dropped {:?} out-of-date pending blocks before epoch and round: {:?}",
                    self.pending_blocks.len(),
                    (received_payload_epoch, received_payload_round)
                ))
            );
        }

        // TODO: optimize this flow!

        // Clear all blocks from the pending block stores
        self.clear_pending_blocks();

        // Update the pending block stores to only include the blocks at higher rounds
        self.pending_blocks = blocks_at_higher_rounds;
        for pending_block in self.pending_blocks.values() {
            let first_block = pending_block.ordered_block().first_block();
            self.pending_blocks_by_hash
                .insert(first_block.id(), pending_block.clone());
        }

        // Return the ready block (if one exists)
        ready_block
    }

    /// Updates the metrics for the pending blocks
    pub fn update_pending_blocks_metrics(&self) {
        // Update the number of pending block entries
        let num_entries = self.pending_blocks.len() as u64;
        metrics::set_gauge_with_label(
            &metrics::OBSERVER_NUM_PROCESSED_BLOCKS,
            metrics::PENDING_BLOCK_ENTRIES_LABEL,
            num_entries,
        );

        // Update the number of pending block by hash entries
        let num_entries_by_hash = self.pending_blocks_by_hash.len() as u64;
        metrics::set_gauge_with_label(
            &metrics::OBSERVER_NUM_PROCESSED_BLOCKS,
            metrics::PENDING_BLOCK_ENTRIES_BY_HASH_LABEL,
            num_entries_by_hash,
        );

        // Update the total number of pending blocks
        let num_pending_blocks = self
            .pending_blocks
            .values()
            .map(|block| block.ordered_block().blocks().len() as u64)
            .sum();
        metrics::set_gauge_with_label(
            &metrics::OBSERVER_NUM_PROCESSED_BLOCKS,
            metrics::PENDING_BLOCKS_LABEL,
            num_pending_blocks,
        );

        // Update the total number of pending blocks by hash
        let num_pending_blocks_by_hash = self
            .pending_blocks_by_hash
            .values()
            .map(|block| block.ordered_block().blocks().len() as u64)
            .sum();
        metrics::set_gauge_with_label(
            &metrics::OBSERVER_NUM_PROCESSED_BLOCKS,
            metrics::PENDING_BLOCKS_BY_HASH_LABEL,
            num_pending_blocks_by_hash,
        );

        // Update the highest round for the pending blocks
        let highest_pending_round = self
            .pending_blocks
            .last_key_value()
            .map(|(_, pending_block)| pending_block.ordered_block().last_block().round())
            .unwrap_or(0);
        metrics::set_gauge_with_label(
            &metrics::OBSERVER_PROCESSED_BLOCK_ROUNDS,
            metrics::PENDING_BLOCKS_LABEL,
            highest_pending_round,
        );
    }
}

/// Implement the BlockStorage trait for the PendingBlockStore.
/// This is required to calculate and fetch the block window.
impl BlockStorage for PendingBlockStore {
    fn get_pipelined_block(&self, block_id: &HashValue) -> Option<Arc<PipelinedBlock>> {
        // Lookup the ordered block by hash
        let pending_block_with_metadata = match self.get_pending_block_by_hash(*block_id) {
            Some(pending_block_with_metadata) => pending_block_with_metadata,
            None => {
                return None; // The block is not in the store
            },
        };

        // Extract the pipelined block (if it exists)
        for pipelined_block in pending_block_with_metadata.ordered_block().blocks() {
            if pipelined_block.block().id() == *block_id {
                return Some(pipelined_block.clone());
            }
        }

        // Log an error and return None. This should never really
        // happen as the block was inserted into the store!
        error!(
            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                "The pipelined block was not found in the ordered block entry: {:?}",
                pending_block_with_metadata.ordered_block().blocks()
            ))
        );
        None
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
        pipelined_block::{OrderedBlockWindow, PipelinedBlock},
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
    fn test_clear_pending_blocks() {
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
        pending_block_store.lock().clear_pending_blocks();

        // Verify that the store is now empty
        assert!(pending_block_store.lock().pending_blocks.is_empty());

        // Verify that the hash store is now empty
        assert!(pending_block_store.lock().pending_blocks_by_hash.is_empty());
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
            // Verify that the block is in the store
            assert!(pending_block_store
                .lock()
                .existing_pending_block(pending_block));

            // Verify that the block is in the store by hash
            let block_hash = pending_block.first_block().id();
            assert!(pending_block_store
                .lock()
                .get_pending_block_by_hash(block_hash)
                .is_some());
        }

        // Create a new block payload store and insert payloads for the second block
        let block_payload_store = Arc::new(Mutex::new(BlockPayloadStore::new(
            consensus_observer_config,
        )));
        let second_block = pending_blocks[1].clone();
        insert_payloads_for_ordered_block(block_payload_store.clone(), &second_block);

        // Remove the second block (which is now ready)
        let payload_round = second_block.first_block().round();
        let ready_block = pending_block_store
            .lock()
            .remove_ready_block(current_epoch, payload_round, block_payload_store.clone())
            .unwrap();
        assert_eq!(ready_block.ordered_block().clone(), second_block);

        // Verify that the first and second blocks were removed
        verify_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks - 2,
            &pending_blocks[2..].to_vec(),
        );

        // Verify that the first and second blocks are no longer in the store
        for pending_block in &pending_blocks[..2] {
            // Verify that the block is not in the store
            assert!(!pending_block_store
                .lock()
                .existing_pending_block(pending_block));

            // Verify that the block is not in the store by hash
            let block_hash = pending_block.first_block().id();
            assert!(pending_block_store
                .lock()
                .get_pending_block_by_hash(block_hash)
                .is_none());
        }
    }

    #[test]
    fn test_get_pending_block_by_hash() {
        // Create a new pending block store
        let max_num_pending_blocks = 50;
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

        // Verify that all blocks were inserted correctly
        for pending_block in &pending_blocks {
            let pending_block_by_hash = pending_block_store
                .lock()
                .get_pending_block_by_hash(pending_block.first_block().id())
                .unwrap();
            assert_eq!(
                pending_block_by_hash.observed_ordered_block.ordered_block(),
                pending_block
            );
        }

        // Remove the first and second blocks manually
        for block in &pending_blocks[..2] {
            pending_block_store
                .lock()
                .pending_blocks_by_hash
                .remove(&block.first_block().id());
        }

        // Verify that the first and second blocks are no longer in the store
        for pending_block in &pending_blocks[..2] {
            assert!(pending_block_store
                .lock()
                .get_pending_block_by_hash(pending_block.first_block().id())
                .is_none());
        }

        // Verify that the remaining blocks are still in the store by hash
        for pending_block in &pending_blocks[2..] {
            let pending_block_by_hash = pending_block_store
                .lock()
                .get_pending_block_by_hash(pending_block.first_block().id())
                .unwrap();
            assert_eq!(
                pending_block_by_hash.observed_ordered_block.ordered_block(),
                pending_block
            );
        }

        // Clear the blocks from the store
        pending_block_store.lock().clear_pending_blocks();

        // Verify that all blocks are no longer in the store by hash
        for pending_block in &pending_blocks {
            assert!(pending_block_store
                .lock()
                .get_pending_block_by_hash(pending_block.first_block().id())
                .is_none());
        }
    }

    #[test]
    fn test_get_pipelined_block() {
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

        // Verify that all blocks can be fetched by hash
        for pending_block in &pending_blocks {
            let block_hash = pending_block.first_block().id();
            let block = pending_block_store
                .lock()
                .get_pipelined_block(&block_hash)
                .unwrap();
            assert_eq!(block, pending_block.first_block());
        }

        // Verify that missing block hashes are not in the store
        let missing_block_hash = HashValue::random();
        let block = pending_block_store
            .lock()
            .get_pipelined_block(&missing_block_hash);
        assert!(block.is_none());

        // Clear the blocks from the store
        pending_block_store.lock().clear_pending_blocks();

        // Reinsert the first block into the hash store (manually) using an incorrect hash
        let first_block = pending_blocks[0].clone();
        let incorrect_hash = HashValue::random();
        let pending_block_with_metadata = PendingBlockWithMetadata::new_with_arc(
            PeerNetworkId::random(),
            Instant::now(),
            ObservedOrderedBlock::new_for_testing(first_block.clone()),
        );
        pending_block_store
            .lock()
            .pending_blocks_by_hash
            .insert(incorrect_hash, pending_block_with_metadata);

        // Verify the block cannot be found (there is a hash mismatch the entry and pipelined block)
        let block = pending_block_store
            .lock()
            .get_pipelined_block(&incorrect_hash);
        assert!(block.is_none());
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

        // Clear the blocks from the store
        pending_block_store.lock().clear_pending_blocks();

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
    }

    #[test]
    fn test_insert_pending_block_limit() {
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

        // Insert the maximum number of blocks into the store (again)
        let starting_round = (max_num_pending_blocks * 100) as Round;
        let pending_blocks = create_and_add_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks,
            current_epoch,
            starting_round,
            5,
        );

        // Verify that none of the new blocks were inserted (as we've reached the limit)
        for block in &pending_blocks {
            assert!(!pending_block_store.lock().existing_pending_block(block));
        }
        verify_num_pending_blocks(&pending_block_store, max_num_pending_blocks);

        // Clear the blocks from the store
        pending_block_store.lock().clear_pending_blocks();

        // Insert more than the maximum number of blocks into the store
        let pending_blocks = create_and_add_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks * 2, // Double the limit
            current_epoch,
            starting_round,
            5,
        );

        // Verify that only the first half of the blocks were inserted
        verify_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks,
            &pending_blocks[..max_num_pending_blocks].to_vec(),
        );

        // Verify that the second half of the blocks were not inserted
        for block in &pending_blocks[max_num_pending_blocks..] {
            assert!(!pending_block_store.lock().existing_pending_block(block));
        }
        verify_num_pending_blocks(&pending_block_store, max_num_pending_blocks);

        // Clear the blocks from the store
        pending_block_store.lock().clear_pending_blocks();

        // Insert less than the number of blocks into the store
        let num_pending_blocks = max_num_pending_blocks / 2; // Half the limit
        let pending_blocks = create_and_add_pending_blocks(
            pending_block_store.clone(),
            num_pending_blocks,
            current_epoch,
            starting_round,
            5,
        );

        // Verify that all blocks were inserted correctly
        verify_pending_blocks(
            pending_block_store.clone(),
            num_pending_blocks,
            &pending_blocks,
        );
    }

    #[test]
    fn test_pending_block_metadata() {
        // Create a new pending block store
        let max_num_pending_blocks = 10;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            ..ConsensusObserverConfig::default()
        };
        let pending_block_store = Arc::new(Mutex::new(PendingBlockStore::new(
            consensus_observer_config,
        )));

        // Insert the maximum number of pending blocks into the store
        let mut pending_blocks_with_metadata = vec![];
        for i in 0..max_num_pending_blocks {
            // Create an observed ordered block
            let ordered_block = create_ordered_block(0, 0, 1, i);
            let observed_ordered_block = ObservedOrderedBlock::new(ordered_block.clone());

            // Create a pending block with metadata
            let pending_block_with_metadata = PendingBlockWithMetadata::new_with_arc(
                PeerNetworkId::random(),
                Instant::now(),
                observed_ordered_block.clone(),
            );

            // Insert the ordered block into the pending block store
            pending_block_store
                .lock()
                .insert_pending_block(pending_block_with_metadata.clone());

            // Add the pending block with metadata to the list
            pending_blocks_with_metadata.push(pending_block_with_metadata);
        }

        // Create a new block payload store and insert payloads for all pending blocks
        let block_payload_store = Arc::new(Mutex::new(BlockPayloadStore::new(
            consensus_observer_config,
        )));
        for pending_block_with_metadata in &pending_blocks_with_metadata {
            insert_payloads_for_ordered_block(
                block_payload_store.clone(),
                pending_block_with_metadata.ordered_block(),
            );
        }

        // Remove each of the pending blocks and verify that the metadata is correct
        for expected_block_with_metadata in pending_blocks_with_metadata {
            // Unpack the expected block with metadata into its components
            let (expected_peer_network_id, expected_block_receipt_time, expected_ordered_block) =
                expected_block_with_metadata.unpack();

            // Remove the pending block from the store
            let first_block = expected_ordered_block.ordered_block().first_block();
            let removed_block_with_metadata = pending_block_store
                .lock()
                .remove_ready_block(
                    first_block.epoch(),
                    first_block.round(),
                    block_payload_store.clone(),
                )
                .unwrap();

            // Verify that the pending block metadata is correct
            assert_eq!(
                removed_block_with_metadata.peer_network_id,
                expected_peer_network_id
            );
            assert_eq!(
                removed_block_with_metadata.block_receipt_time,
                expected_block_receipt_time
            );
            assert_eq!(
                removed_block_with_metadata.ordered_block(),
                expected_ordered_block.ordered_block()
            );
        }
    }

    #[test]
    fn test_remove_blocks_for_commit() {
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

        // Remove the pending blocks for a commit at the first round (without an execution pool window)
        let commit_ledger_info = create_ledger_info_for_epoch_round(current_epoch, starting_round);
        pending_block_store
            .lock()
            .remove_blocks_for_commit(&commit_ledger_info, None);

        // Verify that the block is removed from the store
        verify_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks - 1,
            &pending_blocks[1..].to_vec(),
        );

        // Remove the pending blocks for a commit at the 10th round (without an execution pool window)
        let commit_ledger_info =
            create_ledger_info_for_epoch_round(current_epoch, starting_round + 10);
        pending_block_store
            .lock()
            .remove_blocks_for_commit(&commit_ledger_info, None);

        // Verify that the blocks are removed from the store
        verify_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks - 11,
            &pending_blocks[11..].to_vec(),
        );

        // Remove the pending blocks for a commit at the last round (without an execution pool window)
        let commit_ledger_info =
            create_ledger_info_for_epoch_round(current_epoch, starting_round + 99);
        pending_block_store
            .lock()
            .remove_blocks_for_commit(&commit_ledger_info, None);

        // Verify that the store is empty
        verify_pending_blocks(pending_block_store.clone(), 0, &vec![]);
    }

    #[test]
    fn test_remove_blocks_for_commit_execution_pool() {
        // Create a new consensus observer config
        let max_num_pending_blocks = 100;
        let observer_block_window_buffer_multiplier = 2; // Buffer twice the window
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            observer_block_window_buffer_multiplier,
            ..ConsensusObserverConfig::default()
        };

        // Create a new pending block store
        let pending_block_store = Arc::new(Mutex::new(PendingBlockStore::new(
            consensus_observer_config,
        )));

        // Insert the maximum number of blocks into the store
        let current_epoch = 10;
        let starting_round = 0;
        let pending_blocks = create_and_add_pending_blocks(
            pending_block_store.clone(),
            max_num_pending_blocks,
            current_epoch,
            starting_round,
            1,
        );

        // Process commits for rounds less than the buffer (i.e., < window * 2)
        let window_size = 7;
        let buffer_size = window_size * (observer_block_window_buffer_multiplier as usize);
        for commit_round in 0..buffer_size {
            // Process a commit at the commit round
            let pending_block = pending_blocks.get(commit_round).unwrap().first_block();
            let commit_ledger_info =
                create_ledger_info_for_epoch_round(pending_block.epoch(), pending_block.round());
            pending_block_store
                .lock()
                .remove_blocks_for_commit(&commit_ledger_info, Some(window_size as u64));

            // Verify that the block is not removed from the store
            verify_contains_block(&pending_block_store, pending_block.clone(), true);
        }

        // Verify that no payloads were removed
        verify_num_pending_blocks(&pending_block_store.clone(), max_num_pending_blocks);

        // Process a commit for a round one greater than the buffer
        let commit_round = buffer_size;
        let pending_block = pending_blocks.get(commit_round).unwrap().first_block();
        let commit_ledger_info =
            create_ledger_info_for_epoch_round(pending_block.epoch(), pending_block.round());
        pending_block_store
            .lock()
            .remove_blocks_for_commit(&commit_ledger_info, Some(window_size as u64));

        // Verify the first payload was removed (it's outside the window)
        let pending_block = pending_blocks.first().unwrap().first_block();
        verify_contains_block(&pending_block_store, pending_block.clone(), false);
        verify_num_pending_blocks(&pending_block_store, max_num_pending_blocks - 1);

        // Process a commit for the last round
        let commit_round = max_num_pending_blocks - 1;
        let pending_block = pending_blocks.get(commit_round).unwrap().first_block();
        let commit_ledger_info =
            create_ledger_info_for_epoch_round(pending_block.epoch(), pending_block.round());
        pending_block_store
            .lock()
            .remove_blocks_for_commit(&commit_ledger_info, Some(window_size as u64));

        // Verify that all payloads before the buffer were removed
        let buffer_start_round = commit_round - buffer_size + 1;
        for removed_round in 0..buffer_start_round {
            let pending_block = pending_blocks.get(removed_round).unwrap().first_block();
            verify_contains_block(&pending_block_store, pending_block.clone(), false);
        }

        // Verify that all payloads after the buffer start were retained
        for retained_round in buffer_start_round..max_num_pending_blocks {
            let pending_block = pending_blocks.get(retained_round).unwrap().first_block();
            verify_contains_block(&pending_block_store, pending_block.clone(), true);
        }

        // Verify that only the payloads in the buffer were retained
        verify_num_pending_blocks(&pending_block_store.clone(), buffer_size);
    }

    #[test]
    fn test_remove_blocks_for_commit_execution_pool_epoch() {
        // Create a new consensus observer config
        let max_num_pending_blocks = 300;
        let observer_block_window_buffer_multiplier = 3; // Buffer three times the window
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            observer_block_window_buffer_multiplier,
            ..ConsensusObserverConfig::default()
        };

        // Create a new pending block store
        let pending_block_store = Arc::new(Mutex::new(PendingBlockStore::new(
            consensus_observer_config,
        )));

        // Add some blocks to the store for the current epoch
        let current_epoch = 15;
        let num_pending_blocks = 50;
        let pending_blocks = create_and_add_pending_blocks(
            pending_block_store.clone(),
            num_pending_blocks,
            current_epoch,
            0,
            1,
        );

        // Add some blocks to the store for the next epoch
        let next_epoch = current_epoch + 1;
        let num_pending_blocks_next_epoch = 60;
        let pending_blocks_next_epoch = create_and_add_pending_blocks(
            pending_block_store.clone(),
            num_pending_blocks_next_epoch,
            next_epoch,
            0,
            1,
        );

        // Add some blocks to the store for a future epoch
        let future_epoch = next_epoch + 1;
        let num_pending_blocks_future_epoch = 70;
        let pending_blocks_future_epoch = create_and_add_pending_blocks(
            pending_block_store.clone(),
            num_pending_blocks_future_epoch,
            future_epoch,
            0,
            1,
        );

        // Verify the number of pending blocks
        verify_num_pending_blocks(
            &pending_block_store.clone(),
            num_pending_blocks + num_pending_blocks_next_epoch + num_pending_blocks_future_epoch,
        );

        // Process commits for rounds less than the buffer in the next epoch (i.e., < window * 3)
        let window_size = 8;
        let buffer_size = window_size * (observer_block_window_buffer_multiplier as usize);
        for commit_round in 0..buffer_size {
            // Process a commit at the commit round
            let pending_block = pending_blocks_next_epoch
                .get(commit_round)
                .unwrap()
                .first_block();
            let commit_ledger_info =
                create_ledger_info_for_epoch_round(pending_block.epoch(), pending_block.round());
            pending_block_store
                .lock()
                .remove_blocks_for_commit(&commit_ledger_info, Some(window_size as u64));

            // Verify the block was not removed (it's within the window)
            verify_contains_block(&pending_block_store, pending_block.clone(), true);
        }

        // Verify the pending blocks for the current epoch were all removed
        for pending_block in &pending_blocks {
            verify_contains_block(
                &pending_block_store,
                pending_block.first_block().clone(),
                false,
            );
        }
        verify_num_pending_blocks(
            &pending_block_store.clone(),
            num_pending_blocks_next_epoch + num_pending_blocks_future_epoch,
        );

        // Process a commit for the last round in the next epoch
        let commit_round = num_pending_blocks_next_epoch - 1;
        let pending_block = pending_blocks_next_epoch
            .get(commit_round)
            .unwrap()
            .first_block();
        let commit_ledger_info =
            create_ledger_info_for_epoch_round(pending_block.epoch(), pending_block.round());
        pending_block_store
            .lock()
            .remove_blocks_for_commit(&commit_ledger_info, Some(window_size as u64));

        // Verify that all payloads before the buffer were removed
        let buffer_start_round = commit_round - buffer_size + 1;
        for removed_round in 0..buffer_start_round {
            let pending_block = pending_blocks_next_epoch
                .get(removed_round)
                .unwrap()
                .first_block();
            verify_contains_block(&pending_block_store, pending_block.clone(), false);
        }

        // Verify that all payloads after the buffer start were retained
        for retained_round in buffer_start_round..num_pending_blocks_next_epoch {
            let pending_block = pending_blocks_next_epoch
                .get(retained_round)
                .unwrap()
                .first_block();
            verify_contains_block(&pending_block_store, pending_block.clone(), true);
        }

        // Verify the number of pending blocks
        verify_num_pending_blocks(
            &pending_block_store.clone(),
            buffer_size + num_pending_blocks_future_epoch,
        );

        // Process a commit for the first round in the future epoch
        let pending_block = pending_blocks_future_epoch.first().unwrap().first_block();
        let commit_ledger_info =
            create_ledger_info_for_epoch_round(pending_block.epoch(), pending_block.round());
        pending_block_store
            .lock()
            .remove_blocks_for_commit(&commit_ledger_info, Some(window_size as u64));

        // Verify the pending blocks for the next epoch were all removed
        for pending_block in &pending_blocks_next_epoch {
            verify_contains_block(
                &pending_block_store,
                pending_block.first_block().clone(),
                false,
            );
        }

        // Verify the pending blocks for the future epoch were all retained
        for pending_block in &pending_blocks_future_epoch {
            verify_contains_block(
                &pending_block_store,
                pending_block.first_block().clone(),
                true,
            );
        }
    }

    #[test]
    fn test_remove_payloads_for_commit_execution_pool_windows() {
        // Create a new consensus observer config
        let max_num_pending_blocks = 100;
        let observer_block_window_buffer_multiplier = 1; // Buffer the exact window size
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks,
            observer_block_window_buffer_multiplier,
            ..ConsensusObserverConfig::default()
        };

        // Test various window pool sizes
        for window_size in 1..11 {
            // Create a new pending block store
            let pending_block_store = Arc::new(Mutex::new(PendingBlockStore::new(
                consensus_observer_config,
            )));

            // Add some pending blocks to the store for the current epoch
            let current_epoch = 10;
            let num_pending_blocks = 50;
            let pending_blocks = create_and_add_pending_blocks(
                pending_block_store.clone(),
                num_pending_blocks,
                current_epoch,
                0,
                1,
            );

            // Process commits for rounds less than the buffer (i.e., < window)
            for commit_round in 0..window_size {
                // Process a commit for the pending block
                let pending_block = pending_blocks.get(commit_round).unwrap().first_block();
                let commit_ledger_info = create_ledger_info_for_epoch_round(
                    pending_block.epoch(),
                    pending_block.round(),
                );
                pending_block_store
                    .lock()
                    .remove_blocks_for_commit(&commit_ledger_info, Some(window_size as u64));

                // Verify the block was not removed (it's within the window)
                verify_contains_block(&pending_block_store, pending_block.clone(), true);
            }

            // Verify that no blocks were removed
            verify_num_pending_blocks(&pending_block_store.clone(), num_pending_blocks);

            // Process commits for rounds greater than the buffer (i.e., >= window)
            for commit_round in window_size..num_pending_blocks {
                // Process a commit for the pending block
                let pending_block = pending_blocks.get(commit_round).unwrap().first_block();
                let commit_ledger_info = create_ledger_info_for_epoch_round(
                    pending_block.epoch(),
                    pending_block.round(),
                );
                pending_block_store
                    .lock()
                    .remove_blocks_for_commit(&commit_ledger_info, Some(window_size as u64));

                // Verify that all blocks before the window were removed
                let window_start_round = commit_round - window_size + 1;
                for removed_round in 0..window_start_round {
                    let pending_block = pending_blocks.get(removed_round).unwrap().first_block();
                    verify_contains_block(&pending_block_store, pending_block.clone(), false);
                }

                // Verify that all blocks after the window start were retained
                for retained_round in window_start_round..num_pending_blocks {
                    let pending_block = pending_blocks.get(retained_round).unwrap().first_block();
                    verify_contains_block(&pending_block_store, pending_block.clone(), true);
                }
            }
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
        let ready_block = pending_block_store
            .lock()
            .remove_ready_block(current_epoch, payload_round, block_payload_store.clone())
            .unwrap();
        assert_eq!(ready_block.ordered_block().clone(), second_block);

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
        let ready_block = pending_block_store
            .lock()
            .remove_ready_block(current_epoch, payload_round, block_payload_store.clone())
            .unwrap();

        // Verify that the last block was removed
        assert_eq!(ready_block.ordered_block().clone(), last_block);

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
                let ordered_block = ready_block.unwrap().ordered_block().clone();
                assert_eq!(ordered_block, first_block.clone());

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
        let ready_block = pending_block_store
            .lock()
            .remove_ready_block(current_epoch, payload_round, block_payload_store.clone())
            .unwrap();
        assert_eq!(ready_block.ordered_block().clone(), first_block);

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
        let ready_block = pending_block_store
            .lock()
            .remove_ready_block(current_epoch, payload_round, block_payload_store.clone())
            .unwrap();
        assert_eq!(ready_block.ordered_block().clone(), second_block);

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
        let ready_block = pending_block_store
            .lock()
            .remove_ready_block(current_epoch, payload_round, block_payload_store.clone())
            .unwrap();

        // Verify that the last block was removed
        assert_eq!(ready_block.ordered_block().clone(), last_block);

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
            // Create an ordered block
            let ordered_block =
                create_ordered_block(epoch, starting_round, max_pipelined_blocks, i);

            // Create an observed ordered block
            let observed_ordered_block =
                ObservedOrderedBlock::new_for_testing(ordered_block.clone());

            // Create a pending block with metadata
            let pending_block_with_metadata = PendingBlockWithMetadata::new_with_arc(
                PeerNetworkId::random(),
                Instant::now(),
                observed_ordered_block,
            );

            // Insert the ordered block into the pending block store
            pending_block_store
                .lock()
                .insert_pending_block(pending_block_with_metadata.clone());

            // Add the ordered block to the pending blocks
            pending_blocks.push(ordered_block);
        }

        pending_blocks
    }

    /// Creates and returns a ledger info for the specified epoch and round
    fn create_ledger_info_for_epoch_round(epoch: u64, round: u64) -> LedgerInfoWithSignatures {
        LedgerInfoWithSignatures::new(
            LedgerInfo::new(
                BlockInfo::random_with_epoch(epoch, round),
                HashValue::random(),
            ),
            AggregateSignature::empty(),
        )
    }

    /// Creates and returns an ordered block with the specified maximum number of pipelined blocks
    fn create_ordered_block(
        epoch: u64,
        starting_round: Round,
        max_pipelined_blocks: u64,
        block_index: usize,
    ) -> OrderedBlock {
        // Create the pipelined blocks
        let num_pipelined_blocks = rand::thread_rng().gen_range(1, max_pipelined_blocks + 1);
        let mut pipelined_blocks = vec![];
        for j in 0..num_pipelined_blocks {
            // Calculate the block round
            let round = starting_round + ((block_index as Round) * max_pipelined_blocks) + j; // Ensure gaps between blocks

            // Create a new block info
            let block_info = BlockInfo::new(
                epoch,
                round,
                HashValue::random(),
                HashValue::random(),
                round,
                block_index as u64,
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
            let pipelined_block = Arc::new(PipelinedBlock::new_ordered(
                block,
                // TODO @bchocho @hariria revisit this, not sure how i would do this right now...
                OrderedBlockWindow::empty(),
            ));

            // Add the pipelined block to the list
            pipelined_blocks.push(pipelined_block);
        }

        // Create and return an ordered block
        let ordered_proof = LedgerInfoWithSignatures::new(
            LedgerInfo::new(
                BlockInfo::random_with_epoch(epoch, starting_round),
                HashValue::random(),
            ),
            AggregateSignature::empty(),
        );
        OrderedBlock::new(pipelined_blocks, ordered_proof.clone())
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

    /// Verifies the presence of the block in the pending payload store
    fn verify_contains_block(
        pending_block_store: &Arc<Mutex<PendingBlockStore>>,
        block: Arc<PipelinedBlock>,
        expect_contains: bool,
    ) {
        // Check the presence of the block in the store
        let pending_block_store = pending_block_store.lock();
        let block_found = pending_block_store
            .pending_blocks
            .contains_key(&(block.epoch(), block.round()));
        assert_eq!(block_found, expect_contains);

        // Check the presence of the block in the store by hash
        let block_found_by_hash = pending_block_store
            .pending_blocks_by_hash
            .contains_key(&block.id());
        assert_eq!(block_found_by_hash, expect_contains);
    }

    /// Verifies that the pending block store contains the expected number of blocks
    fn verify_num_pending_blocks(
        pending_block_store: &Arc<Mutex<PendingBlockStore>>,
        max_num_pending_blocks: usize,
    ) {
        // Verify the number of pending blocks
        assert_eq!(
            pending_block_store.lock().pending_blocks.len(),
            max_num_pending_blocks
        );

        // Verify the number of pending blocks by hash
        assert_eq!(
            pending_block_store.lock().pending_blocks_by_hash.len(),
            max_num_pending_blocks
        );
    }

    /// Verifies that the pending block store contains the expected blocks
    fn verify_pending_blocks(
        pending_block_store: Arc<Mutex<PendingBlockStore>>,
        num_expected_blocks: usize,
        pending_blocks: &Vec<OrderedBlock>,
    ) {
        // Check the number of pending blocks
        verify_num_pending_blocks(&pending_block_store, num_expected_blocks);

        // Check that all pending blocks are in the stores
        for pending_block in pending_blocks {
            // Lock the pending block store
            let pending_block_store = pending_block_store.lock();

            // Get the pending block in the store
            let first_block = pending_block.first_block();
            let block_in_store = pending_block_store
                .pending_blocks
                .get(&(first_block.epoch(), first_block.round()))
                .unwrap();

            // Verify that the pending block is in the store
            assert_eq!(block_in_store.ordered_block(), pending_block);

            // Get the pending block in the store by hash
            let first_block_hash = first_block.id();
            let block_in_store = pending_block_store
                .pending_blocks_by_hash
                .get(&first_block_hash)
                .unwrap();

            // Verify the pending block is in the store by hash
            assert_eq!(block_in_store.ordered_block(), pending_block);
        }
    }
}
