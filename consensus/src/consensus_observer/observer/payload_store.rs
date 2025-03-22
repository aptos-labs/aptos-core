// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::{
    common::{
        error::Error,
        logging::{LogEntry, LogSchema},
        metrics,
    },
    network::observer_message::{BlockPayload, OrderedBlock},
};
use aptos_config::config::ConsensusObserverConfig;
use aptos_consensus_types::{common::Round, pipelined_block::PipelinedBlock};
use aptos_infallible::Mutex;
use aptos_logger::{error, warn};
use aptos_types::{epoch_state::EpochState, ledger_info::LedgerInfoWithSignatures};
use std::{
    collections::{btree_map::Entry, BTreeMap},
    sync::Arc,
};

/// The status of the block payload
pub enum BlockPayloadStatus {
    AvailableAndVerified(BlockPayload),
    AvailableAndUnverified(BlockPayload),
}

/// A simple struct to store the block payloads of ordered and committed blocks
pub struct BlockPayloadStore {
    // The configuration of the consensus observer
    consensus_observer_config: ConsensusObserverConfig,

    // Block transaction payloads (indexed by epoch and round).
    // This is directly accessed by the payload manager.
    block_payloads: Arc<Mutex<BTreeMap<(u64, Round), BlockPayloadStatus>>>,
}

impl BlockPayloadStore {
    pub fn new(consensus_observer_config: ConsensusObserverConfig) -> Self {
        Self {
            consensus_observer_config,
            block_payloads: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Returns true iff all the payloads for the given blocks
    /// are available and have been verified.
    pub fn all_payloads_exist(&self, blocks: &[Arc<PipelinedBlock>]) -> bool {
        let block_payloads = self.block_payloads.lock();
        blocks.iter().all(|block| {
            let epoch_and_round = (block.epoch(), block.round());
            matches!(
                block_payloads.get(&epoch_and_round),
                Some(BlockPayloadStatus::AvailableAndVerified(_))
            )
        })
    }

    /// Clears all the payloads from the block payload store
    pub fn clear_all_payloads(&self) {
        self.block_payloads.lock().clear();
    }

    /// Returns true iff we already have a payload entry for the given block
    pub fn existing_payload_entry(&self, block_payload: &BlockPayload) -> bool {
        // Get the epoch and round of the payload
        let epoch_and_round = (block_payload.epoch(), block_payload.round());

        // Check if a payload already exists in the store
        self.block_payloads.lock().contains_key(&epoch_and_round)
    }

    /// Returns a reference to the block payloads
    pub fn get_block_payloads(&self) -> Arc<Mutex<BTreeMap<(u64, Round), BlockPayloadStatus>>> {
        self.block_payloads.clone()
    }

    /// Inserts the given block payload data into the payload store
    pub fn insert_block_payload(
        &mut self,
        block_payload: BlockPayload,
        verified_payload_signatures: bool,
    ) {
        // Verify that the number of payloads doesn't exceed the maximum
        let max_num_pending_blocks = self.consensus_observer_config.max_num_pending_blocks as usize;
        if self.block_payloads.lock().len() >= max_num_pending_blocks {
            warn!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Exceeded the maximum number of payloads: {:?}. Dropping block: {:?}!",
                    max_num_pending_blocks,
                    block_payload.block(),
                ))
            );
            return; // Drop the block if we've exceeded the maximum
        }

        // Create the new payload status
        let epoch_and_round = (block_payload.epoch(), block_payload.round());
        let payload_status = if verified_payload_signatures {
            BlockPayloadStatus::AvailableAndVerified(block_payload)
        } else {
            BlockPayloadStatus::AvailableAndUnverified(block_payload)
        };

