// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::{
    logging::{LogEntry, LogSchema},
    metrics,
    network_message::{CommitDecision, OrderedBlock},
};
use aptos_config::config::ConsensusObserverConfig;
use aptos_consensus_types::common::Round;
use aptos_infallible::Mutex;
use aptos_logger::{debug, error, warn};
use aptos_types::{
    block_info::BlockInfo, epoch_state::EpochState, ledger_info::LedgerInfoWithSignatures,
};
use std::{collections::BTreeMap, sync::Arc};

/// A simple struct to store the block payloads of ordered and committed blocks
#[derive(Clone)]
pub struct PendingOrderedBlocks {
    // The configuration of the consensus observer
    consensus_observer_config: ConsensusObserverConfig,

    // Verified and unverified pending ordered blocks. The key is the epoch and
    // round of the last block in the ordered block. Each entry contains the
    // block, if the block was verified, and the commit decision (if any).
    pending_blocks:
        Arc<Mutex<BTreeMap<(u64, Round), (OrderedBlock, bool, Option<CommitDecision>)>>>,
}

impl PendingOrderedBlocks {
    pub fn new(consensus_observer_config: ConsensusObserverConfig) -> Self {
        Self {
            consensus_observer_config,
            pending_blocks: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Clears all pending blocks
    pub fn clear_all_pending_blocks(&self) {
        self.pending_blocks.lock().clear();
    }

    /// Returns a copy of the verified pending blocks
    pub fn get_all_verified_pending_blocks(
        &self,
    ) -> BTreeMap<(u64, Round), (OrderedBlock, Option<CommitDecision>)> {
        let mut verified_pending_blocks = BTreeMap::new();
        for (key, (ordered_block, verified_ordered_proof, commit_decision)) in
            self.pending_blocks.lock().iter()
        {
            if *verified_ordered_proof {
                verified_pending_blocks
                    .insert(*key, (ordered_block.clone(), commit_decision.clone()));
            }
        }
        verified_pending_blocks
    }

    /// Returns the last pending ordered block (if any). We take into
    /// account verified and unverified pending blocks (to ensure we're
    /// able to buffer blocks across epoch boundaries).
    pub fn get_last_pending_block(&self) -> Option<BlockInfo> {
        self.pending_blocks
            .lock()
            .last_key_value()
            .map(|(_, (ordered_block, _, _))| ordered_block.last_block().block_info())
    }

    /// Returns the verified pending ordered block (if any)
    pub fn get_verified_pending_block(&self, epoch: u64, round: Round) -> Option<OrderedBlock> {
        self.pending_blocks.lock().get(&(epoch, round)).and_then(
            |(ordered_block, verified_ordered_proof, _)| {
                if *verified_ordered_proof {
                    Some(ordered_block.clone())
                } else {
                    None
                }
            },
        )
    }

    /// Inserts the given ordered block into the pending blocks. This function
    /// assumes the block has already been checked to extend the current pending blocks.
    pub fn insert_ordered_block(&self, ordered_block: OrderedBlock, verified_ordered_proof: bool) {
        // Verify that the number of pending blocks doesn't exceed the maximum
        let max_num_pending_blocks = self.consensus_observer_config.max_num_pending_blocks as usize;
        if self.pending_blocks.lock().len() >= max_num_pending_blocks {
            warn!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Exceeded the maximum number of pending blocks: {:?}. Block verification: {:?}, block: {:?}.",
                    max_num_pending_blocks,
                    verified_ordered_proof,
                    ordered_block.proof_block_info()
                ))
            );
            return; // Drop the block if we've exceeded the maximum
        }

