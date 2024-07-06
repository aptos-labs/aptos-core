// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::{
    logging::{LogEntry, LogSchema},
    metrics,
    network_message::BlockTransactionPayload,
};
use aptos_config::config::ConsensusObserverConfig;
use aptos_consensus_types::{
    common::Round, pipelined_block::PipelinedBlock, proof_of_store::ProofCache,
};
use aptos_infallible::Mutex;
use aptos_logger::{error, warn};
use aptos_types::{block_info::BlockInfo, epoch_state::EpochState};
use std::{
    collections::{btree_map::Entry, BTreeMap},
    mem,
    sync::Arc,
};
use tokio::sync::oneshot;

/// The status of the block payload (requested or available)
pub enum BlockPayloadStatus {
    Requested(oneshot::Sender<BlockTransactionPayload>),
    Available(BlockTransactionPayload),
}

/// A simple struct to store the block payloads of ordered and committed blocks
#[derive(Clone)]
pub struct BlockPayloadStore {
    // The configuration of the consensus observer
    consensus_observer_config: ConsensusObserverConfig,

    // Verified block transaction payloads (indexed by the epoch and round)
    verified_block_transaction_payloads: Arc<Mutex<BTreeMap<(u64, Round), BlockPayloadStatus>>>,

    // Unverified block transaction payloads (indexed by the epoch and round)
    unverified_block_transaction_payloads: Arc<Mutex<BTreeMap<(u64, Round), BlockPayloadStatus>>>,
}

impl BlockPayloadStore {
    pub fn new(consensus_observer_config: ConsensusObserverConfig) -> Self {
        Self {
            consensus_observer_config,
            verified_block_transaction_payloads: Arc::new(Mutex::new(BTreeMap::new())),
            unverified_block_transaction_payloads: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Returns true iff all the payloads for the given blocks are available.
    /// Here, we only consider verified payloads as available.
    pub fn all_payloads_exist(&self, blocks: &[Arc<PipelinedBlock>]) -> bool {
        let block_transaction_payloads = self.verified_block_transaction_payloads.lock();
        blocks.iter().all(|block| {
            let epoch_and_round = (block.epoch(), block.round());
            matches!(
                block_transaction_payloads.get(&epoch_and_round),
                Some(BlockPayloadStatus::Available(_))
            )
        })
    }

    /// Clears all the payloads from the block payload store
    pub fn clear_all_payloads(&self) {
        self.verified_block_transaction_payloads.lock().clear();
        self.unverified_block_transaction_payloads.lock().clear();
    }

    /// Returns a reference to the verified block transaction payloads
    pub fn get_verified_block_payloads(
        &self,
    ) -> Arc<Mutex<BTreeMap<(u64, Round), BlockPayloadStatus>>> {
        self.verified_block_transaction_payloads.clone()
    }

    /// Inserts the given block payload data into the payload store
    pub fn insert_block_payload(
        &mut self,
        block: BlockInfo,
        payload: BlockTransactionPayload,
        verified_payload_signatures: bool,
    ) {
        // Verify that the number of payloads doesn't exceed the maximum
        let max_num_pending_blocks = self.consensus_observer_config.max_num_pending_blocks as usize;
        let block_transaction_payloads = if verified_payload_signatures {
            &mut self.verified_block_transaction_payloads
        } else {
            &mut self.unverified_block_transaction_payloads
        };
        if block_transaction_payloads.lock().len() >= max_num_pending_blocks {
            warn!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Exceeded the maximum number of payloads: {:?}. Dropping block: {:?}!",
                    max_num_pending_blocks, block,
                ))
            );
            return; // Drop the block if we've exceeded the maximum
        }

