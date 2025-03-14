// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::{
    common::{
        logging::{LogEntry, LogSchema},
        metrics,
    },
    network::observer_message::CommitDecision,
    observer::execution_pool::ObservedOrderedBlock,
};
use aptos_config::config::ConsensusObserverConfig;
use aptos_consensus_types::{common::Round, pipelined_block::PipelinedBlock};
use aptos_logger::{debug, warn};
use aptos_types::ledger_info::LedgerInfoWithSignatures;
use std::{collections::BTreeMap, sync::Arc};

/// A simple struct to store ordered blocks
pub struct OrderedBlockStore {
    // The configuration of the consensus observer
    consensus_observer_config: ConsensusObserverConfig,

    // The highest committed block (epoch and round)
    highest_committed_epoch_round: Option<(u64, Round)>,

    // Ordered blocks. The key is the epoch and round of the last block in the
    // ordered block. Each entry contains the block and the commit decision (if any).
    ordered_blocks: BTreeMap<(u64, Round), (ObservedOrderedBlock, Option<CommitDecision>)>,
}

impl OrderedBlockStore {
    pub fn new(consensus_observer_config: ConsensusObserverConfig) -> Self {
        Self {
            consensus_observer_config,
            highest_committed_epoch_round: None,
            ordered_blocks: BTreeMap::new(),
        }
    }

    /// Clears all ordered blocks
    pub fn clear_all_ordered_blocks(&mut self) {
        self.ordered_blocks.clear();
    }

    /// Returns a copy of the ordered blocks
    pub fn get_all_ordered_blocks(
        &self,
    ) -> BTreeMap<(u64, Round), (ObservedOrderedBlock, Option<CommitDecision>)> {
        self.ordered_blocks.clone()
    }

    /// Returns the highest committed epoch and round (if any)
    pub fn get_highest_committed_epoch_round(&self) -> Option<(u64, Round)> {
        self.highest_committed_epoch_round
    }

    /// Returns the last ordered block (if any)
    pub fn get_last_ordered_block(&self) -> Option<Arc<PipelinedBlock>> {
        self.ordered_blocks
            .last_key_value()
            .map(|(_, (observed_ordered_block, _))| {
                observed_ordered_block.ordered_block().last_block()
            })
    }

    /// Returns the observed ordered block for the given epoch and round (if any)
    pub fn get_observed_ordered_block(
        &self,
        epoch: u64,
        round: Round,
    ) -> Option<ObservedOrderedBlock> {
        self.ordered_blocks
            .get(&(epoch, round))
            .map(|(observed_ordered_block, _)| observed_ordered_block.clone())
    }