        // Otherwise, we can add the block to the pending blocks
        debug!(
            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                "Adding ordered block to the pending blocks: {}. Verified ordered proof: {:?}",
                verified_ordered_proof,
                ordered_block.proof_block_info()
            ))
        );

        // Get the epoch and round of the last ordered block
        let last_block = ordered_block.last_block();
        let last_block_epoch = last_block.epoch();
        let last_block_round = last_block.round();

        // Insert the pending block
        self.pending_blocks.lock().insert(
            (last_block_epoch, last_block_round),
            (ordered_block, verified_ordered_proof, None),
        );
    }

    /// Removes the pending blocks for the given commit ledger info. This will
    /// remove all blocks up to (and including) the epoch and round of the
    /// commit. Note: this function must remove both verified and unverified
    /// blocks (to support state sync commits).
    pub fn remove_blocks_for_commit(&self, commit_ledger_info: &LedgerInfoWithSignatures) {
        // Determine the epoch and round to split off
        let split_off_epoch = commit_ledger_info.ledger_info().epoch();
        let split_off_round = commit_ledger_info.commit_info().round().saturating_add(1);

        // Remove the blocks from the pending ordered blocks
        let mut pending_blocks = self.pending_blocks.lock();
        *pending_blocks = pending_blocks.split_off(&(split_off_epoch, split_off_round));
    }

    /// Updates the commit decision of the pending ordered block (if found).
    /// This can only be done for verified pending blocks.
    pub fn update_commit_decision(&self, commit_decision: &CommitDecision) {
        // Get the epoch and round of the commit decision
        let commit_decision_epoch = commit_decision.epoch();
        let commit_decision_round = commit_decision.round();

        // Update the commit decision for the verified pending blocks
        let mut pending_blocks = self.pending_blocks.lock();
        if let Some((_, verified_ordered_proof, existing_commit_decision)) =
            pending_blocks.get_mut(&(commit_decision_epoch, commit_decision_round))
        {
            if *verified_ordered_proof {
                *existing_commit_decision = Some(commit_decision.clone());
            } else {
                warn!(
                    LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                        "Attempting to update commit decision for unverified block! Epoch: {:?}, Round: {:?}",
                        commit_decision_epoch,
                        commit_decision_round
                    ))
                );
            }
        }
    }

    /// Updates the metrics for the pending blocks
    pub fn update_pending_blocks_metrics(&self) {
        // Update the number of pending block entries
        let pending_blocks = self.pending_blocks.lock();
        let num_entries = pending_blocks.len() as u64;
        metrics::set_gauge_with_label(
            &metrics::OBSERVER_NUM_PROCESSED_BLOCKS,
            metrics::PENDING_ORDERED_ENTRIES_LABEL,
            num_entries,
        );

        // Update the total number of pending blocks
        let num_pending_blocks = pending_blocks
            .values()
            .map(|(block, _, _)| block.blocks().len() as u64)
            .sum();
        metrics::set_gauge_with_label(
            &metrics::OBSERVER_NUM_PROCESSED_BLOCKS,
            metrics::PENDING_ORDERED_BLOCKS_LABEL,
            num_pending_blocks,
        );

        // Update the highest round for the pending blocks
        let highest_pending_round = pending_blocks
            .last_key_value()
            .map(|(_, (ordered_block, _, _))| ordered_block.last_block().round())
            .unwrap_or(0);
        metrics::set_gauge_with_label(
            &metrics::OBSERVER_PROCESSED_BLOCK_ROUNDS,
            metrics::PENDING_ORDERED_BLOCKS_LABEL,
            highest_pending_round,
        );
    }

    /// Verifies the pending blocks against the given epoch state.
    /// If verification is successful, blocks are marked as verified.
    pub fn verify_pending_blocks(&self, epoch_state: &EpochState) {
        // Get the current epoch
        let current_epoch = epoch_state.epoch;

        // Go through all the pending blocks and verify them
        let mut failed_verification_round = None;
        for ((epoch, round), (ordered_block, verified_ordered_proof, _)) in
            self.pending_blocks.lock().iter_mut()
        {
            // Check if we can return early (BtreeMaps are sorted by key)
            if *epoch > current_epoch {
                return;
            }

            // If the block is not verified, attempt to verify it
            if *epoch == current_epoch && !(*verified_ordered_proof) {
                match ordered_block.verify_ordered_proof(epoch_state) {
                    Ok(_) => {
                        // Mark the block as verified
                        *verified_ordered_proof = true;
                    },
                    Err(error) => {
                        // Log the verification failure
                        error!(
                            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                                "Failed to verify ordered block: {}. Error: {:?}",
                                ordered_block.last_block().block_info(),
                                error
                            ))
                        );

                        // Note the failure and break early
                        failed_verification_round = Some(*round);
                        break;
                    },
                }
            }
        }

        // If verification failed, remove all blocks after (and including) the failure
        if let Some(failed_round) = failed_verification_round {
            self.pending_blocks
                .lock()
                .split_off(&(current_epoch, failed_round));
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_consensus_types::{
        block::Block,
        block_data::{BlockData, BlockType},
        pipelined_block::PipelinedBlock,
        quorum_cert::QuorumCert,
    };
    use aptos_crypto::HashValue;
    use aptos_types::{
        aggregate_signature::AggregateSignature,
        ledger_info::LedgerInfo,
        transaction::Version,
        validator_signer::ValidatorSigner,
        validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
    };

    #[test]
    fn test_clear_all_pending_blocks() {
        // Create new pending ordered blocks
        let pending_ordered_blocks = PendingOrderedBlocks::new(ConsensusObserverConfig::default());

        // Insert several verified blocks for the current epoch
        let current_epoch = 0;
        let num_verified_blocks = 10;
        create_and_add_pending_blocks(
            &pending_ordered_blocks,
            num_verified_blocks,
            current_epoch,
            true,
        );

        // Insert several unverified blocks for the next epoch
        let next_epoch = current_epoch + 1;
        let num_unverified_blocks = 20;
        create_and_add_pending_blocks(
            &pending_ordered_blocks,
            num_unverified_blocks,
            next_epoch,
            false,
        );

        // Clear all pending blocks
        pending_ordered_blocks.clear_all_pending_blocks();

        // Check all the pending blocks were removed
        let num_pending_blocks = pending_ordered_blocks.pending_blocks.lock().len();
        assert_eq!(num_pending_blocks, 0);
    }

    #[test]
    fn test_get_last_pending_block() {
        // Create new pending ordered blocks
        let pending_ordered_blocks = PendingOrderedBlocks::new(ConsensusObserverConfig::default());

        // Verify that we have no last pending block
        assert!(pending_ordered_blocks.get_last_pending_block().is_none());

        // Insert several verified blocks for the current epoch
        let current_epoch = 0;
        let num_verified_blocks = 50;
        let verified_blocks = create_and_add_pending_blocks(
            &pending_ordered_blocks,
            num_verified_blocks,
            current_epoch,
            true,
        );

        // Verify the last pending block is the verified block with the highest round
        let last_verified_block = verified_blocks.last().unwrap();
        let last_verified_block_info = last_verified_block.last_block().block_info();
        assert_eq!(
            last_verified_block_info,
            pending_ordered_blocks.get_last_pending_block().unwrap()
        );

        // Insert several unverified blocks for the next epoch
        let next_epoch = current_epoch + 1;
        let num_unverified_blocks = 50;
        let unverified_blocks = create_and_add_pending_blocks(
            &pending_ordered_blocks,
            num_unverified_blocks,
            next_epoch,
            false,
        );

        // Verify the last pending block is the unverified block with the highest round
        let last_unverified_block = unverified_blocks.last().unwrap();
        let last_unverified_block_info = last_unverified_block.last_block().block_info();
        assert_eq!(
            last_unverified_block_info,
            pending_ordered_blocks.get_last_pending_block().unwrap()
        );

        // Clear the unverified pending blocks
        pending_ordered_blocks
            .pending_blocks
            .lock()
            .retain(|_, (_, verified_ordered_proof, _)| *verified_ordered_proof);

        // Verify the last pending block is the verified block with the highest round
        assert_eq!(
            last_verified_block_info,
            pending_ordered_blocks.get_last_pending_block().unwrap()
        );
    }

    #[test]
    fn test_get_verified_pending_block() {
        // Create new pending ordered blocks
        let pending_ordered_blocks = PendingOrderedBlocks::new(ConsensusObserverConfig::default());

        // Insert several verified blocks for the current epoch
        let current_epoch = 0;
        let num_verified_blocks = 10;
        let verified_blocks = create_and_add_pending_blocks(
            &pending_ordered_blocks,
            num_verified_blocks,
            current_epoch,
            true,
        );

        // Ensure the verified pending blocks were all inserted
        let all_verified_blocks = pending_ordered_blocks.get_all_verified_pending_blocks();
        assert_eq!(all_verified_blocks.len(), num_verified_blocks);

        // Verify the pending blocks can be retrieved
        for verified_block in &verified_blocks {
            let block_info = verified_block.last_block().block_info();
            let pending_block = pending_ordered_blocks
                .get_verified_pending_block(block_info.epoch(), block_info.round())
                .unwrap();
            assert_eq!(verified_block.clone(), pending_block);
        }

        // Verify that a non-existent block cannot be retrieved
        let non_existent_block = verified_blocks.last().unwrap();
        let non_existent_block_info = non_existent_block.last_block().block_info();
        let pending_block = pending_ordered_blocks.get_verified_pending_block(
            non_existent_block_info.epoch(),
            non_existent_block_info.round() + 1, // Request a round that doesn't exist
        );
        assert!(pending_block.is_none());

        // Insert several unverified blocks for the next epoch
        let next_epoch = current_epoch + 1;
        let num_unverified_blocks = 20;
        let unverified_blocks = create_and_add_pending_blocks(
            &pending_ordered_blocks,
            num_unverified_blocks,
            next_epoch,
            false,
        );

        // Ensure the unverified pending blocks cannot be retrieved
        for unverified_block in &unverified_blocks {
            let block_info = unverified_block.last_block().block_info();
            let pending_block = pending_ordered_blocks
                .get_verified_pending_block(block_info.epoch(), block_info.round());
            assert!(pending_block.is_none());
        }
    }

    #[test]
    fn test_insert_ordered_block_limit() {
        // Create a consensus observer config with a maximum of 10 pending blocks
        let max_num_pending_blocks = 10;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            ..ConsensusObserverConfig::default()
        };

        // Create new pending ordered blocks
        let pending_ordered_blocks = PendingOrderedBlocks::new(consensus_observer_config);

        // Insert several verified blocks for the current epoch
        let current_epoch = 0;
        let num_verified_blocks = max_num_pending_blocks * 2; // Insert more than the maximum
        create_and_add_pending_blocks(
            &pending_ordered_blocks,
            num_verified_blocks,
            current_epoch,
            true,
        );

        // Verify the verified pending blocks were inserted up to the maximum
        let all_verified_blocks = pending_ordered_blocks.get_all_verified_pending_blocks();
        assert_eq!(all_verified_blocks.len(), max_num_pending_blocks);

        // Insert several unverified blocks for the next epoch
        let next_epoch = current_epoch + 1;
        let num_unverified_blocks = max_num_pending_blocks - 1; // Insert less than the maximum
        let unverified_blocks = create_and_add_pending_blocks(
            &pending_ordered_blocks,
            num_unverified_blocks,
            next_epoch,
            false,
        );

        // Verify the unverified pending blocks were not inserted
        for unverified_block in &unverified_blocks {
            let block_info = unverified_block.last_block().block_info();
            let pending_block = pending_ordered_blocks
                .get_verified_pending_block(block_info.epoch(), block_info.round());
            assert!(pending_block.is_none());
        }

        // Verify the pending blocks don't exceed the maximum
        let num_pending_blocks = get_num_pending_blocks(&pending_ordered_blocks);
        assert_eq!(num_pending_blocks, max_num_pending_blocks);
    }

    #[test]
    fn test_remove_blocks_for_commit() {
        // Create new pending ordered blocks
        let pending_ordered_blocks = PendingOrderedBlocks::new(ConsensusObserverConfig::default());

        // Insert several verified blocks for the current epoch
        let current_epoch = 10;
        let num_verified_blocks = 10;
        let verified_blocks = create_and_add_pending_blocks(
            &pending_ordered_blocks,
            num_verified_blocks,
            current_epoch,
            true,
        );

        // Insert several unverified blocks for the next epoch
        let next_epoch = current_epoch + 1;
        let num_unverified_blocks = 20;
        let unverified_blocks = create_and_add_pending_blocks(
            &pending_ordered_blocks,
            num_unverified_blocks,
            next_epoch,
            false,
        );

        // Insert additional unverified blocks for a future epoch
        let future_epoch = next_epoch + 1;
        let num_future_blocks = 30;
        create_and_add_pending_blocks(
            &pending_ordered_blocks,
            num_future_blocks,
            future_epoch,
            false,
        );

        // Create a commit decision for the first pending verified block
        let first_verified_block = verified_blocks.first().unwrap();
        let first_verified_block_info = first_verified_block.last_block().block_info();
        let commit_decision = CommitDecision::new(LedgerInfoWithSignatures::new(
            LedgerInfo::new(first_verified_block_info.clone(), HashValue::random()),
            AggregateSignature::empty(),
        ));

        // Remove the pending blocks for the commit decision
        pending_ordered_blocks.remove_blocks_for_commit(commit_decision.commit_proof());

        // Verify the first verified block was removed
        let all_verified_blocks = pending_ordered_blocks.get_all_verified_pending_blocks();
        assert_eq!(all_verified_blocks.len(), num_verified_blocks - 1);
        assert!(!all_verified_blocks.contains_key(&(
            first_verified_block_info.epoch(),
            first_verified_block_info.round()
        )));

        // Create a commit decision for the last pending verified block
        let last_verified_block = verified_blocks.last().unwrap();
        let last_verified_block_info = last_verified_block.last_block().block_info();
        let commit_decision = CommitDecision::new(LedgerInfoWithSignatures::new(
            LedgerInfo::new(last_verified_block_info.clone(), HashValue::random()),
            AggregateSignature::empty(),
        ));

        // Remove the pending blocks for the commit decision
        pending_ordered_blocks.remove_blocks_for_commit(commit_decision.commit_proof());

        // Verify all verified pending blocks were removed
        let all_verified_blocks = pending_ordered_blocks.get_all_verified_pending_blocks();
        assert!(all_verified_blocks.is_empty());

        // Verify the unverified pending blocks were not removed
        let num_pending_blocks = get_num_pending_blocks(&pending_ordered_blocks);
        assert_eq!(
            num_pending_blocks,
            num_unverified_blocks + num_future_blocks
        );

        // Create a commit decision for the last pending unverified block (next epoch)
        let last_unverified_block = unverified_blocks.last().unwrap();
        let last_unverified_block_info = last_unverified_block.last_block().block_info();
        let commit_decision = CommitDecision::new(LedgerInfoWithSignatures::new(
            LedgerInfo::new(last_unverified_block_info.clone(), HashValue::random()),
            AggregateSignature::empty(),
        ));

        // Remove the pending blocks for the commit decision
        pending_ordered_blocks.remove_blocks_for_commit(commit_decision.commit_proof());

        // Verify the unverified pending blocks were removed (next epoch)
        let num_pending_blocks = get_num_pending_blocks(&pending_ordered_blocks);
        assert_eq!(num_pending_blocks, num_future_blocks);

        // Verify the last unverified block was removed (next epoch)
        let pending_blocks = pending_ordered_blocks.pending_blocks.lock();
        assert!(!pending_blocks.contains_key(&(
            last_unverified_block_info.epoch(),
            last_unverified_block_info.round()
        )));
    }

    #[test]
    fn test_update_commit_decision() {
        // Create new pending ordered blocks
        let pending_ordered_blocks = PendingOrderedBlocks::new(ConsensusObserverConfig::default());

        // Insert several verified blocks for the current epoch
        let current_epoch = 0;
        let num_verified_blocks = 10;
        let verified_blocks = create_and_add_pending_blocks(
            &pending_ordered_blocks,
            num_verified_blocks,
            current_epoch,
            true,
        );

        // Insert several unverified blocks for the next epoch
        let next_epoch = current_epoch + 1;
        let num_unverified_blocks = 30;
        let unverified_blocks = create_and_add_pending_blocks(
            &pending_ordered_blocks,
            num_unverified_blocks,
            next_epoch,
            false,
        );

        // Ensure the verified pending blocks were all inserted
        let all_verified_blocks = pending_ordered_blocks.get_all_verified_pending_blocks();
        assert_eq!(all_verified_blocks.len(), num_verified_blocks);

        // Verify the pending blocks don't have any commit decisions
        for (_, (_, commit_decision)) in all_verified_blocks.iter() {
            assert!(commit_decision.is_none());
        }

        // Verify the unverified pending blocks were all inserted
        let num_pending_blocks = get_num_pending_blocks(&pending_ordered_blocks);
        assert_eq!(
            num_pending_blocks,
            num_verified_blocks + num_unverified_blocks
        );

        // Create a commit decision for the first verified block
        let first_verified_block = verified_blocks.first().unwrap();
        let first_verified_block_info = first_verified_block.last_block().block_info();
        let commit_decision = CommitDecision::new(LedgerInfoWithSignatures::new(
            LedgerInfo::new(first_verified_block_info.clone(), HashValue::random()),
            AggregateSignature::empty(),
        ));

        // Update the commit decision for the first verified block
        pending_ordered_blocks.update_commit_decision(&commit_decision);

        // Verify the commit decision was updated
        verify_commit_decision(
            &pending_ordered_blocks,
            &first_verified_block_info,
            commit_decision,
        );

        // Create a commit decision for the last pending block
        let last_pending_block = verified_blocks.last().unwrap();
        let last_pending_block_info = last_pending_block.last_block().block_info();
        let commit_decision = CommitDecision::new(LedgerInfoWithSignatures::new(
            LedgerInfo::new(last_pending_block_info.clone(), HashValue::random()),
            AggregateSignature::empty(),
        ));

        // Update the commit decision for the last pending block
        pending_ordered_blocks.update_commit_decision(&commit_decision);

        // Verify the commit decision was updated
        verify_commit_decision(
            &pending_ordered_blocks,
            &last_pending_block_info,
            commit_decision,
        );

        // Verify the commit decisions for the remaining blocks are still missing
        let all_verified_blocks = pending_ordered_blocks.get_all_verified_pending_blocks();
        for i in 1..9 {
            let (_, commit_decision) = all_verified_blocks.get(&(current_epoch, i as u64)).unwrap();
            assert!(commit_decision.is_none());
        }

        // Create a commit decision for the last unverified pending block
        let last_unverified_block = unverified_blocks.last().unwrap();
        let last_unverified_block_info = last_unverified_block.last_block().block_info();
        let commit_decision = CommitDecision::new(LedgerInfoWithSignatures::new(
            LedgerInfo::new(last_unverified_block_info.clone(), HashValue::random()),
            AggregateSignature::empty(),
        ));

        // Update the commit decision for the last unverified pending block
        pending_ordered_blocks.update_commit_decision(&commit_decision);

        // Verify the commit decision was not updated
        let pending_blocks = pending_ordered_blocks.pending_blocks.lock();
        let (_, _, commit_decision) = pending_blocks
            .get(&(next_epoch, last_unverified_block_info.round()))
            .unwrap();
        assert!(commit_decision.is_none());
    }

    #[test]
    fn test_verify_pending_blocks() {
        // Create new pending ordered blocks
        let pending_ordered_blocks = PendingOrderedBlocks::new(ConsensusObserverConfig::default());

        // Insert several verified blocks for the current epoch
        let current_epoch = 0;
        let num_verified_blocks = 5;
        create_and_add_pending_blocks(
            &pending_ordered_blocks,
            num_verified_blocks,
            current_epoch,
            true,
        );

        // Insert several unverified blocks for the next epoch
        let next_epoch = current_epoch + 1;
        let num_unverified_blocks = 10;
        create_and_add_pending_blocks(
            &pending_ordered_blocks,
            num_unverified_blocks,
            next_epoch,
            false,
        );

        // Insert additional unverified blocks for a future epoch
        let future_epoch = next_epoch + 1;
        let num_future_blocks = 30;
        create_and_add_pending_blocks(
            &pending_ordered_blocks,
            num_future_blocks,
            future_epoch,
            false,
        );

        // Create an epoch state for the next epoch (with an empty verifier)
        let epoch_state = Arc::new(EpochState::new(next_epoch, ValidatorVerifier::new(vec![])));

        // Verify the pending blocks for the next epoch
        pending_ordered_blocks.verify_pending_blocks(&epoch_state);

        // Ensure the verified pending blocks were all inserted
        let all_verified_blocks = pending_ordered_blocks.get_all_verified_pending_blocks();
        assert_eq!(
            all_verified_blocks.len(),
            num_verified_blocks + num_unverified_blocks
        );

        // Create an epoch state for the future epoch (with an empty verifier)
        let epoch_state = EpochState::new(future_epoch, ValidatorVerifier::new(vec![]));

        // Verify the pending blocks for a future epoch
        pending_ordered_blocks.verify_pending_blocks(&epoch_state);

        // Ensure the verified pending blocks were all inserted
        let all_verified_blocks = pending_ordered_blocks.get_all_verified_pending_blocks();
        assert_eq!(
            all_verified_blocks.len(),
            num_verified_blocks + num_unverified_blocks + num_future_blocks
        );

        // Ensure there are no longer any unverified pending blocks
        assert_eq!(
            all_verified_blocks.len(),
            get_num_pending_blocks(&pending_ordered_blocks),
        );
    }

    #[test]
    fn test_verify_pending_blocks_failure() {
        // Create new pending ordered blocks
        let pending_ordered_blocks = PendingOrderedBlocks::new(ConsensusObserverConfig::default());

        // Insert several verified blocks for the current epoch
        let current_epoch = 0;
        let num_verified_blocks = 5;
        create_and_add_pending_blocks(
            &pending_ordered_blocks,
            num_verified_blocks,
            current_epoch,
            true,
        );

        // Insert several unverified blocks for the next epoch
        let next_epoch = current_epoch + 1;
        let num_unverified_blocks = 10;
        create_and_add_pending_blocks(
            &pending_ordered_blocks,
            num_unverified_blocks,
            next_epoch,
            false,
        );

        // Insert additional unverified blocks for a future epoch
        let future_epoch = next_epoch + 1;
        let num_future_blocks = 30;
        create_and_add_pending_blocks(
            &pending_ordered_blocks,
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
        let validator_verified = ValidatorVerifier::new(vec![validator_consensus_info]);
        let epoch_state = EpochState::new(next_epoch, validator_verified);

        // Verify the pending blocks for the next epoch
        pending_ordered_blocks.verify_pending_blocks(&epoch_state);

        // Ensure the unverified pending blocks were not inserted
        let all_verified_blocks = pending_ordered_blocks.get_all_verified_pending_blocks();
        assert_eq!(all_verified_blocks.len(), num_verified_blocks);

        // Ensure the unverified pending blocks were all removed
        let num_pending_blocks = get_num_pending_blocks(&pending_ordered_blocks);
        assert_eq!(num_pending_blocks, num_verified_blocks);
    }

    /// Creates and adds the specified number of pending blocks to the pending ordered blocks
    fn create_and_add_pending_blocks(
        pending_ordered_blocks: &PendingOrderedBlocks,
        num_pending_blocks: usize,
        epoch: u64,
        verified_ordered_proof: bool,
    ) -> Vec<OrderedBlock> {
        let mut pending_blocks = vec![];
        for i in 0..num_pending_blocks {
            // Create a new block info
            let block_info = BlockInfo::new(
                epoch,
                i as aptos_types::block_info::Round,
                HashValue::random(),
                HashValue::random(),
                i as Version,
                i as u64,
                None,
            );

            // Create a pipelined block
            let block_data = BlockData::new_for_testing(
                block_info.epoch(),
                block_info.round(),
                block_info.timestamp_usecs(),
                QuorumCert::dummy(),
                BlockType::Genesis,
            );
            let block = Block::new_for_testing(block_info.id(), block_data, None);
            let pipelined_block = Arc::new(PipelinedBlock::new_ordered(block));

            // Create an ordered block
            let blocks = vec![pipelined_block];
            let ordered_proof = create_ledger_info(epoch, i as Round);
            let ordered_block = OrderedBlock::new(blocks, ordered_proof);

            // Insert the ordered block into the pending ordered blocks
            pending_ordered_blocks
                .insert_ordered_block(ordered_block.clone(), verified_ordered_proof);

            // Add the ordered block to the pending blocks
            pending_blocks.push(ordered_block);
        }

        pending_blocks
    }

    /// Creates and returns a new ledger info with the specified epoch and round
    fn create_ledger_info(epoch: u64, round: Round) -> LedgerInfoWithSignatures {
        LedgerInfoWithSignatures::new(
            LedgerInfo::new(
                BlockInfo::random_with_epoch(epoch, round),
                HashValue::random(),
            ),
            AggregateSignature::empty(),
        )
    }

    /// Returns the number of pending blocks (both verified and unverified)
    fn get_num_pending_blocks(pending_ordered_blocks: &PendingOrderedBlocks) -> usize {
        pending_ordered_blocks.pending_blocks.lock().len()
    }

    /// Verifies the commit decision for the specified block info
    fn verify_commit_decision(
        pending_ordered_blocks: &PendingOrderedBlocks,
        block_info: &BlockInfo,
        commit_decision: CommitDecision,
    ) {
        // Get the commit decision for the block
        let all_verified_blocks = pending_ordered_blocks.get_all_verified_pending_blocks();
        let (_, updated_commit_decision) = all_verified_blocks
            .get(&(block_info.epoch(), block_info.round()))
            .unwrap();

        // Verify the commit decision is expected
        assert_eq!(
            commit_decision,
            updated_commit_decision.as_ref().unwrap().clone()
        );
    }
}