        // Update the block transaction payloads
        let epoch_and_round = (block.epoch(), block.round());
        match block_transaction_payloads.lock().entry(epoch_and_round) {
            Entry::Occupied(mut entry) => {
                // Replace the data status with the new block payload
                let mut status = BlockPayloadStatus::Available(payload.clone());
                mem::swap(entry.get_mut(), &mut status);

                // If the status was originally requested, send the payload to the listener
                if let BlockPayloadStatus::Requested(payload_sender) = status {
                    if payload_sender.send(payload).is_err() {
                        error!(LogSchema::new(LogEntry::ConsensusObserver)
                            .message("Failed to send block payload to listener!",));
                    }
                }
            },
            Entry::Vacant(entry) => {
                // Insert the block payload directly into the payload store
                entry.insert(BlockPayloadStatus::Available(payload));
            },
        }
    }

    /// Removes all blocks with an epoch and round less than the given epoch and round
    pub fn remove_blocks_for_epoch_round(&self, epoch: u64, round: Round) {
        // Determine the round to split off
        let split_off_round = round.saturating_add(1);

        // Remove the blocks from both the verified and unverified payloads
        for block_transaction_payloads in [
            self.verified_block_transaction_payloads.clone(),
            self.unverified_block_transaction_payloads.clone(),
        ] {
            let mut block_transaction_payloads = block_transaction_payloads.lock();
            *block_transaction_payloads =
                block_transaction_payloads.split_off(&(epoch, split_off_round));
        }
    }

    /// Removes the committed blocks from the payload store
    pub fn remove_committed_blocks(&self, committed_blocks: &[Arc<PipelinedBlock>]) {
        // Identify the highest epoch and round for the committed blocks
        let (highest_epoch, highest_round) = committed_blocks
            .iter()
            .map(|block| (block.epoch(), block.round()))
            .max()
            .unwrap_or((0, 0));

        // Remove the blocks
        self.remove_blocks_for_epoch_round(highest_epoch, highest_round);
    }

    /// Updates the metrics for the payload store
    pub fn update_payload_store_metrics(&self) {
        // Update the number of verified block payloads
        let num_payloads = self.verified_block_transaction_payloads.lock().len() as u64;
        metrics::set_gauge_with_label(
            &metrics::OBSERVER_NUM_PROCESSED_BLOCKS,
            metrics::VERIFIED_STORED_PAYLOADS_LABEL,
            num_payloads,
        );

        // Update the highest round for the verified block payloads
        let highest_verified_round =
            get_highest_round(self.verified_block_transaction_payloads.clone());
        metrics::set_gauge_with_label(
            &metrics::OBSERVER_PROCESSED_BLOCK_ROUNDS,
            metrics::VERIFIED_STORED_PAYLOADS_LABEL,
            highest_verified_round,
        );

        // Update the number of unverified block payloads
        let num_payloads = self.unverified_block_transaction_payloads.lock().len() as u64;
        metrics::set_gauge_with_label(
            &metrics::OBSERVER_NUM_PROCESSED_BLOCKS,
            metrics::UNVERIFIED_STORED_PAYLOADS_LABEL,
            num_payloads,
        );

        // Update the highest round for the unverified block payloads
        let highest_unverified_round =
            get_highest_round(self.unverified_block_transaction_payloads.clone());
        metrics::set_gauge_with_label(
            &metrics::OBSERVER_PROCESSED_BLOCK_ROUNDS,
            metrics::UNVERIFIED_STORED_PAYLOADS_LABEL,
            highest_unverified_round,
        );
    }

    /// Verifies the block payload signatures against the given epoch state.
    /// If verification is successful, blocks are marked as verified.
    pub fn verify_payload_signatures(&self, epoch_state: &EpochState) {
        // Get the current and next epoch
        let current_epoch = epoch_state.epoch;
        let next_epoch = current_epoch.saturating_add(1);

        // Split the unverified block payloads for the current epoch
        let payloads_for_next_epoch = self
            .unverified_block_transaction_payloads
            .lock()
            .split_off(&(next_epoch, 0));

        // Process the unverified block payloads for the current epoch
        let mut gap_in_verified_blocks = false;
        for ((epoch, round), block_payload_status) in
            self.unverified_block_transaction_payloads.lock().iter()
        {
            if let BlockPayloadStatus::Available(block_payload) = block_payload_status {
                // Create a dummy proof cache to verify the proofs
                let proof_cache = ProofCache::new(1);

                // Verify each of the proof signatures
                let validator_verifier = &epoch_state.verifier;
                for proof_of_store in &block_payload.proof_with_data.proofs {
                    if let Err(error) = proof_of_store.verify(validator_verifier, &proof_cache) {
                        // Failed to verify the proof signatures
                        error!(
                            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                                "Failed to verify the proof of store for batch: {:?}, Error: {:?}",
                                proof_of_store.info(),
                                error
                            ))
                        );

                        // Break early and cause all remaining blocks to be dropped
                        gap_in_verified_blocks = true;
                        break;
                    } else {
                        // Insert the block payload into the verified block payloads
                        self.verified_block_transaction_payloads.lock().insert(
                            (*epoch, *round),
                            BlockPayloadStatus::Available(block_payload.clone()),
                        );
                    }
                }
            } else {
                // The payload is missing
                warn!(
                    LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                        "Missing block payload for epoch: {} and round: {}",
                        epoch, round
                    ))
                );

                // Break early and cause all remaining blocks to be dropped
                gap_in_verified_blocks = true;
                break;
            }
        }

        // Update the unverified pending blocks
        if gap_in_verified_blocks {
            // A gap was detected (drop all remaining blocks and reset)
            self.unverified_block_transaction_payloads.lock().clear();
        } else {
            // No gap was detected (continue to store the blocks for the next epoch)
            *self.unverified_block_transaction_payloads.lock() = payloads_for_next_epoch;
        }
    }
}

