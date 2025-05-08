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
use aptos_types::epoch_state::EpochState;
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

    /// Removes all blocks up to the specified epoch and round (inclusive)
    pub fn remove_blocks_for_epoch_round(&self, epoch: u64, round: Round) {
        // Determine the round to split off
        let split_off_round = round.saturating_add(1);

        // Remove the blocks from the payload store
        let mut block_payloads = self.block_payloads.lock();
        *block_payloads = block_payloads.split_off(&(epoch, split_off_round));
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
        remove_committed_blocks(&mut block_payload_store, subset_verified_blocks);

        // Check that the payloads no longer exist in the block payload store
        assert!(!block_payload_store.all_payloads_exist(subset_verified_blocks));

        // Check that the remaining payloads still exist in the block payload store
        let subset_verified_blocks = &verified_blocks[50..100];
        assert!(block_payload_store.all_payloads_exist(subset_verified_blocks));

        // Remove the remaining payloads from the block payload store
        remove_committed_blocks(&mut block_payload_store, subset_verified_blocks);

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
    fn test_remove_blocks_for_epoch_round_verified() {
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

        // Remove all the blocks for the given epoch and round
        block_payload_store.remove_blocks_for_epoch_round(current_epoch, 49);

        // Check that the block payload store no longer contains the removed blocks
        let block_payloads = block_payload_store.get_block_payloads();
        for verified_block in verified_blocks.iter().take(50) {
            assert!(!block_payloads
                .lock()
                .contains_key(&(verified_block.epoch(), verified_block.round())));
        }

        // Verify the number of blocks in the block payload store
        check_num_verified_payloads(&block_payload_store, num_blocks_in_store - 50);

        // Remove all the blocks for the given epoch and round
        block_payload_store
            .remove_blocks_for_epoch_round(current_epoch, num_blocks_in_store as Round);

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

        // Remove all the blocks for the future epoch and round
        let future_epoch = next_epoch + 1;
        block_payload_store.remove_blocks_for_epoch_round(future_epoch, 0);

        // Verify the store is now empty
        check_num_verified_payloads(&block_payload_store, 0);
    }

    #[test]
    fn test_remove_blocks_for_epoch_round_unverified() {
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

        // Remove all the blocks for the given epoch and round
        block_payload_store.remove_blocks_for_epoch_round(current_epoch, 49);

        // Check that the block payload store no longer contains the removed blocks
        for unverified_block in unverified_blocks.iter().take(50) {
            assert!(!block_payload_store
                .block_payloads
                .lock()
                .contains_key(&(unverified_block.epoch(), unverified_block.round())));
        }

        // Verify the number of blocks in the block payload store
        check_num_unverified_payloads(&block_payload_store, num_blocks_in_store - 50);

        // Remove all the blocks for the given epoch and round
        block_payload_store
            .remove_blocks_for_epoch_round(current_epoch, num_blocks_in_store as Round);

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

        // Remove all the blocks for the future epoch and round
        let future_epoch = next_epoch + 10;
        block_payload_store.remove_blocks_for_epoch_round(future_epoch, 0);

        // Verify the store is now empty
        check_num_unverified_payloads(&block_payload_store, 0);
    }

    #[test]
    fn test_remove_committed_blocks_verified() {
        // Create a new consensus observer config
        let max_num_pending_blocks = 100;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks,
            ..ConsensusObserverConfig::default()
        };

        // Create a new block payload store
        let mut block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Add some blocks to the payload store for the current epoch
        let current_epoch = 0;
        let num_blocks_in_store = 100;
        let verified_blocks = create_and_add_blocks_to_store(
            &mut block_payload_store,
            num_blocks_in_store,
            current_epoch,
            true,
        );

        // Remove the first block from the block payload store
        remove_committed_blocks(&mut block_payload_store, &verified_blocks[0..1]);

        // Check that the block payload store no longer contains the removed block
        let block_payloads = block_payload_store.get_block_payloads();
        let removed_block = &verified_blocks[0];
        assert!(!block_payloads
            .lock()
            .contains_key(&(removed_block.epoch(), removed_block.round())));

        // Verify the number of blocks in the block payload store
        check_num_verified_payloads(&block_payload_store, num_blocks_in_store - 1);

        // Remove the last 5 blocks from the block payload store
        remove_committed_blocks(&mut block_payload_store, &verified_blocks[5..10]);

        // Check that the block payload store no longer contains the removed blocks
        let block_payloads = block_payload_store.get_block_payloads();
        for verified_block in verified_blocks.iter().take(10).skip(5) {
            assert!(!block_payloads
                .lock()
                .contains_key(&(verified_block.epoch(), verified_block.round())));
        }

        // Verify the number of blocks in the block payload store
        check_num_verified_payloads(&block_payload_store, num_blocks_in_store - 10);

        // Remove all the blocks from the block payload store (including some that don't exist)
        remove_committed_blocks(
            &mut block_payload_store,
            &verified_blocks[0..num_blocks_in_store],
        );

        // Check that the block payload store no longer contains any blocks
        let block_payloads = block_payload_store.get_block_payloads();
        assert!(block_payloads.lock().is_empty());

        // Verify the number of blocks in the block payload store
        check_num_verified_payloads(&block_payload_store, 0);

        // Add some blocks to the payload store for the next epoch
        let next_epoch = 1;
        let verified_blocks = create_and_add_blocks_to_store(
            &mut block_payload_store,
            num_blocks_in_store,
            next_epoch,
            true,
        );

        // Remove the last committed block from the future epoch
        remove_committed_blocks(&mut block_payload_store, &verified_blocks[99..100]);

        // Check that the block payload store is now empty
        check_num_verified_payloads(&block_payload_store, 0);
    }

    #[test]
    fn test_remove_committed_blocks_unverified() {
        // Create a new consensus observer config
        let max_num_pending_blocks = 100;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks,
            ..ConsensusObserverConfig::default()
        };

        // Create a new block payload store
        let mut block_payload_store = BlockPayloadStore::new(consensus_observer_config);

        // Add some blocks to the payload store for the current epoch
        let current_epoch = 10;
        let num_blocks_in_store = 100;
        let unverified_blocks = create_and_add_blocks_to_store(
            &mut block_payload_store,
            num_blocks_in_store,
            current_epoch,
            false,
        );

        // Remove the first block from the block payload store
        remove_committed_blocks(&mut block_payload_store, &unverified_blocks[0..1]);

        // Check that the block payload store no longer contains the removed block
        let removed_block = &unverified_blocks[0];
        assert!(!block_payload_store
            .block_payloads
            .lock()
            .contains_key(&(removed_block.epoch(), removed_block.round())));

        // Verify the number of blocks in the block payload store
        check_num_unverified_payloads(&block_payload_store, num_blocks_in_store - 1);

        // Remove the last 5 blocks from the block payload store
        remove_committed_blocks(&mut block_payload_store, &unverified_blocks[5..10]);

        // Check that the block payload store no longer contains the removed blocks
        for verified_block in unverified_blocks.iter().take(10).skip(5) {
            assert!(!block_payload_store
                .block_payloads
                .lock()
                .contains_key(&(verified_block.epoch(), verified_block.round())));
        }

        // Verify the number of blocks in the block payload store
        check_num_unverified_payloads(&block_payload_store, num_blocks_in_store - 10);

        // Remove all the blocks from the block payload store (including some that don't exist)
        remove_committed_blocks(
            &mut block_payload_store,
            &unverified_blocks[0..num_blocks_in_store],
        );

        // Check that the block payload store no longer contains any blocks
        assert!(block_payload_store.block_payloads.lock().is_empty());

        // Verify the number of blocks in the block payload store
        check_num_unverified_payloads(&block_payload_store, 0);

        // Add some blocks to the payload store for the next epoch
        let next_epoch = 11;
        let unverified_blocks = create_and_add_blocks_to_store(
            &mut block_payload_store,
            num_blocks_in_store,
            next_epoch,
            false,
        );

        // Remove the last committed block from the future epoch
        remove_committed_blocks(&mut block_payload_store, &unverified_blocks[99..100]);

        // Check that the block payload store is now empty
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

        // Clear the verified blocks and check the verified blocks are empty
        remove_committed_blocks(&mut block_payload_store, &unverified_blocks);
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

    /// Removes the committed blocks from the payload store
    fn remove_committed_blocks(
        block_payload_store: &mut BlockPayloadStore,
        committed_blocks: &[Arc<PipelinedBlock>],
    ) {
        for committed_block in committed_blocks {
            block_payload_store
                .remove_blocks_for_epoch_round(committed_block.epoch(), committed_block.round());
        }
    }
}