        // Insert the new payload status
        self.block_payloads
            .lock()
            .insert(epoch_and_round, payload_status);
    }

    /// Removes the block payloads for the given commit ledger info. If
    /// the execution pool window size is None, all payloads up to (and
    /// including) the epoch and round of the commit will be removed.
    /// Otherwise, a buffer of payloads preceding the commit will be retained
    /// (to ensure we have enough payloads to satisfy the execution window).
    pub fn remove_block_payloads_for_commit(
        &mut self,
        commit_ledger_info: &LedgerInfoWithSignatures,
        execution_pool_window_size: Option<u64>,
    ) {
        // Determine the epoch to split off (execution pool doesn't buffer across epochs)
        let split_off_epoch = commit_ledger_info.ledger_info().epoch();

        // Determine the round to split off
        let commit_round = commit_ledger_info.ledger_info().round();
        let split_off_round = if let Some(window_size) = execution_pool_window_size {
            let window_buffer_multiplier = self
                .consensus_observer_config
                .observer_block_window_buffer_multiplier;
            let window_buffer_size = window_size * window_buffer_multiplier;
            if commit_round < window_buffer_size {
                0 // Clear everything from previous epochs
            } else {
                // Retain all payloads in the window buffer
                commit_round
                    .saturating_sub(window_buffer_size)
                    .saturating_add(1)
            }
        } else {
            // Execution pool is disabled. Remove everything up to (and including) the commit round.
            commit_round.saturating_add(1)
        };

        // Remove the blocks from the payload store
        let mut block_payloads = self.block_payloads.lock();
        *block_payloads = block_payloads.split_off(&(split_off_epoch, split_off_round));
    }

    /// Updates the metrics for the payload store
    pub fn update_payload_store_metrics(&self) {
        // Update the number of block payloads
        let num_payloads = self.block_payloads.lock().len() as u64;
        metrics::set_gauge_with_label(
            &metrics::OBSERVER_NUM_PROCESSED_BLOCKS,
            metrics::STORED_PAYLOADS_LABEL,
            num_payloads,
        );

        // Update the highest round for the block payloads
        let highest_round = self
            .block_payloads
            .lock()
            .last_key_value()
            .map(|((_, round), _)| *round)
            .unwrap_or(0);
        metrics::set_gauge_with_label(
            &metrics::OBSERVER_PROCESSED_BLOCK_ROUNDS,
            metrics::STORED_PAYLOADS_LABEL,
            highest_round,
        );
    }

    /// Verifies all block payloads against the given ordered block.
    /// If verification fails, an error is returned.
    pub fn verify_payloads_against_ordered_block(
        &mut self,
        ordered_block: &OrderedBlock,
    ) -> Result<(), Error> {
        // Verify each of the blocks in the ordered block
        for ordered_block in ordered_block.blocks() {
            // Get the block epoch and round
            let block_epoch = ordered_block.epoch();
            let block_round = ordered_block.round();

            // Fetch the block payload
            match self.block_payloads.lock().entry((block_epoch, block_round)) {
                Entry::Occupied(entry) => {
                    // Get the block transaction payload
                    let transaction_payload = match entry.get() {
                        BlockPayloadStatus::AvailableAndVerified(block_payload) => {
                            block_payload.transaction_payload()
                        },
                        BlockPayloadStatus::AvailableAndUnverified(_) => {
                            // The payload should have already been verified
                            return Err(Error::InvalidMessageError(format!(
                                "Payload verification failed! Block payload for epoch: {:?} and round: {:?} is unverified.",
                                ordered_block.epoch(),
                                ordered_block.round()
                            )));
                        },
                    };

                    // Get the ordered block payload
                    let ordered_block_payload = match ordered_block.block().payload() {
                        Some(payload) => payload,
                        None => {
                            return Err(Error::InvalidMessageError(format!(
                                "Payload verification failed! Missing block payload for epoch: {:?} and round: {:?}",
                                ordered_block.epoch(),
                                ordered_block.round()
                            )));
                        },
                    };

                    // Verify the transaction payload against the ordered block payload
                    transaction_payload.verify_against_ordered_payload(ordered_block_payload)?;
                },
                Entry::Vacant(_) => {
                    // The payload is missing (this should never happen)
                    return Err(Error::InvalidMessageError(format!(
                        "Payload verification failed! Missing block payload for epoch: {:?} and round: {:?}",
                        ordered_block.epoch(),
                        ordered_block.round()
                    )));
                },
            }
        }

        Ok(())
    }

    /// Verifies the block payload signatures against the given epoch state.
    /// If verification is successful, blocks are marked as verified.
    pub fn verify_payload_signatures(&mut self, epoch_state: &EpochState) -> Vec<Round> {
        // Get the current epoch
        let current_epoch = epoch_state.epoch;

        // Gather the keys for the block payloads
        let payload_epochs_and_rounds: Vec<(u64, Round)> =
            self.block_payloads.lock().keys().cloned().collect();

        // Go through all unverified blocks and attempt to verify the signatures
        let mut verified_payloads_to_update = vec![];
        for (epoch, round) in payload_epochs_and_rounds {
            // Check if we can break early (BtreeMaps are sorted by key)
            if epoch > current_epoch {
                break;
            }

            // Otherwise, attempt to verify the payload signatures
            if epoch == current_epoch {
                if let Entry::Occupied(mut entry) = self.block_payloads.lock().entry((epoch, round))
                {
                    if let BlockPayloadStatus::AvailableAndUnverified(block_payload) =
                        entry.get_mut()
                    {
                        if let Err(error) = block_payload.verify_payload_signatures(epoch_state) {
                            // Log the verification failure
                            error!(
                                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                                    "Failed to verify the block payload signatures for epoch: {:?} and round: {:?}. Error: {:?}",
                                    epoch, round, error
                                ))
                            );

                            // Remove the block payload from the store
                            entry.remove();
                        } else {
                            // Save the block payload for reinsertion
                            verified_payloads_to_update.push(block_payload.clone());
                        }
                    }
                }
            }
        }

        // Collect the rounds of all newly verified blocks
        let verified_payload_rounds: Vec<Round> = verified_payloads_to_update
            .iter()
            .map(|block_payload| block_payload.round())
            .collect();

        // Update the verified block payloads. Note: this will cause
        // notifications to be sent to any listeners that are waiting.
        for verified_payload in verified_payloads_to_update {
            self.insert_block_payload(verified_payload, true);
        }

        // Return the newly verified payload rounds
        verified_payload_rounds
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::consensus_observer::network::observer_message::BlockTransactionPayload;
    use aptos_bitvec::BitVec;
    use aptos_consensus_types::{
        block::Block,
        block_data::{BlockData, BlockType},
        common::{Author, Payload, ProofWithData},
        pipelined_block::OrderedBlockWindow,
        proof_of_store::{BatchId, BatchInfo, ProofOfStore},
        quorum_cert::QuorumCert,
    };
    use aptos_crypto::HashValue;
    use aptos_types::{
        aggregate_signature::AggregateSignature,
        block_info::{BlockInfo, Round},
        ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
        transaction::Version,
        validator_signer::ValidatorSigner,
        validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
        PeerId,
    };
    use claims::assert_matches;

    #[test]
    fn test_all_payloads_exist() {
        // Create the consensus observer config
        let max_num_pending_blocks = 1000;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks,
            ..ConsensusObserverConfig::default()
        };

        // Create a new block payload store
        let mut block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Add some unverified blocks to the payload store
        let num_blocks_in_store = 100;
        let unverified_blocks =
            create_and_add_blocks_to_store(&mut block_payload_store, num_blocks_in_store, 1, false);

        // Verify the payloads don't exist in the block payload store
        assert!(!block_payload_store.all_payloads_exist(&unverified_blocks));
        assert_eq!(get_num_verified_payloads(&block_payload_store), 0);
        assert_eq!(
            get_num_unverified_payloads(&block_payload_store),
            num_blocks_in_store
        );

        // Add some verified blocks to the payload store
        let num_blocks_in_store = 100;
        let verified_blocks =
            create_and_add_blocks_to_store(&mut block_payload_store, num_blocks_in_store, 0, true);

        // Check that all the payloads exist in the block payload store
        assert!(block_payload_store.all_payloads_exist(&verified_blocks));

        // Check that a subset of the payloads exist in the block payload store
        let subset_verified_blocks = &verified_blocks[0..50];
        assert!(block_payload_store.all_payloads_exist(subset_verified_blocks));

        // Remove some of the payloads from the block payload store
        for block in subset_verified_blocks {
            block_payload_store
                .block_payloads
                .lock()
                .remove(&(block.epoch(), block.round()))
                .unwrap();
        }

        // Check that the payloads no longer exist in the block payload store
        assert!(!block_payload_store.all_payloads_exist(subset_verified_blocks));

        // Check that the remaining payloads still exist in the block payload store
        let subset_verified_blocks = &verified_blocks[50..100];
        assert!(block_payload_store.all_payloads_exist(subset_verified_blocks));

        // Remove the remaining payloads from the block payload store
        for block in subset_verified_blocks {
            block_payload_store
                .block_payloads
                .lock()
                .remove(&(block.epoch(), block.round()))
                .unwrap();
        }

        // Check that the payloads no longer exist in the block payload store
        assert!(!block_payload_store.all_payloads_exist(subset_verified_blocks));
    }

    #[test]
    fn test_all_payloads_exist_unverified() {
        // Create a new block payload store
        let consensus_observer_config = ConsensusObserverConfig::default();
        let mut block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Add several verified blocks to the payload store
        let num_blocks_in_store = 10;
        let verified_blocks =
            create_and_add_blocks_to_store(&mut block_payload_store, num_blocks_in_store, 0, true);

        // Check that the payloads exists in the block payload store
        assert!(block_payload_store.all_payloads_exist(&verified_blocks));

        // Mark the payload of the first block as unverified
        mark_payload_as_unverified(&block_payload_store, &verified_blocks[0]);

        // Check that the payload no longer exists in the block payload store
        assert!(!block_payload_store.all_payloads_exist(&verified_blocks));

        // Check that the remaining payloads still exist in the block payload store
        assert!(block_payload_store.all_payloads_exist(&verified_blocks[1..10]));
    }

    #[test]
    fn test_clear_all_payloads() {
        // Create a new consensus observer config
        let max_num_pending_blocks = 100;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks,
            ..ConsensusObserverConfig::default()
        };

        // Create a new block payload store
        let mut block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Add some unverified blocks to the payload store
        let num_blocks_in_store = 30;
        create_and_add_blocks_to_store(&mut block_payload_store, num_blocks_in_store, 1, false);

        // Add some verified blocks to the payload store
        let verified_blocks =
            create_and_add_blocks_to_store(&mut block_payload_store, num_blocks_in_store, 0, true);

        // Check that the payloads exist in the block payload store
        assert!(block_payload_store.all_payloads_exist(&verified_blocks));

        // Verify the number of blocks in the block payload store
        check_num_unverified_payloads(&block_payload_store, num_blocks_in_store);
        check_num_verified_payloads(&block_payload_store, num_blocks_in_store);

        // Clear all the payloads from the block payload store
        block_payload_store.clear_all_payloads();

        // Check that the payloads no longer exist in the block payload store
        assert!(!block_payload_store.all_payloads_exist(&verified_blocks));

        // Check that the block payload store is empty
        check_num_unverified_payloads(&block_payload_store, 0);
        check_num_verified_payloads(&block_payload_store, 0);
    }

    #[test]
    fn test_existing_payload_entry() {
        // Create a new block payload store
        let consensus_observer_config = ConsensusObserverConfig::default();
        let mut block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Create a new block payload
        let epoch = 10;
        let round = 100;
        let block_payload = create_block_payload(epoch, round);

        // Check that the payload doesn't exist in the block payload store
        assert!(!block_payload_store.existing_payload_entry(&block_payload));

        // Insert the verified block payload into the block payload store
        block_payload_store.insert_block_payload(block_payload.clone(), true);

        // Check that the payload now exists in the block payload store
        assert!(block_payload_store.existing_payload_entry(&block_payload));

        // Create another block payload
        let epoch = 5;
        let round = 101;
        let block_payload = create_block_payload(epoch, round);

        // Check that the payload doesn't exist in the block payload store
        assert!(!block_payload_store.existing_payload_entry(&block_payload));

        // Insert the unverified block payload into the block payload store
        block_payload_store.insert_block_payload(block_payload.clone(), false);

        // Check that the payload now exists in the block payload store
        assert!(block_payload_store.existing_payload_entry(&block_payload));
    }

    #[test]
    fn test_insert_block_payload() {
        // Create a new consensus observer config
        let max_num_pending_blocks = 100;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks,
            ..ConsensusObserverConfig::default()
        };

        // Create a new block payload store
        let mut block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Add some verified blocks to the payload store
        let num_blocks_in_store = 20;
        let verified_blocks =
            create_and_add_blocks_to_store(&mut block_payload_store, num_blocks_in_store, 0, true);

        // Check that the block payload store contains the new block payloads
        assert!(block_payload_store.all_payloads_exist(&verified_blocks));

        // Verify the number of blocks in the block payload store
        check_num_unverified_payloads(&block_payload_store, 0);
        check_num_verified_payloads(&block_payload_store, num_blocks_in_store);

        // Mark the payload of the first block as unverified
        mark_payload_as_unverified(&block_payload_store, &verified_blocks[0]);

        // Check that the payload no longer exists in the block payload store
        assert!(!block_payload_store.all_payloads_exist(&verified_blocks));

        // Verify the number of verified blocks in the block payload store
        check_num_verified_payloads(&block_payload_store, num_blocks_in_store - 1);

        // Insert the same block payload into the block payload store (as verified)
        let transaction_payload = BlockTransactionPayload::empty();
        let block_payload = BlockPayload::new(verified_blocks[0].block_info(), transaction_payload);
        block_payload_store.insert_block_payload(block_payload, true);

        // Check that the block payload store now contains the requested block payload
        assert!(block_payload_store.all_payloads_exist(&verified_blocks));
    }

    #[test]
    fn test_insert_block_payload_limit_verified() {
        // Create a new config observer config
        let max_num_pending_blocks = 10;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks,
            ..ConsensusObserverConfig::default()
        };

        // Create a new block payload store
        let mut block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Add the maximum number of verified blocks to the payload store
        let num_blocks_in_store = max_num_pending_blocks as usize;
        create_and_add_blocks_to_store(&mut block_payload_store, num_blocks_in_store, 0, true);

        // Verify the number of blocks in the block payload store
        check_num_verified_payloads(&block_payload_store, num_blocks_in_store);
        check_num_unverified_payloads(&block_payload_store, 0);

        // Add more blocks to the payload store
        let num_blocks_to_add = 5;
        create_and_add_blocks_to_store(&mut block_payload_store, num_blocks_to_add, 0, true);

        // Verify the number of blocks in the block payload store
        check_num_verified_payloads(&block_payload_store, max_num_pending_blocks as usize);
        check_num_unverified_payloads(&block_payload_store, 0);

        // Add a large number of blocks to the payload store
        let num_blocks_to_add = 100;
        create_and_add_blocks_to_store(&mut block_payload_store, num_blocks_to_add, 0, true);

        // Verify the number of blocks in the block payload store
        check_num_verified_payloads(&block_payload_store, max_num_pending_blocks as usize);
        check_num_unverified_payloads(&block_payload_store, 0);
    }

    #[test]
    fn test_insert_block_payload_limit_unverified() {
        // Create a new config observer config
        let max_num_pending_blocks = 10;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks,
            ..ConsensusObserverConfig::default()
        };

        // Create a new block payload store
        let mut block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Add the maximum number of unverified blocks to the payload store
        let num_blocks_in_store = max_num_pending_blocks as usize;
        create_and_add_blocks_to_store(&mut block_payload_store, num_blocks_in_store, 0, false);

        // Verify the number of blocks in the block payload store
        check_num_unverified_payloads(&block_payload_store, num_blocks_in_store);
        check_num_verified_payloads(&block_payload_store, 0);

        // Add more blocks to the payload store
        let num_blocks_to_add = 5;
        create_and_add_blocks_to_store(&mut block_payload_store, num_blocks_to_add, 0, false);

        // Verify the number of blocks in the block payload store
        check_num_unverified_payloads(&block_payload_store, max_num_pending_blocks as usize);
        check_num_verified_payloads(&block_payload_store, 0);

        // Add a large number of blocks to the payload store
        let num_blocks_to_add = 100;
        create_and_add_blocks_to_store(&mut block_payload_store, num_blocks_to_add, 0, false);

        // Verify the number of blocks in the block payload store
        check_num_unverified_payloads(&block_payload_store, max_num_pending_blocks as usize);
        check_num_verified_payloads(&block_payload_store, 0);
    }

    #[test]
    fn test_remove_payloads_for_commit_execution_pool() {
        // Create a new consensus observer config
        let max_num_pending_blocks = 100;
        let observer_block_window_buffer_multiplier = 2; // Buffer twice the window
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks,
            observer_block_window_buffer_multiplier,
            ..ConsensusObserverConfig::default()
        };

        // Create a new block payload store
        let mut block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Add some verified blocks to the payload store for the current epoch
        let current_epoch = 10;
        let num_blocks_in_store = 50;
        let verified_blocks = create_and_add_blocks_to_store(
            &mut block_payload_store,
            num_blocks_in_store,
            current_epoch,
            true,
        );

        // Process commits for rounds less than the buffer (i.e., < window * 2)
        let window_size = 7;
        let buffer_size = window_size * (observer_block_window_buffer_multiplier as usize);
        for commit_round in 0..buffer_size {
            // Process a commit for the verified block
            let verified_block = verified_blocks.get(commit_round).unwrap();
            let commit_ledger_info = create_ledger_info_for_block(verified_block);
            block_payload_store
                .remove_block_payloads_for_commit(&commit_ledger_info, Some(window_size as u64));

            // Verify the block payload was not removed (it's within the window)
            verify_contains_payload(&block_payload_store, verified_block.clone(), true);
        }

        // Verify that no payloads were removed
        check_num_total_payloads(&block_payload_store, num_blocks_in_store);

        // Process a commit for a round one greater than the buffer
        let commit_round = buffer_size;
        let verified_block = verified_blocks.get(commit_round).unwrap();
        let commit_ledger_info = create_ledger_info_for_block(verified_block);
        block_payload_store
            .remove_block_payloads_for_commit(&commit_ledger_info, Some(window_size as u64));

        // Verify the first payload was removed (it's outside the window)
        verify_contains_payload(&block_payload_store, verified_blocks[0].clone(), false);
        check_num_total_payloads(&block_payload_store, num_blocks_in_store - 1);

        // Process a commit for the last round
        let commit_round = num_blocks_in_store - 1;
        let verified_block = verified_blocks.get(commit_round).unwrap();
        let commit_ledger_info = create_ledger_info_for_block(verified_block);
        block_payload_store
            .remove_block_payloads_for_commit(&commit_ledger_info, Some(window_size as u64));

        // Verify that all payloads before the buffer were removed
        let buffer_start_round = commit_round - buffer_size + 1;
        for removed_round in 0..buffer_start_round {
            let verified_block = verified_blocks.get(removed_round).unwrap();
            verify_contains_payload(&block_payload_store, verified_block.clone(), false);
        }

        // Verify that all payloads after the buffer start were retained
        for retained_round in buffer_start_round..num_blocks_in_store {
            let verified_block = verified_blocks.get(retained_round).unwrap();
            verify_contains_payload(&block_payload_store, verified_block.clone(), true);
        }

        // Verify that only the payloads in the buffer were retained
        check_num_total_payloads(&block_payload_store, buffer_size);
    }

    #[test]
    fn test_remove_payloads_for_commit_execution_pool_epoch() {
        // Create a new consensus observer config
        let max_num_pending_blocks = 300;
        let observer_block_window_buffer_multiplier = 3; // Buffer three times the window
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks,
            observer_block_window_buffer_multiplier,
            ..ConsensusObserverConfig::default()
        };

        // Create a new block payload store
        let mut block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Add some verified blocks to the payload store for the current epoch
        let current_epoch = 15;
        let num_verified_blocks = 50;
        let verified_blocks = create_and_add_blocks_to_store(
            &mut block_payload_store,
            num_verified_blocks,
            current_epoch,
            true,
        );

        // Add some unverified blocks to the payload store for the next epoch
        let next_epoch = current_epoch + 1;
        let num_unverified_blocks_next_epoch = 60;
        let unverified_blocks_next_epoch = create_and_add_blocks_to_store(
            &mut block_payload_store,
            num_unverified_blocks_next_epoch,
            next_epoch,
            false,
        );

        // Add some unverified blocks to the payload store for a future epoch
        let future_epoch = next_epoch + 1;
        let num_unverified_blocks_future_epoch = 70;
        let unverified_blocks_future_epoch = create_and_add_blocks_to_store(
            &mut block_payload_store,
            num_unverified_blocks_future_epoch,
            future_epoch,
            false,
        );

        // Verify the number of payloads (and types)
        check_num_verified_payloads(&block_payload_store, num_verified_blocks);
        check_num_unverified_payloads(
            &block_payload_store,
            num_unverified_blocks_next_epoch + num_unverified_blocks_future_epoch,
        );

        // Process commits for rounds less than the buffer in the next epoch (i.e., < window * 3)
        let window_size = 8;
        let buffer_size = window_size * (observer_block_window_buffer_multiplier as usize);
        for commit_round in 0..buffer_size {
            // Process a commit for the unverified block in the next epoch
            let unverified_block = unverified_blocks_next_epoch.get(commit_round).unwrap();
            let commit_ledger_info = create_ledger_info_for_block(unverified_block);
            block_payload_store
                .remove_block_payloads_for_commit(&commit_ledger_info, Some(window_size as u64));

            // Verify the block payload was not removed (it's within the window)
            verify_contains_payload(&block_payload_store, unverified_block.clone(), true);
        }

        // Verify the verified blocks for the previous epoch were all removed
        for verified_block in &verified_blocks {
            verify_contains_payload(&block_payload_store, verified_block.clone(), false);
        }
        check_num_verified_payloads(&block_payload_store, 0);

        // Process a commit for the last round in the next epoch
        let commit_round = num_unverified_blocks_next_epoch - 1;
        let unverified_block = unverified_blocks_next_epoch.get(commit_round).unwrap();
        let commit_ledger_info = create_ledger_info_for_block(unverified_block);
        block_payload_store
            .remove_block_payloads_for_commit(&commit_ledger_info, Some(window_size as u64));

        // Verify that all payloads before the buffer were removed
        let buffer_start_round = commit_round - buffer_size + 1;
        for removed_round in 0..buffer_start_round {
            let unverified_block = unverified_blocks_next_epoch.get(removed_round).unwrap();
            verify_contains_payload(&block_payload_store, unverified_block.clone(), false);
        }

        // Verify that all payloads after the buffer start were retained
        for retained_round in buffer_start_round..num_unverified_blocks_next_epoch {
            let unverified_block = unverified_blocks_next_epoch.get(retained_round).unwrap();
            verify_contains_payload(&block_payload_store, unverified_block.clone(), true);
        }

        // Verify the number of payloads
        check_num_total_payloads(
            &block_payload_store,
            buffer_size + num_unverified_blocks_future_epoch,
        );

        // Process a commit for the first round in the future epoch
        let unverified_block = unverified_blocks_future_epoch.first().unwrap();
        let commit_ledger_info = create_ledger_info_for_block(unverified_block);
        block_payload_store
            .remove_block_payloads_for_commit(&commit_ledger_info, Some(window_size as u64));

        // Verify that all payloads in the next epoch were removed
        for unverified_block in &unverified_blocks_next_epoch {
            verify_contains_payload(&block_payload_store, unverified_block.clone(), false);
        }

        // Verify that all payloads in the future epoch were retained
        for unverified_block in &unverified_blocks_future_epoch {
            verify_contains_payload(&block_payload_store, unverified_block.clone(), true);
        }

        // Verify the number of payloads
        check_num_total_payloads(&block_payload_store, num_unverified_blocks_future_epoch);
    }

    #[test]
    fn test_remove_blocks_for_commit_execution_pool_windows() {
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
            // Create a new block payload store
            let mut block_payload_store = BlockPayloadStore::new(consensus_observer_config);

            // Add some verified blocks to the payload store for the current epoch
            let current_epoch = 10;
            let num_payloads_in_store = 50;
            let verified_blocks = create_and_add_blocks_to_store(
                &mut block_payload_store,
                num_payloads_in_store,
                current_epoch,
                true,
            );

            // Process commits for rounds less than the buffer (i.e., < window)
            for commit_round in 0..window_size {
                // Process a commit for the verified block
                let verified_block = verified_blocks.get(commit_round).unwrap();
                let commit_ledger_info = create_ledger_info_for_block(verified_block);
                block_payload_store.remove_block_payloads_for_commit(
                    &commit_ledger_info,
                    Some(window_size as u64),
                );

                // Verify the block payload was not removed (it's within the window)
                verify_contains_payload(&block_payload_store, verified_block.clone(), true);
            }

            // Verify that no payloads were removed
            check_num_total_payloads(&block_payload_store, num_payloads_in_store);

            // Process commits for rounds greater than the buffer (i.e., >= window)
            for commit_round in window_size..num_payloads_in_store {
                // Process a commit for the verified block
                let verified_block = verified_blocks.get(commit_round).unwrap();
                let commit_ledger_info = create_ledger_info_for_block(verified_block);
                block_payload_store.remove_block_payloads_for_commit(
                    &commit_ledger_info,
                    Some(window_size as u64),
                );

                // Verify that all blocks before the window were removed
                let window_start_round = commit_round - window_size + 1;
                for removed_round in 0..window_start_round {
                    let verified_block = verified_blocks.get(removed_round).unwrap();
                    verify_contains_payload(&block_payload_store, verified_block.clone(), false);
                }

                // Verify that all blocks after the window start were retained
                for retained_round in window_start_round..num_payloads_in_store {
                    let verified_block = verified_blocks.get(retained_round).unwrap();
                    verify_contains_payload(&block_payload_store, verified_block.clone(), true);
                }
            }
        }
    }

    #[test]
    fn test_remove_payloads_for_commit_verified() {
        // Create a new consensus observer config
        let max_num_pending_blocks = 100;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks,
            ..ConsensusObserverConfig::default()
        };

        // Create a new block payload store
        let mut block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Add some verified blocks to the payload store for the current epoch
        let current_epoch = 0;
        let num_blocks_in_store = 100;
        let verified_blocks = create_and_add_blocks_to_store(
            &mut block_payload_store,
            num_blocks_in_store,
            current_epoch,
            true,
        );

        // Create a commit ledger info for the 50th block in the store
        let commit_block_number = 50;
        let verified_ordered_block = verified_blocks.get(commit_block_number - 1).unwrap();
        let commit_ledger_info = create_ledger_info_for_block(verified_ordered_block);

        // Remove the block payloads for the commit (with execution pool disabled)
        block_payload_store.remove_block_payloads_for_commit(&commit_ledger_info, None);

        // Check that the block payload store no longer contains the removed blocks
        let block_payloads = block_payload_store.get_block_payloads();
        for verified_block in verified_blocks.iter().take(commit_block_number) {
            assert!(!block_payloads
                .lock()
                .contains_key(&(verified_block.epoch(), verified_block.round())));
        }

        // Verify the number of blocks in the block payload store
        check_num_verified_payloads(
            &block_payload_store,
            num_blocks_in_store - commit_block_number,
        );

        // Create a commit ledger info for the last block in the store
        let verified_ordered_block = verified_blocks.get(num_blocks_in_store - 1).unwrap();
        let commit_ledger_info = create_ledger_info_for_block(verified_ordered_block);

        // Remove the block payloads for the commit (with execution pool disabled)
        block_payload_store.remove_block_payloads_for_commit(&commit_ledger_info, None);

        // Check that the block payload store no longer contains any blocks
        let block_payloads = block_payload_store.get_block_payloads();
        assert!(block_payloads.lock().is_empty());

        // Verify the number of blocks in the block payload store
        check_num_verified_payloads(&block_payload_store, 0);

        // Add some verified blocks to the payload store for the next epoch
        let next_epoch = current_epoch + 1;
        create_and_add_blocks_to_store(
            &mut block_payload_store,
            num_blocks_in_store,
            next_epoch,
            true,
        );

        // Create a commit ledger info for a future epoch and round
        let commit_ledger_info = create_empty_ledger_info(next_epoch + 1);

        // Remove the block payloads for the commit (with execution pool disabled)
        block_payload_store.remove_block_payloads_for_commit(&commit_ledger_info, None);

        // Verify the store is now empty
        check_num_verified_payloads(&block_payload_store, 0);
    }

    #[test]
    fn test_remove_payloads_for_commit_unverified() {
        // Create a new consensus observer config
        let max_num_pending_blocks = 100;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks,
            ..ConsensusObserverConfig::default()
        };

        // Create a new block payload store
        let mut block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Add some unverified blocks to the payload store for the current epoch
        let current_epoch = 10;
        let num_blocks_in_store = 100;
        let unverified_blocks = create_and_add_blocks_to_store(
            &mut block_payload_store,
            num_blocks_in_store,
            current_epoch,
            false,
        );

        // Create a commit ledger info for the 50th block in the store
        let commit_block_number = 50;
        let unverified_ordered_block = unverified_blocks.get(commit_block_number - 1).unwrap();
        let commit_ledger_info = create_ledger_info_for_block(unverified_ordered_block);

        // Remove the block payloads for the commit (with execution pool disabled)
        block_payload_store.remove_block_payloads_for_commit(&commit_ledger_info, None);

        // Check that the block payload store no longer contains the removed blocks
        for unverified_block in unverified_blocks.iter().take(commit_block_number) {
            assert!(!block_payload_store
                .block_payloads
                .lock()
                .contains_key(&(unverified_block.epoch(), unverified_block.round())));
        }

        // Verify the number of blocks in the block payload store
        check_num_unverified_payloads(
            &block_payload_store,
            num_blocks_in_store - commit_block_number,
        );

        // Create a commit ledger info for the last block in the store
        let unverified_ordered_block = unverified_blocks.get(num_blocks_in_store - 1).unwrap();
        let commit_ledger_info = create_ledger_info_for_block(unverified_ordered_block);

        // Remove the block payloads for the commit (with execution pool disabled)
        block_payload_store.remove_block_payloads_for_commit(&commit_ledger_info, None);

        // Check that the block payload store no longer contains any blocks
        assert!(block_payload_store.block_payloads.lock().is_empty());

        // Verify the number of blocks in the block payload store
        check_num_unverified_payloads(&block_payload_store, 0);

        // Add some unverified blocks to the payload store for the next epoch
        let next_epoch = current_epoch + 1;
        create_and_add_blocks_to_store(
            &mut block_payload_store,
            num_blocks_in_store,
            next_epoch,
            false,
        );

        // Create a commit ledger info for a future epoch and round
        let commit_ledger_info = create_empty_ledger_info(next_epoch + 1);

        // Remove the block payloads for the commit (with execution pool disabled)
        block_payload_store.remove_block_payloads_for_commit(&commit_ledger_info, None);

        // Verify the store is now empty
        check_num_unverified_payloads(&block_payload_store, 0);
    }

    #[test]
    fn test_verify_payload_signatures() {
        // Create a new consensus observer config
        let max_num_pending_blocks = 100;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks,
            ..ConsensusObserverConfig::default()
        };

        // Create a new block payload store
        let mut block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Add some verified blocks for the current epoch
        let current_epoch = 0;
        let num_verified_blocks = 10;
        create_and_add_blocks_to_store(
            &mut block_payload_store,
            num_verified_blocks,
            current_epoch,
            true,
        );

        // Add some unverified blocks for the next epoch
        let next_epoch = current_epoch + 1;
        let num_unverified_blocks = 20;
        let unverified_blocks = create_and_add_blocks_to_store(
            &mut block_payload_store,
            num_unverified_blocks,
            next_epoch,
            false,
        );

        // Add some unverified blocks for a future epoch
        let future_epoch = current_epoch + 30;
        let num_future_blocks = 30;
        let future_unverified_blocks = create_and_add_blocks_to_store(
            &mut block_payload_store,
            num_future_blocks,
            future_epoch,
            false,
        );

        // Create an epoch state for the next epoch (with an empty verifier)
        let epoch_state = EpochState::new(next_epoch, ValidatorVerifier::new(vec![]));

        // Verify the block payload signatures
        let verified_rounds = block_payload_store.verify_payload_signatures(&epoch_state);

        // Verify the unverified payloads were moved to the verified store
        assert!(block_payload_store.all_payloads_exist(&unverified_blocks));
        assert_eq!(
            get_num_verified_payloads(&block_payload_store),
            num_verified_blocks + num_unverified_blocks
        );
        assert_eq!(
            get_num_unverified_payloads(&block_payload_store),
            num_future_blocks
        );

        // Check the rounds of the newly verified payloads
        let expected_verified_rounds = unverified_blocks
            .iter()
            .map(|block| block.round())
            .collect::<Vec<_>>();
        assert_eq!(verified_rounds, expected_verified_rounds);

        // Create a commit ledger info for the last block in the current epoch
        let verified_ordered_block = unverified_blocks.last().unwrap();
        let commit_ledger_info = create_ledger_info_for_block(verified_ordered_block);

        // Remove the block payloads for the commit (with execution pool disabled)
        block_payload_store.remove_block_payloads_for_commit(&commit_ledger_info, None);

        // Ensure there are no verified payloads in the store
        assert_eq!(get_num_verified_payloads(&block_payload_store), 0);

        // Create an epoch state for the future epoch (with an empty verifier)
        let epoch_state = EpochState::new(future_epoch, ValidatorVerifier::new(vec![]));

        // Verify the block payload signatures for a future epoch
        let verified_rounds = block_payload_store.verify_payload_signatures(&epoch_state);

        // Verify the future unverified payloads were moved to the verified store
        assert!(block_payload_store.all_payloads_exist(&future_unverified_blocks));
        assert_eq!(
            get_num_verified_payloads(&block_payload_store),
            num_future_blocks
        );
        assert_eq!(get_num_unverified_payloads(&block_payload_store), 0);

        // Check the rounds of the newly verified payloads
        let expected_verified_rounds = future_unverified_blocks
            .iter()
            .map(|block| block.round())
            .collect::<Vec<_>>();
        assert_eq!(verified_rounds, expected_verified_rounds);
    }

    #[test]
    fn test_verify_payloads_against_ordered_block() {
        // Create a new block payload store
        let consensus_observer_config = ConsensusObserverConfig::default();
        let mut block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Add some verified blocks for the current epoch
        let current_epoch = 0;
        let num_verified_blocks = 10;
        let verified_blocks = create_and_add_blocks_to_store(
            &mut block_payload_store,
            num_verified_blocks,
            current_epoch,
            true,
        );

        // Create an ordered block using the verified blocks
        let ordered_block = OrderedBlock::new(
            verified_blocks.clone(),
            create_empty_ledger_info(current_epoch),
        );

        // Verify the ordered block and ensure it passes
        block_payload_store
            .verify_payloads_against_ordered_block(&ordered_block)
            .unwrap();

        // Mark the first block payload as unverified
        mark_payload_as_unverified(&block_payload_store, &verified_blocks[0]);

        // Verify the ordered block and ensure it fails (since the payloads are unverified)
        let error = block_payload_store
            .verify_payloads_against_ordered_block(&ordered_block)
            .unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));

        // Clear the block payload store
        block_payload_store.clear_all_payloads();

        // Verify the ordered block and ensure it fails (since the payloads are missing)
        let error = block_payload_store
            .verify_payloads_against_ordered_block(&ordered_block)
            .unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));
    }

    #[test]
    fn test_verify_payload_signatures_failure() {
        // Create a new consensus observer config
        let max_num_pending_blocks = 100;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks,
            ..ConsensusObserverConfig::default()
        };

        // Create a new block payload store
        let mut block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Add some verified blocks for the current epoch
        let current_epoch = 10;
        let num_verified_blocks = 6;
        create_and_add_blocks_to_store(
            &mut block_payload_store,
            num_verified_blocks,
            current_epoch,
            true,
        );

        // Add some unverified blocks for the next epoch
        let next_epoch = current_epoch + 1;
        let num_unverified_blocks = 15;
        let unverified_blocks = create_and_add_blocks_to_store(
            &mut block_payload_store,
            num_unverified_blocks,
            next_epoch,
            false,
        );

        // Add some unverified blocks for a future epoch
        let future_epoch = next_epoch + 1;
        let num_future_blocks = 10;
        let unverified_future_blocks = create_and_add_blocks_to_store(
            &mut block_payload_store,
            num_future_blocks,
            future_epoch,
            false,
        );

        // Create an epoch state for the next epoch (with a non-empty verifier)
        let validator_signer = ValidatorSigner::random(None);
        let validator_consensus_info = ValidatorConsensusInfo::new(
            validator_signer.author(),
            validator_signer.public_key(),
            100,
        );
        let validator_verifier = Arc::new(ValidatorVerifier::new(vec![validator_consensus_info]));
        let epoch_state = EpochState {
            epoch: next_epoch,
            verifier: validator_verifier.clone(),
        };

        // Verify the block payload signatures (for this epoch)
        block_payload_store.verify_payload_signatures(&epoch_state);

        // Ensure the unverified payloads were not verified
        assert!(!block_payload_store.all_payloads_exist(&unverified_blocks));

        // Ensure the unverified payloads were all removed (for this epoch)
        assert_eq!(
            get_num_unverified_payloads(&block_payload_store),
            num_future_blocks
        );

        // Create an epoch state for the future epoch (with a non-empty verifier)
        let epoch_state = EpochState {
            epoch: future_epoch,
            verifier: validator_verifier.clone(),
        };

        // Verify the block payload signatures (for the future epoch)
        block_payload_store.verify_payload_signatures(&epoch_state);

        // Ensure the future unverified payloads were not verified
        assert!(!block_payload_store.all_payloads_exist(&unverified_future_blocks));

        // Ensure the future unverified payloads were all removed (for the future epoch)
        assert_eq!(get_num_unverified_payloads(&block_payload_store), 0);
    }

    /// Creates and adds the given number of blocks to the block payload store
    fn create_and_add_blocks_to_store(
        block_payload_store: &mut BlockPayloadStore,
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

            // Create the block transaction payload with proofs of store
            let mut proofs_of_store = vec![];
            for _ in 0..10 {
                let batch_info = BatchInfo::new(
                    PeerId::random(),
                    BatchId::new(0),
                    epoch,
                    0,
                    HashValue::random(),
                    0,
                    0,
                    0,
                );
                proofs_of_store.push(ProofOfStore::new(batch_info, AggregateSignature::empty()));
            }
            let block_transaction_payload = BlockTransactionPayload::new_quorum_store_inline_hybrid(
                vec![],
                proofs_of_store.clone(),
                None,
                None,
                vec![],
                true,
            );

            // Insert the block payload into the store
            let block_payload = BlockPayload::new(block_info.clone(), block_transaction_payload);
            block_payload_store.insert_block_payload(block_payload, verified_payload_signatures);

            // Create the block type
            let payload = Payload::InQuorumStore(ProofWithData::new(proofs_of_store));
            let block_type = BlockType::DAGBlock {
                author: Author::random(),
                failed_authors: vec![],
                validator_txns: vec![],
                payload,
                node_digests: vec![],
                parent_block_id: HashValue::random(),
                parents_bitvec: BitVec::with_num_bits(0),
            };

            // Create the equivalent pipelined block
            let block_data = BlockData::new_for_testing(
                block_info.epoch(),
                block_info.round(),
                block_info.timestamp_usecs(),
                QuorumCert::dummy(),
                block_type,
            );
            let block = Block::new_for_testing(block_info.id(), block_data, None);
            let pipelined_block = Arc::new(PipelinedBlock::new_ordered(
                block,
                OrderedBlockWindow::empty(),
            ));

            // Add the pipelined block to the list
            pipelined_blocks.push(pipelined_block.clone());
        }

        pipelined_blocks
    }

    /// Creates a new block payload with the given epoch and round
    fn create_block_payload(epoch: u64, round: Round) -> BlockPayload {
        let block_info = BlockInfo::random_with_epoch(epoch, round);
        BlockPayload::new(block_info, BlockTransactionPayload::empty())
    }

    /// Creates and returns a ledger info for the given block
    fn create_ledger_info_for_block(
        verified_ordered_block: &Arc<PipelinedBlock>,
    ) -> LedgerInfoWithSignatures {
        LedgerInfoWithSignatures::new(
            LedgerInfo::new(
                verified_ordered_block.block_info().clone(),
                HashValue::random(),
            ),
            AggregateSignature::empty(),
        )
    }

    /// Checks the number of total payloads in the block payload store
    fn check_num_total_payloads(
        block_payload_store: &BlockPayloadStore,
        expected_num_payloads: usize,
    ) {
        let num_payloads = block_payload_store.get_block_payloads().lock().len();
        assert_eq!(num_payloads, expected_num_payloads);
    }

    /// Checks the number of unverified payloads in the block payload store
    fn check_num_unverified_payloads(
        block_payload_store: &BlockPayloadStore,
        expected_num_payloads: usize,
    ) {
        let num_payloads = get_num_unverified_payloads(block_payload_store);
        assert_eq!(num_payloads, expected_num_payloads);
    }

    /// Checks the number of verified payloads in the block payload store
    fn check_num_verified_payloads(
        block_payload_store: &BlockPayloadStore,
        expected_num_payloads: usize,
    ) {
        let num_payloads = get_num_verified_payloads(block_payload_store);
        assert_eq!(num_payloads, expected_num_payloads);
    }

    /// Creates and returns a new ledger info with an empty signature set
    fn create_empty_ledger_info(epoch: u64) -> LedgerInfoWithSignatures {
        LedgerInfoWithSignatures::new(
            LedgerInfo::new(BlockInfo::random_with_epoch(epoch, 0), HashValue::random()),
            AggregateSignature::empty(),
        )
    }

    /// Returns the number of unverified payloads in the block payload store
    fn get_num_unverified_payloads(block_payload_store: &BlockPayloadStore) -> usize {
        let mut num_unverified_payloads = 0;
        for (_, block_payload_status) in block_payload_store.block_payloads.lock().iter() {
            if let BlockPayloadStatus::AvailableAndUnverified(_) = block_payload_status {
                num_unverified_payloads += 1;
            }
        }
        num_unverified_payloads
    }

    /// Returns the number of verified payloads in the block payload store
    fn get_num_verified_payloads(block_payload_store: &BlockPayloadStore) -> usize {
        let mut num_verified_payloads = 0;
        for (_, block_payload_status) in block_payload_store.block_payloads.lock().iter() {
            if let BlockPayloadStatus::AvailableAndVerified(_) = block_payload_status {
                num_verified_payloads += 1;
            }
        }
        num_verified_payloads
    }

    /// Marks the payload of the given block as unverified
    fn mark_payload_as_unverified(
        block_payload_store: &BlockPayloadStore,
        block: &Arc<PipelinedBlock>,
    ) {
        // Get the payload entry for the given block
        let block_payloads = block_payload_store.get_block_payloads();
        let mut block_payloads = block_payloads.lock();
        let block_payload = block_payloads
            .get_mut(&(block.epoch(), block.round()))
            .unwrap();

        // Mark the block payload as unverified
        *block_payload = BlockPayloadStatus::AvailableAndUnverified(BlockPayload::new(
            block.block_info(),
            BlockTransactionPayload::empty(),
        ));
    }

    /// Verifies the presence of the payload in the block payload store
    fn verify_contains_payload(
        block_payload_store: &BlockPayloadStore,
        block: Arc<PipelinedBlock>,
        expect_contains: bool,
    ) {
        let payload_found = block_payload_store
            .block_payloads
            .lock()
            .contains_key(&(block.epoch(), block.round()));
        assert_eq!(payload_found, expect_contains);
    }
}