/// Returns the highest block round from the given map of transaction payloads
fn get_highest_round(
    block_transaction_payloads: Arc<Mutex<BTreeMap<(u64, Round), BlockPayloadStatus>>>,
) -> Round {
    block_transaction_payloads
        .lock()
        .last_key_value()
        .map(|((_, round), _)| *round)
        .unwrap_or(0)
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_consensus_types::{
        block::Block,
        block_data::{BlockData, BlockType},
        common::ProofWithData,
        quorum_cert::QuorumCert,
    };
    use aptos_crypto::HashValue;
    use aptos_types::{block_info::Round, transaction::Version};

    #[test]
    fn test_all_payloads_exist() {
        // Create a new block payload store
        let consensus_observer_config = ConsensusObserverConfig::default();
        let block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Add some blocks to the payload store
        let num_blocks_in_store = 100;
        let pipelined_blocks = create_and_add_blocks_to_store(
            block_payload_store.clone(),
            num_blocks_in_store,
            0,
            true,
        );

        // Check that all the payloads exist in the block payload store
        assert!(block_payload_store.all_payloads_exist(&pipelined_blocks));

        // Check that a subset of the payloads exist in the block payload store
        let subset_pipelined_blocks = &pipelined_blocks[0..50];
        assert!(block_payload_store.all_payloads_exist(subset_pipelined_blocks));

        // Remove some of the payloads from the block payload store
        block_payload_store.remove_committed_blocks(subset_pipelined_blocks);

        // Check that the payloads no longer exist in the block payload store
        assert!(!block_payload_store.all_payloads_exist(subset_pipelined_blocks));

        // Check that the remaining payloads still exist in the block payload store
        let subset_pipelined_blocks = &pipelined_blocks[50..100];
        assert!(block_payload_store.all_payloads_exist(subset_pipelined_blocks));

        // Remove the remaining payloads from the block payload store
        block_payload_store.remove_committed_blocks(subset_pipelined_blocks);

        // Check that the payloads no longer exist in the block payload store
        assert!(!block_payload_store.all_payloads_exist(subset_pipelined_blocks));
    }

    #[test]
    fn test_all_payloads_exist_requested() {
        // Create a new block payload store
        let consensus_observer_config = ConsensusObserverConfig::default();
        let block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Add several blocks to the payload store
        let num_blocks_in_store = 10;
        let pipelined_blocks = create_and_add_blocks_to_store(
            block_payload_store.clone(),
            num_blocks_in_store,
            0,
            true,
        );

        // Check that the payloads exists in the block payload store
        assert!(block_payload_store.all_payloads_exist(&pipelined_blocks));

        // Mark the payload of the first block as requested
        mark_payload_as_requested(block_payload_store.clone(), &pipelined_blocks[0]);

        // Check that the payload no longer exists in the block payload store
        assert!(!block_payload_store.all_payloads_exist(&pipelined_blocks));

        // Check that the remaining payloads still exist in the block payload store
        assert!(block_payload_store.all_payloads_exist(&pipelined_blocks[1..10]));
    }

    #[test]
    fn test_clear_all_payloads() {
        // Create a new block payload store
        let consensus_observer_config = ConsensusObserverConfig::default();
        let block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Add some blocks to the payload store
        let num_blocks_in_store = 10;
        let pipelined_blocks = create_and_add_blocks_to_store(
            block_payload_store.clone(),
            num_blocks_in_store,
            0,
            true,
        );

        // Check that the payloads exist in the block payload store
        assert!(block_payload_store.all_payloads_exist(&pipelined_blocks));

        // Verify the number of blocks in the block payload store
        check_num_verified_payloads(&block_payload_store, num_blocks_in_store);

        // Clear all the payloads from the block payload store
        block_payload_store.clear_all_payloads();

        // Check that the payloads no longer exist in the block payload store
        assert!(!block_payload_store.all_payloads_exist(&pipelined_blocks));

        // Check that the block payload store is empty
        let block_transaction_payloads = block_payload_store.get_verified_block_payloads();
        assert!(block_transaction_payloads.lock().is_empty());

        // Verify the number of blocks in the block payload store
        check_num_verified_payloads(&block_payload_store, 0);
    }

    #[test]
    fn test_insert_block_payload() {
        // Create a new block payload store
        let consensus_observer_config = ConsensusObserverConfig::default();
        let mut block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Add some blocks to the payload store
        let num_blocks_in_store = 10;
        let pipelined_blocks = create_and_add_blocks_to_store(
            block_payload_store.clone(),
            num_blocks_in_store,
            0,
            true,
        );

        // Check that the block payload store contains the new block payloads
        assert!(block_payload_store.all_payloads_exist(&pipelined_blocks));

        // Verify the number of blocks in the block payload store
        check_num_verified_payloads(&block_payload_store, num_blocks_in_store);

        // Mark the payload of the first block as requested
        let payload_receiver =
            mark_payload_as_requested(block_payload_store.clone(), &pipelined_blocks[0]);

        // Check that the payload no longer exists in the block payload store
        assert!(!block_payload_store.all_payloads_exist(&pipelined_blocks));

        // Verify the number of blocks in the block payload store
        check_num_verified_payloads(&block_payload_store, num_blocks_in_store);

        // Insert the same block payload into the block payload store
        let transaction_payload =
            BlockTransactionPayload::new(vec![], Some(0), ProofWithData::empty(), vec![]);
        block_payload_store.insert_block_payload(
            pipelined_blocks[0].block_info(),
            transaction_payload,
            true,
        );

        // Check that the block payload store now contains the requested block payload
        assert!(block_payload_store.all_payloads_exist(&pipelined_blocks));

        // Check that the payload receiver receives the requested block payload message
        let block_transaction_payload = payload_receiver.blocking_recv().unwrap();
        assert!(block_transaction_payload.transactions.is_empty());
        assert_eq!(block_transaction_payload.limit, Some(0));
    }

    #[test]
    fn test_insert_block_payload_limit() {
        // Create a new config observer config
        let max_num_pending_blocks = 10;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks,
            ..ConsensusObserverConfig::default()
        };

        // Create a new block payload store
        let block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Add the maximum number of blocks to the payload store
        let num_blocks_in_store = max_num_pending_blocks as usize;
        create_and_add_blocks_to_store(block_payload_store.clone(), num_blocks_in_store, 0, true);

        // Verify the number of blocks in the block payload store
        check_num_verified_payloads(&block_payload_store, num_blocks_in_store);

        // Add more blocks to the payload store
        let num_blocks_to_add = 5;
        create_and_add_blocks_to_store(block_payload_store.clone(), num_blocks_to_add, 0, true);

        // Verify the number of blocks in the block payload store
        check_num_verified_payloads(&block_payload_store, max_num_pending_blocks as usize);

        // Add a large number of blocks to the payload store
        let num_blocks_to_add = 100;
        create_and_add_blocks_to_store(block_payload_store.clone(), num_blocks_to_add, 0, true);

        // Verify the number of blocks in the block payload store
        check_num_verified_payloads(&block_payload_store, max_num_pending_blocks as usize);
    }

    #[test]
    fn test_remove_blocks_for_epoch_round() {
        // Create a new block payload store
        let consensus_observer_config = ConsensusObserverConfig::default();
        let block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Add some blocks to the payload store for the current epoch
        let current_epoch = 0;
        let num_blocks_in_store = 100;
        let pipelined_blocks = create_and_add_blocks_to_store(
            block_payload_store.clone(),
            num_blocks_in_store,
            current_epoch,
            true,
        );

        // Remove all the blocks for the given epoch and round
        block_payload_store.remove_blocks_for_epoch_round(current_epoch, 49);

        // Check that the block payload store no longer contains the removed blocks
        let block_transaction_payloads = block_payload_store.get_verified_block_payloads();
        for pipelined_block in pipelined_blocks.iter().take(50) {
            assert!(!block_transaction_payloads
                .lock()
                .contains_key(&(pipelined_block.epoch(), pipelined_block.round())));
        }

        // Verify the number of blocks in the block payload store
        check_num_verified_payloads(&block_payload_store, num_blocks_in_store - 50);

        // Remove all the blocks for the given epoch and round
        block_payload_store
            .remove_blocks_for_epoch_round(current_epoch, num_blocks_in_store as Round);

        // Check that the block payload store no longer contains any blocks
        let block_transaction_payloads = block_payload_store.get_verified_block_payloads();
        assert!(block_transaction_payloads.lock().is_empty());

        // Verify the number of blocks in the block payload store
        check_num_verified_payloads(&block_payload_store, 0);

        // Add some blocks to the payload store for the next epoch
        let next_epoch = 1;
        create_and_add_blocks_to_store(
            block_payload_store.clone(),
            num_blocks_in_store,
            next_epoch,
            true,
        );

        // Remove all the blocks for the future epoch and round
        let future_epoch = 2;
        block_payload_store.remove_blocks_for_epoch_round(future_epoch, 0);

        // Verify the store is now empty
        check_num_verified_payloads(&block_payload_store, 0);
    }

    #[test]
    fn test_remove_committed_blocks() {
        // Create a new block payload store
        let consensus_observer_config = ConsensusObserverConfig::default();
        let block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Add some blocks to the payload store for the current epoch
        let current_epoch = 0;
        let num_blocks_in_store = 100;
        let pipelined_blocks = create_and_add_blocks_to_store(
            block_payload_store.clone(),
            num_blocks_in_store,
            current_epoch,
            true,
        );

        // Remove the first block from the block payload store
        block_payload_store.remove_committed_blocks(&pipelined_blocks[0..1]);

        // Check that the block payload store no longer contains the removed block
        let block_transaction_payloads = block_payload_store.get_verified_block_payloads();
        let removed_block = &pipelined_blocks[0];
        assert!(!block_transaction_payloads
            .lock()
            .contains_key(&(removed_block.epoch(), removed_block.round())));

        // Verify the number of blocks in the block payload store
        check_num_verified_payloads(&block_payload_store, num_blocks_in_store - 1);

        // Remove the last 5 blocks from the block payload store
        block_payload_store.remove_committed_blocks(&pipelined_blocks[5..10]);

        // Check that the block payload store no longer contains the removed blocks
        let block_transaction_payloads = block_payload_store.get_verified_block_payloads();
        for pipelined_block in pipelined_blocks.iter().take(10).skip(5) {
            assert!(!block_transaction_payloads
                .lock()
                .contains_key(&(pipelined_block.epoch(), pipelined_block.round())));
        }

        // Verify the number of blocks in the block payload store
        check_num_verified_payloads(&block_payload_store, num_blocks_in_store - 10);

        // Remove all the blocks from the block payload store (including some that don't exist)
        block_payload_store.remove_committed_blocks(&pipelined_blocks[0..num_blocks_in_store]);

        // Check that the block payload store no longer contains any blocks
        let block_transaction_payloads = block_payload_store.get_verified_block_payloads();
        assert!(block_transaction_payloads.lock().is_empty());

        // Verify the number of blocks in the block payload store
        check_num_verified_payloads(&block_payload_store, 0);

        // Add some blocks to the payload store for the next epoch
        let next_epoch = 1;
        let pipelined_blocks = create_and_add_blocks_to_store(
            block_payload_store.clone(),
            num_blocks_in_store,
            next_epoch,
            true,
        );

        // Remove the last committed block from the future epoch
        block_payload_store.remove_committed_blocks(&pipelined_blocks[99..100]);

        // Check that the block payload store is now empty
        check_num_verified_payloads(&block_payload_store, 0);
    }

    /// Creates and adds the given number of blocks to the block payload store
    fn create_and_add_blocks_to_store(
        mut block_payload_store: BlockPayloadStore,
        num_blocks: usize,
        epoch: u64,
        verified_payload_signatures: bool,
    ) -> Vec<Arc<PipelinedBlock>> {
        let mut pipelined_blocks = vec![];
        for i in 0..num_blocks {
            // Create the block info
            let block_info = BlockInfo::new(
                epoch,
                i as Round,
                HashValue::random(),
                HashValue::random(),
                i as Version,
                i as u64,
                None,
            );

            // Insert the block payload into the store
            block_payload_store.insert_block_payload(
                block_info.clone(),
                BlockTransactionPayload::empty(),
                verified_payload_signatures,
            );

            // Create the equivalent pipelined block
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
            pipelined_blocks.push(pipelined_block.clone());
        }

        pipelined_blocks
    }

    /// Marks the payload of the given block as requested and returns the receiver
    fn mark_payload_as_requested(
        block_payload_store: BlockPayloadStore,
        block: &Arc<PipelinedBlock>,
    ) -> oneshot::Receiver<BlockTransactionPayload> {
        // Get the payload entry for the given block
        let block_payloads = block_payload_store.get_verified_block_payloads();
        let mut block_payloads = block_payloads.lock();
        let block_payload = block_payloads
            .get_mut(&(block.epoch(), block.round()))
            .unwrap();

        // Mark the block payload as requested
        let (payload_sender, payload_receiver) = oneshot::channel();
        *block_payload = BlockPayloadStatus::Requested(payload_sender);

        // Return the payload receiver
        payload_receiver
    }

    /// Checks the number of verified payloads in the block payload store
    fn check_num_verified_payloads(
        block_payload_store: &BlockPayloadStore,
        expected_num_entries: usize,
    ) {
        let num_payloads = block_payload_store
            .verified_block_transaction_payloads
            .lock()
            .len();
        assert_eq!(num_payloads, expected_num_entries);
    }
}