    /// Inserts the given ordered block into the ordered blocks. This function
    /// assumes the block has already been checked to extend the current ordered
    /// blocks, and that the ordered proof has been verified.
    pub fn insert_ordered_block(&mut self, observed_ordered_block: ObservedOrderedBlock) {
        // Verify that the number of ordered blocks doesn't exceed the maximum
        let max_num_ordered_blocks = self.consensus_observer_config.max_num_pending_blocks as usize;
        if self.ordered_blocks.len() >= max_num_ordered_blocks {
            warn!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Exceeded the maximum number of ordered blocks: {:?}. Dropping block: {:?}.",
                    max_num_ordered_blocks,
                    observed_ordered_block.ordered_block().proof_block_info()
                ))
            );
            return; // Drop the block if we've exceeded the maximum
        }

        // Otherwise, we can add the block to the ordered blocks
        debug!(
            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                "Adding ordered block to the ordered blocks: {:?}",
                observed_ordered_block.ordered_block().proof_block_info()
            ))
        );

        // Get the epoch and round of the last ordered block
        let last_block = observed_ordered_block.ordered_block().last_block();
        let last_block_epoch = last_block.epoch();
        let last_block_round = last_block.round();

        // Insert the ordered block
        self.ordered_blocks.insert(
            (last_block_epoch, last_block_round),
            (observed_ordered_block, None),
        );
    }

    /// Removes the ordered blocks for the given commit ledger info. If
    /// the execution pool window size is None, all blocks up to (and
    /// including) the epoch and round of the commit will be removed.
    /// Otherwise, a buffer of blocks preceding the commit will be retained
    /// (to ensure we have enough blocks to satisfy the execution window).
    pub fn remove_blocks_for_commit(
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
                // Retain all blocks in the window buffer
                commit_round
                    .saturating_sub(window_buffer_size)
                    .saturating_add(1)
            }
        } else {
            // Execution pool is disabled. Remove everything up to (and including) the commit round.
            commit_round.saturating_add(1)
        };

        // Remove the blocks from the ordered blocks
        self.ordered_blocks = self
            .ordered_blocks
            .split_off(&(split_off_epoch, split_off_round));

        // Update the highest committed epoch and round
        self.update_highest_committed_epoch_round(commit_ledger_info);
    }

    /// Updates the commit decision of the ordered block (if found)
    pub fn update_commit_decision(&mut self, commit_decision: &CommitDecision) {
        // Get the epoch and round of the commit decision
        let commit_decision_epoch = commit_decision.epoch();
        let commit_decision_round = commit_decision.round();

        // Update the commit decision for the ordered blocks
        if let Some((_, existing_commit_decision)) = self
            .ordered_blocks
            .get_mut(&(commit_decision_epoch, commit_decision_round))
        {
            *existing_commit_decision = Some(commit_decision.clone());
        }

        // Update the highest committed epoch and round
        self.update_highest_committed_epoch_round(commit_decision.commit_proof());
    }

    /// Updates the highest committed epoch and round based on the commit ledger info
    fn update_highest_committed_epoch_round(
        &mut self,
        commit_ledger_info: &LedgerInfoWithSignatures,
    ) {
        // Get the epoch and round of the commit ledger info
        let commit_epoch = commit_ledger_info.ledger_info().epoch();
        let commit_round = commit_ledger_info.commit_info().round();
        let commit_epoch_round = (commit_epoch, commit_round);

        // Update the highest committed epoch and round (if appropriate)
        match self.highest_committed_epoch_round {
            Some(highest_committed_epoch_round) => {
                if commit_epoch_round > highest_committed_epoch_round {
                    self.highest_committed_epoch_round = Some(commit_epoch_round);
                }
            },
            None => {
                self.highest_committed_epoch_round = Some(commit_epoch_round);
            },
        }
    }

    /// Updates the metrics for the ordered blocks
    pub fn update_ordered_blocks_metrics(&self) {
        // Update the number of ordered block entries
        let num_entries = self.ordered_blocks.len() as u64;
        metrics::set_gauge_with_label(
            &metrics::OBSERVER_NUM_PROCESSED_BLOCKS,
            metrics::ORDERED_BLOCK_ENTRIES_LABEL,
            num_entries,
        );

        // Update the total number of ordered blocks
        let num_ordered_blocks = self
            .ordered_blocks
            .values()
            .map(|(observed_ordered_block, _)| {
                observed_ordered_block.ordered_block().blocks().len() as u64
            })
            .sum();
        metrics::set_gauge_with_label(
            &metrics::OBSERVER_NUM_PROCESSED_BLOCKS,
            metrics::ORDERED_BLOCK_LABEL,
            num_ordered_blocks,
        );

        // Update the highest round for the ordered blocks
        let highest_ordered_round = self
            .ordered_blocks
            .last_key_value()
            .map(|(_, (observed_ordered_block, _))| {
                observed_ordered_block.ordered_block().last_block().round()
            })
            .unwrap_or(0);
        metrics::set_gauge_with_label(
            &metrics::OBSERVER_PROCESSED_BLOCK_ROUNDS,
            metrics::ORDERED_BLOCK_LABEL,
            highest_ordered_round,
        );

        // Update the highest round for the committed blocks
        let highest_committed_round = self
            .highest_committed_epoch_round
            .map(|(_, round)| round)
            .unwrap_or(0);
        metrics::set_gauge_with_label(
            &metrics::OBSERVER_PROCESSED_BLOCK_ROUNDS,
            metrics::COMMITTED_BLOCKS_LABEL,
            highest_committed_round,
        );
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::consensus_observer::network::observer_message::OrderedBlock;
    use aptos_consensus_types::{
        block::Block,
        block_data::{BlockData, BlockType},
        pipelined_block::PipelinedBlock,
        quorum_cert::QuorumCert,
    };
    use aptos_crypto::HashValue;
    use aptos_types::{
        aggregate_signature::AggregateSignature, block_info::BlockInfo, ledger_info::LedgerInfo,
        transaction::Version,
    };
    use std::sync::Arc;

    #[test]
    fn test_clear_all_ordered_blocks() {
        // Create a new ordered block store
        let mut ordered_block_store = OrderedBlockStore::new(ConsensusObserverConfig::default());

        // Insert several ordered blocks for the current epoch
        let current_epoch = 0;
        let num_ordered_blocks = 10;
        create_and_add_ordered_blocks(&mut ordered_block_store, num_ordered_blocks, current_epoch);

        // Clear all ordered blocks
        ordered_block_store.clear_all_ordered_blocks();

        // Check that all the ordered blocks were removed
        assert!(ordered_block_store.ordered_blocks.is_empty());
    }

    #[test]
    fn test_get_highest_committed_epoch_round() {
        // Create a new ordered block store
        let mut ordered_block_store = OrderedBlockStore::new(ConsensusObserverConfig::default());

        // Verify that we have no highest committed epoch and round
        assert!(ordered_block_store
            .get_highest_committed_epoch_round()
            .is_none());

        // Insert several ordered blocks for the current epoch
        let current_epoch = 10;
        let num_ordered_blocks = 50;
        let ordered_blocks = create_and_add_ordered_blocks(
            &mut ordered_block_store,
            num_ordered_blocks,
            current_epoch,
        );

        // Create a commit decision for the first ordered block
        let first_ordered_block = ordered_blocks.first().unwrap();
        let (first_ordered_block_info, commit_decision) =
            create_commit_decision_for_block(first_ordered_block);

        // Update the commit decision for the first ordered block
        ordered_block_store.update_commit_decision(&commit_decision);

        // Verify the highest committed epoch and round is the first ordered block
        verify_highest_committed_epoch_round(&ordered_block_store, &first_ordered_block_info);

        // Create a commit decision for the last ordered block
        let last_ordered_block = ordered_blocks.last().unwrap();
        let (last_ordered_block_info, commit_decision) =
            create_commit_decision_for_block(last_ordered_block);

        // Update the commit decision for the last ordered block
        ordered_block_store.update_commit_decision(&commit_decision);

        // Verify the highest committed epoch and round is the last ordered block
        verify_highest_committed_epoch_round(&ordered_block_store, &last_ordered_block_info);

        // Insert several ordered blocks for the next epoch
        let next_epoch = current_epoch + 1;
        let num_ordered_blocks = 10;
        let ordered_blocks =
            create_and_add_ordered_blocks(&mut ordered_block_store, num_ordered_blocks, next_epoch);

        // Verify the highest committed epoch and round is still the last ordered block
        verify_highest_committed_epoch_round(&ordered_block_store, &last_ordered_block_info);

        // Create a commit decision for the first ordered block (in the next epoch)
        let first_ordered_block = ordered_blocks.first().unwrap();
        let (first_ordered_block_info, commit_decision) =
            create_commit_decision_for_block(first_ordered_block);

        // Update the commit decision for the first ordered block
        ordered_block_store.update_commit_decision(&commit_decision);

        // Verify the highest committed epoch and round is the first ordered block (in the next epoch)
        verify_highest_committed_epoch_round(&ordered_block_store, &first_ordered_block_info);

        // Create a commit decision for the last ordered block (in the next epoch)
        let last_ordered_block = ordered_blocks.last().unwrap();
        let (last_ordered_block_info, commit_decision) =
            create_commit_decision_for_block(last_ordered_block);

        // Remove the ordered blocks for the commit decision (without an execution pool window)
        ordered_block_store.remove_blocks_for_commit(commit_decision.commit_proof(), None);

        // Verify the highest committed epoch and round is the last ordered block (in the next epoch)
        verify_highest_committed_epoch_round(&ordered_block_store, &last_ordered_block_info);

        // Create a commit decision for an out-of-date ordered block
        let out_of_date_ordered_block = ordered_blocks.first().unwrap();
        let (_, commit_decision) = create_commit_decision_for_block(out_of_date_ordered_block);

        // Update the commit decision for the out-of-date ordered block
        ordered_block_store.update_commit_decision(&commit_decision);

        // Verify the highest committed epoch and round is still the last ordered block (in the next epoch)
        verify_highest_committed_epoch_round(&ordered_block_store, &last_ordered_block_info);
    }

    #[test]
    fn test_get_last_ordered_block() {
        // Create a new consensus observer config
        let max_num_pending_blocks = 100;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks,
            ..ConsensusObserverConfig::default()
        };

        // Create a new ordered block store
        let mut ordered_block_store = OrderedBlockStore::new(consensus_observer_config);

        // Verify that we have no last ordered block
        assert!(ordered_block_store.get_last_ordered_block().is_none());

        // Insert several ordered blocks for the current epoch
        let current_epoch = 0;
        let num_ordered_blocks = 50;
        let ordered_blocks = create_and_add_ordered_blocks(
            &mut ordered_block_store,
            num_ordered_blocks,
            current_epoch,
        );

        // Verify the last ordered block is the block with the highest round
        let last_ordered_block = ordered_blocks.last().unwrap();
        let last_ordered_block_info = last_ordered_block.last_block().block_info();
        assert_eq!(
            last_ordered_block_info,
            ordered_block_store
                .get_last_ordered_block()
                .unwrap()
                .block_info()
        );

        // Insert several ordered blocks for the next epoch
        let next_epoch = current_epoch + 1;
        let num_ordered_blocks = 50;
        let ordered_blocks =
            create_and_add_ordered_blocks(&mut ordered_block_store, num_ordered_blocks, next_epoch);

        // Verify the last ordered block is the block with the highest epoch and round
        let last_ordered_block = ordered_blocks.last().unwrap();
        let last_ordered_block_info = last_ordered_block.last_block().block_info();
        assert_eq!(
            last_ordered_block_info,
            ordered_block_store
                .get_last_ordered_block()
                .unwrap()
                .block_info()
        );
    }

    #[test]
    fn test_get_ordered_block() {
        // Create a new consensus observer config
        let max_num_pending_blocks = 100;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks,
            ..ConsensusObserverConfig::default()
        };

        // Create a new ordered block store
        let mut ordered_block_store = OrderedBlockStore::new(consensus_observer_config);

        // Insert several ordered blocks for the current epoch
        let current_epoch = 0;
        let num_ordered_blocks = 50;
        let ordered_blocks = create_and_add_ordered_blocks(
            &mut ordered_block_store,
            num_ordered_blocks,
            current_epoch,
        );

        // Ensure the ordered blocks were all inserted
        verify_num_ordered_blocks(&ordered_block_store, num_ordered_blocks);

        // Verify the ordered blocks can be retrieved
        for ordered_block in &ordered_blocks {
            let block_info = ordered_block.last_block().block_info();
            let fetched_ordered_block = ordered_block_store
                .get_observed_ordered_block(block_info.epoch(), block_info.round())
                .unwrap();
            assert_eq!(ordered_block, fetched_ordered_block.ordered_block());
        }

        // Verify that a non-existent block cannot be retrieved
        let last_block = ordered_blocks.last().unwrap();
        let last_block_info = last_block.last_block().block_info();
        let ordered_block = ordered_block_store.get_observed_ordered_block(
            last_block_info.epoch(),
            last_block_info.round() + 1, // Request a round that doesn't exist
        );
        assert!(ordered_block.is_none());
    }

    #[test]
    fn test_insert_ordered_block_limit() {
        // Create a consensus observer config with a maximum of 10 pending blocks
        let max_num_pending_blocks = 10;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks: max_num_pending_blocks as u64,
            ..ConsensusObserverConfig::default()
        };

        // Create a new ordered block store
        let mut ordered_block_store = OrderedBlockStore::new(consensus_observer_config);

        // Insert several ordered blocks for the current epoch
        let current_epoch = 0;
        let num_ordered_blocks = max_num_pending_blocks * 2; // Insert more than the maximum
        create_and_add_ordered_blocks(&mut ordered_block_store, num_ordered_blocks, current_epoch);

        // Verify the ordered blocks were inserted up to the maximum
        verify_num_ordered_blocks(&ordered_block_store, max_num_pending_blocks);

        // Insert several ordered blocks for the next epoch
        let next_epoch = current_epoch + 1;
        let num_ordered_blocks = max_num_pending_blocks - 1; // Insert one less than the maximum
        let ordered_blocks =
            create_and_add_ordered_blocks(&mut ordered_block_store, num_ordered_blocks, next_epoch);

        // Verify the ordered blocks were not inserted (they should have just been dropped)
        for ordered_block in &ordered_blocks {
            let block_info = ordered_block.last_block().block_info();
            let fetched_ordered_block = ordered_block_store
                .get_observed_ordered_block(block_info.epoch(), block_info.round());
            assert!(fetched_ordered_block.is_none());
        }

        // Verify the ordered blocks don't exceed the maximum
        verify_num_ordered_blocks(&ordered_block_store, max_num_pending_blocks);
    }

    #[test]
    fn test_remove_blocks_for_commit() {
        // Create a new consensus observer config
        let max_num_pending_blocks = 100;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks,
            ..ConsensusObserverConfig::default()
        };

        // Create a new ordered block store
        let mut ordered_block_store = OrderedBlockStore::new(consensus_observer_config);

        // Insert several ordered blocks for the current epoch
        let current_epoch = 10;
        let num_ordered_blocks = 10;
        let ordered_blocks = create_and_add_ordered_blocks(
            &mut ordered_block_store,
            num_ordered_blocks,
            current_epoch,
        );

        // Insert several ordered blocks for the next epoch
        let next_epoch = current_epoch + 1;
        let num_ordered_blocks_next_epoch = 20;
        let ordered_blocks_next_epoch = create_and_add_ordered_blocks(
            &mut ordered_block_store,
            num_ordered_blocks_next_epoch,
            next_epoch,
        );

        // Insert several ordered blocks for a future epoch
        let future_epoch = next_epoch + 1;
        let num_ordered_blocks_future_epoch = 30;
        create_and_add_ordered_blocks(
            &mut ordered_block_store,
            num_ordered_blocks_future_epoch,
            future_epoch,
        );

        // Create a commit decision for the first ordered block
        let first_ordered_block = ordered_blocks.first().unwrap();
        let (_, commit_decision) = create_commit_decision_for_block(first_ordered_block);

        // Remove the ordered blocks for the commit decision (without an execution pool window)
        ordered_block_store.remove_blocks_for_commit(commit_decision.commit_proof(), None);

        // Verify the first ordered block was removed
        verify_contains_block(&ordered_block_store, first_ordered_block, false);
        verify_num_ordered_blocks(
            &ordered_block_store,
            num_ordered_blocks + num_ordered_blocks_next_epoch + num_ordered_blocks_future_epoch
                - 1,
        );

        // Create a commit decision for the last ordered block (in the current epoch)
        let last_ordered_block = ordered_blocks.last().unwrap();
        let (_, commit_decision) = create_commit_decision_for_block(last_ordered_block);

        // Remove the ordered blocks for the commit decision (without an execution pool window)
        ordered_block_store.remove_blocks_for_commit(commit_decision.commit_proof(), None);

        // Verify the ordered blocks for the current epoch were removed
        for ordered_block in ordered_blocks {
            verify_contains_block(&ordered_block_store, &ordered_block, false);
        }
        verify_num_ordered_blocks(
            &ordered_block_store,
            num_ordered_blocks_next_epoch + num_ordered_blocks_future_epoch,
        );

        // Create a commit decision for the last ordered block (in the next epoch)
        let last_ordered_block = ordered_blocks_next_epoch.last().unwrap();
        let (_, commit_decision) = create_commit_decision_for_block(last_ordered_block);

        // Remove the ordered blocks for the commit decision (without an execution pool window)
        ordered_block_store.remove_blocks_for_commit(commit_decision.commit_proof(), None);

        // Verify the ordered blocks for the next epoch were removed
        for ordered_block in ordered_blocks_next_epoch {
            verify_contains_block(&ordered_block_store, &ordered_block, false);
        }
        verify_num_ordered_blocks(&ordered_block_store, num_ordered_blocks_future_epoch);
    }

    #[test]
    fn test_remove_blocks_for_commit_execution_pool() {
        // Create a new consensus observer config
        let max_num_pending_blocks = 100;
        let observer_block_window_buffer_multiplier = 2; // Buffer twice the window
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks,
            observer_block_window_buffer_multiplier,
            ..ConsensusObserverConfig::default()
        };

        // Create a new ordered block store
        let mut ordered_block_store = OrderedBlockStore::new(consensus_observer_config);

        // Insert several ordered blocks for the current epoch
        let current_epoch = 10;
        let num_ordered_blocks = 50;
        let ordered_blocks = create_and_add_ordered_blocks(
            &mut ordered_block_store,
            num_ordered_blocks,
            current_epoch,
        );

        // Process commits for rounds less than the buffer (i.e., < window * 2)
        let window_size = 7;
        let buffer_size = window_size * (observer_block_window_buffer_multiplier as usize);
        for commit_round in 0..buffer_size {
            // Process a commit for the ordered block
            let ordered_block = ordered_blocks.get(commit_round).unwrap();
            let (_, commit_decision) = create_commit_decision_for_block(ordered_block);
            ordered_block_store
                .remove_blocks_for_commit(commit_decision.commit_proof(), Some(window_size as u64));

            // Verify the ordered block was not removed (it's within the window)
            verify_contains_block(&ordered_block_store, ordered_block, true);
        }

        // Verify that no ordered blocks were removed
        verify_num_ordered_blocks(&ordered_block_store, num_ordered_blocks);

        // Process a commit for a round one greater than the buffer
        let commit_round = buffer_size;
        let ordered_block = ordered_blocks.get(commit_round).unwrap();
        let (_, commit_decision) = create_commit_decision_for_block(ordered_block);
        ordered_block_store
            .remove_blocks_for_commit(commit_decision.commit_proof(), Some(window_size as u64));

        // Verify that the first ordered block was removed
        verify_contains_block(&ordered_block_store, ordered_blocks.first().unwrap(), false);
        verify_num_ordered_blocks(&ordered_block_store, num_ordered_blocks - 1);

        // Process a commit for the last round
        let commit_round = num_ordered_blocks - 1;
        let ordered_block = ordered_blocks.get(commit_round).unwrap();
        let (_, commit_decision) = create_commit_decision_for_block(ordered_block);
        ordered_block_store
            .remove_blocks_for_commit(commit_decision.commit_proof(), Some(window_size as u64));

        // Verify that all blocks before the buffer were removed
        let buffer_start_round = commit_round - buffer_size + 1;
        for removed_round in 0..buffer_start_round {
            let ordered_block = ordered_blocks.get(removed_round).unwrap();
            verify_contains_block(&ordered_block_store, ordered_block, false);
        }

        // Verify that all blocks after the buffer start were retained
        for retained_round in buffer_start_round..num_ordered_blocks {
            let ordered_block = ordered_blocks.get(retained_round).unwrap();
            verify_contains_block(&ordered_block_store, ordered_block, true);
        }

        // Verify that only the blocks in the buffer were retained
        verify_num_ordered_blocks(&ordered_block_store, buffer_size);
    }

    #[test]
    fn test_remove_blocks_for_commit_execution_pool_epoch() {
        // Create a new consensus observer config
        let max_num_pending_blocks = 300;
        let observer_block_window_buffer_multiplier = 3; // Buffer three times the window
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks,
            observer_block_window_buffer_multiplier,
            ..ConsensusObserverConfig::default()
        };

        // Create a new ordered block store
        let mut ordered_block_store = OrderedBlockStore::new(consensus_observer_config);

        // Insert several ordered blocks for the current epoch
        let current_epoch = 1;
        let num_ordered_blocks = 50;
        let ordered_blocks = create_and_add_ordered_blocks(
            &mut ordered_block_store,
            num_ordered_blocks,
            current_epoch,
        );

        // Insert several ordered blocks for the next epoch
        let next_epoch = current_epoch + 1;
        let num_ordered_blocks_next_epoch = 60;
        let ordered_blocks_next_epoch = create_and_add_ordered_blocks(
            &mut ordered_block_store,
            num_ordered_blocks_next_epoch,
            next_epoch,
        );

        // Insert several ordered blocks for a future epoch
        let future_epoch = next_epoch + 1;
        let num_ordered_blocks_future_epoch = 70;
        let ordered_blocks_future_epoch = create_and_add_ordered_blocks(
            &mut ordered_block_store,
            num_ordered_blocks_future_epoch,
            future_epoch,
        );

        // Process commits for rounds less than the buffer in the next epoch (i.e., < window * 3)
        let window_size = 8;
        let buffer_size = window_size * (observer_block_window_buffer_multiplier as usize);
        for commit_round in 0..buffer_size {
            // Process a commit for the ordered block
            let ordered_block = ordered_blocks_next_epoch.get(commit_round).unwrap();
            let (_, commit_decision) = create_commit_decision_for_block(ordered_block);
            ordered_block_store
                .remove_blocks_for_commit(commit_decision.commit_proof(), Some(window_size as u64));

            // Verify the ordered block was not removed (it's within the window)
            verify_contains_block(&ordered_block_store, ordered_block, true);

            // Verify all the ordered blocks for the current epoch were removed
            for ordered_block in &ordered_blocks {
                verify_contains_block(&ordered_block_store, ordered_block, false);
            }
        }

        // Process a commit for the last round in the next epoch
        let commit_round = num_ordered_blocks_next_epoch - 1;
        let ordered_block = ordered_blocks_next_epoch.get(commit_round).unwrap();
        let (_, commit_decision) = create_commit_decision_for_block(ordered_block);
        ordered_block_store
            .remove_blocks_for_commit(commit_decision.commit_proof(), Some(window_size as u64));

        // Verify that all blocks before the buffer were removed
        let buffer_start_round = commit_round - buffer_size + 1;
        for removed_round in 0..buffer_start_round {
            let ordered_block = ordered_blocks_next_epoch.get(removed_round).unwrap();
            verify_contains_block(&ordered_block_store, ordered_block, false);
        }

        // Verify that all blocks after the buffer start were retained
        for retained_round in buffer_start_round..num_ordered_blocks_next_epoch {
            let ordered_block = ordered_blocks_next_epoch.get(retained_round).unwrap();
            verify_contains_block(&ordered_block_store, ordered_block, true);
        }

        // Process a commit for the first round in the future epoch
        let ordered_block = ordered_blocks_future_epoch.first().unwrap();
        let (_, commit_decision) = create_commit_decision_for_block(ordered_block);
        ordered_block_store
            .remove_blocks_for_commit(commit_decision.commit_proof(), Some(window_size as u64));

        // Verify that all blocks in the next epoch were removed
        for ordered_block in &ordered_blocks_next_epoch {
            verify_contains_block(&ordered_block_store, ordered_block, false);
        }

        // Verify that all blocks in the future epoch were retained
        for ordered_block in &ordered_blocks_future_epoch {
            verify_contains_block(&ordered_block_store, ordered_block, true);
        }
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
            // Create a new ordered block store
            let mut ordered_block_store = OrderedBlockStore::new(consensus_observer_config);

            // Insert ordered blocks for the current epoch
            let current_epoch = 10;
            let num_ordered_blocks = 50;
            let ordered_blocks = create_and_add_ordered_blocks(
                &mut ordered_block_store,
                num_ordered_blocks,
                current_epoch,
            );

            // Process commits for rounds less than the execution pool window size
            for commit_round in 0..window_size {
                // Process a commit for the ordered block
                let ordered_block = ordered_blocks.get(commit_round).unwrap();
                let (_, commit_decision) = create_commit_decision_for_block(ordered_block);
                ordered_block_store.remove_blocks_for_commit(
                    commit_decision.commit_proof(),
                    Some(window_size as u64),
                );

                // Verify the ordered block was not removed (it's within the window)
                verify_contains_block(&ordered_block_store, ordered_block, true);
            }

            // Verify that no ordered blocks were removed
            verify_num_ordered_blocks(&ordered_block_store, num_ordered_blocks);

            // Process commits for rounds greater than the execution pool window size
            for commit_round in window_size..num_ordered_blocks {
                // Process a commit for the ordered block
                let ordered_block = ordered_blocks.get(commit_round).unwrap();
                let (_, commit_decision) = create_commit_decision_for_block(ordered_block);
                ordered_block_store.remove_blocks_for_commit(
                    commit_decision.commit_proof(),
                    Some(window_size as u64),
                );

                // Verify that all blocks before the window were removed
                let window_start_round = commit_round - window_size + 1;
                for removed_round in 0..window_start_round {
                    let ordered_block = ordered_blocks.get(removed_round).unwrap();
                    verify_contains_block(&ordered_block_store, ordered_block, false);
                }

                // Verify that all blocks after the window start were retained
                for retained_round in window_start_round..num_ordered_blocks {
                    let ordered_block = ordered_blocks.get(retained_round).unwrap();
                    verify_contains_block(&ordered_block_store, ordered_block, true);
                }
            }
        }
    }

    #[test]
    fn test_update_commit_decision() {
        // Create a new consensus observer config
        let max_num_pending_blocks = 100;
        let consensus_observer_config = ConsensusObserverConfig {
            max_num_pending_blocks,
            ..ConsensusObserverConfig::default()
        };

        // Create a new ordered block store
        let mut ordered_block_store = OrderedBlockStore::new(consensus_observer_config);

        // Insert several ordered blocks for the current epoch
        let current_epoch = 0;
        let num_ordered_blocks = 10;
        let ordered_blocks = create_and_add_ordered_blocks(
            &mut ordered_block_store,
            num_ordered_blocks,
            current_epoch,
        );

        // Insert several ordered blocks for the next epoch
        let next_epoch = current_epoch + 1;
        let num_ordered_blocks_next_epoch = 20;
        let ordered_blocks_next_epoch = create_and_add_ordered_blocks(
            &mut ordered_block_store,
            num_ordered_blocks_next_epoch,
            next_epoch,
        );

        // Ensure the ordered blocks were all inserted
        verify_num_ordered_blocks(
            &ordered_block_store,
            num_ordered_blocks + num_ordered_blocks_next_epoch,
        );

        // Verify the ordered blocks don't have any commit decisions
        let all_ordered_blocks = ordered_block_store.get_all_ordered_blocks();
        for (_, (_, commit_decision)) in all_ordered_blocks.iter() {
            assert!(commit_decision.is_none());
        }

        // Create a commit decision for the first ordered block
        let first_ordered_block = ordered_blocks.first().unwrap();
        let (first_ordered_block_info, commit_decision) =
            create_commit_decision_for_block(first_ordered_block);

        // Update the commit decision for the first ordered block
        ordered_block_store.update_commit_decision(&commit_decision);

        // Verify the commit decision was updated
        verify_commit_decision(
            &ordered_block_store,
            &first_ordered_block_info,
            commit_decision,
        );

        // Create a commit decision for the last ordered block (in the current epoch)
        let last_ordered_block = ordered_blocks.last().unwrap();
        let (last_ordered_block_info, commit_decision) =
            create_commit_decision_for_block(last_ordered_block);

        // Update the commit decision for the last ordered block
        ordered_block_store.update_commit_decision(&commit_decision);

        // Verify the commit decision was updated
        verify_commit_decision(
            &ordered_block_store,
            &last_ordered_block_info,
            commit_decision,
        );

        // Verify the commit decisions for the remaining blocks are still missing
        let all_ordered_blocks = ordered_block_store.get_all_ordered_blocks();
        for i in 1..9 {
            let (_, commit_decision) = all_ordered_blocks.get(&(current_epoch, i as u64)).unwrap();
            assert!(commit_decision.is_none());
        }

        // Create a commit decision for the last ordered block (in the next epoch)
        let last_ordered_block = ordered_blocks_next_epoch.last().unwrap();
        let (last_ordered_block_info, commit_decision) =
            create_commit_decision_for_block(last_ordered_block);

        // Update the commit decision for the last ordered block
        ordered_block_store.update_commit_decision(&commit_decision);

        // Verify the commit decision was updated
        verify_commit_decision(
            &ordered_block_store,
            &last_ordered_block_info,
            commit_decision,
        );

        // Verify the commit decisions for the remaining blocks are still missing
        let all_ordered_blocks = ordered_block_store.get_all_ordered_blocks();
        for i in 1..19 {
            let (_, commit_decision) = all_ordered_blocks.get(&(next_epoch, i as u64)).unwrap();
            assert!(commit_decision.is_none());
        }
    }

    /// Creates and adds the specified number of ordered blocks to the ordered blocks
    fn create_and_add_ordered_blocks(
        ordered_block_store: &mut OrderedBlockStore,
        num_ordered_blocks: usize,
        epoch: u64,
    ) -> Vec<OrderedBlock> {
        let mut ordered_blocks = vec![];
        for i in 0..num_ordered_blocks {
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

            // Create an observed ordered block
            let observed_ordered_block =
                ObservedOrderedBlock::new_for_testing(ordered_block.clone());

            // Insert the block into the ordered block store
            ordered_block_store.insert_ordered_block(observed_ordered_block.clone());

            // Add the block to the ordered blocks
            ordered_blocks.push(ordered_block);
        }

        ordered_blocks
    }

    /// Creates a commit decision for the given ordered block. Returns the
    /// block info of the last inner block and the corresponding commit decision.
    fn create_commit_decision_for_block(
        ordered_block: &OrderedBlock,
    ) -> (BlockInfo, CommitDecision) {
        let ordered_block_info = ordered_block.last_block().block_info();
        let commit_decision = CommitDecision::new(LedgerInfoWithSignatures::new(
            LedgerInfo::new(ordered_block_info.clone(), HashValue::random()),
            AggregateSignature::empty(),
        ));
        (ordered_block_info, commit_decision)
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

    /// Verifies the commit decision for the specified block info
    fn verify_commit_decision(
        ordered_block_store: &OrderedBlockStore,
        block_info: &BlockInfo,
        commit_decision: CommitDecision,
    ) {
        // Get the commit decision for the block
        let all_ordered_blocks = ordered_block_store.get_all_ordered_blocks();
        let (_, updated_commit_decision) = all_ordered_blocks
            .get(&(block_info.epoch(), block_info.round()))
            .unwrap();

        // Verify the commit decision is expected
        assert_eq!(
            commit_decision,
            updated_commit_decision.as_ref().unwrap().clone()
        );
    }

    /// Fetches the given block from the block store and
    /// verifies its presence based on the expected result.
    fn verify_contains_block(
        ordered_block_store: &OrderedBlockStore,
        ordered_block: &OrderedBlock,
        expect_contains: bool,
    ) {
        // Get the block epoch and round
        let ordered_block_info = ordered_block.last_block().block_info();
        let ordered_block_epoch = ordered_block_info.epoch();
        let ordered_block_round = ordered_block_info.round();

        // Verify the presence of the block in the ordered block store
        let found_block = ordered_block_store
            .get_observed_ordered_block(ordered_block_epoch, ordered_block_round);
        if expect_contains {
            assert_eq!(found_block.unwrap().ordered_block(), ordered_block);
        } else {
            assert!(found_block.is_none());
        }
    }

    /// Verifies the highest committed epoch and round matches the given block info
    fn verify_highest_committed_epoch_round(
        ordered_block_store: &OrderedBlockStore,
        block_info: &BlockInfo,
    ) {
        // Verify the highest committed epoch and round is the block info
        let highest_committed_epoch_round = ordered_block_store
            .get_highest_committed_epoch_round()
            .unwrap();
        assert_eq!(
            highest_committed_epoch_round,
            (block_info.epoch(), block_info.round())
        );
    }

    /// Verifies the number of ordered blocks in the ordered block store
    fn verify_num_ordered_blocks(
        ordered_block_store: &OrderedBlockStore,
        num_ordered_blocks: usize,
    ) {
        assert_eq!(
            ordered_block_store.get_all_ordered_blocks().len(),
            num_ordered_blocks
        );
    }
}
