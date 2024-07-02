// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::{
    logging::{LogEntry, LogSchema},
    network_message::OrderedBlock,
};
use aptos_consensus_types::{common::Round, pipeline::commit_decision::CommitDecision};
use aptos_infallible::Mutex;
use aptos_logger::debug;
use aptos_types::{block_info::BlockInfo, ledger_info::LedgerInfoWithSignatures};
use std::{collections::BTreeMap, sync::Arc};

/// A simple struct to store the block payloads of ordered and committed blocks
#[derive(Clone)]
pub struct PendingOrderedBlocks {
    // Pending ordered blocks (indexed by consensus round)
    pending_ordered_blocks: Arc<Mutex<BTreeMap<Round, (OrderedBlock, Option<CommitDecision>)>>>,
}

impl PendingOrderedBlocks {
    pub fn new() -> Self {
        Self {
            pending_ordered_blocks: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Clears all pending blocks
    pub fn clear_all_pending_blocks(&self) {
        self.pending_ordered_blocks.lock().clear();
    }

    /// Returns a copy of the pending ordered blocks map
    pub fn get_all_pending_blocks(
        &self,
    ) -> BTreeMap<Round, (OrderedBlock, Option<CommitDecision>)> {
        self.pending_ordered_blocks.lock().clone()
    }

    /// Returns the pending ordered block (if any)
    pub fn get_last_pending_block(&self) -> Option<BlockInfo> {
        let pending_ordered_blocks = self.pending_ordered_blocks.lock();
        if let Some((_, (ordered_block, _))) = pending_ordered_blocks.last_key_value() {
            Some(ordered_block.blocks.last().unwrap().block_info())
        } else {
            None // No pending blocks were found
        }
    }

    /// Returns the pending ordered block (if any)
    pub fn get_pending_block(&self, round: Round) -> Option<OrderedBlock> {
        let pending_ordered_blocks = self.pending_ordered_blocks.lock();
        pending_ordered_blocks
            .get(&round)
            .map(|(ordered_block, _)| ordered_block.clone())
    }

    /// Inserts the given ordered block into the pending blocks
    pub fn insert_ordered_block(&self, ordered_block: OrderedBlock) {
        debug!(
            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                "Adding ordered block to the pending blocks: {}",
                ordered_block.ordered_proof.commit_info()
            ))
        );

        // Insert the ordered block into the pending ordered blocks
        let last_block_round = ordered_block.blocks.last().unwrap().round();
        self.pending_ordered_blocks
            .lock()
            .insert(last_block_round, (ordered_block, None));
    }

    /// Removes the pending blocks for the given commit ledger info.
    /// This will remove all blocks up to (and including) the commit
    /// round of the committed ledger info.
    pub fn remove_blocks_for_commit(&self, commit_ledger_info: &LedgerInfoWithSignatures) {
        // Determine the round to split off
        let split_off_round = commit_ledger_info.commit_info().round() + 1;

        // Remove the pending blocks before the split off round
        let mut pending_ordered_blocks = self.pending_ordered_blocks.lock();
        *pending_ordered_blocks = pending_ordered_blocks.split_off(&split_off_round);
    }

    /// Updates the commit decision of the pending ordered block (if found)
    pub fn update_commit_decision(&self, commit_decision: &CommitDecision) {
        let mut pending_ordered_blocks = self.pending_ordered_blocks.lock();
        if let Some((_, existing_commit_decision)) =
            pending_ordered_blocks.get_mut(&commit_decision.round())
        {
            *existing_commit_decision = Some(commit_decision.clone());
        }
    }
}

impl Default for PendingOrderedBlocks {
    fn default() -> Self {
        Self::new()
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
        aggregate_signature::AggregateSignature, ledger_info::LedgerInfo, transaction::Version,
    };

    #[test]
    pub fn test_clear_pending_blocks() {
        // Create new pending ordered blocks
        let pending_ordered_blocks = PendingOrderedBlocks::new();

        // Insert several pending blocks
        let num_pending_blocks = 10;
        let pending_blocks =
            create_and_add_pending_blocks(&pending_ordered_blocks, num_pending_blocks);

        // Verify the pending blocks were all inserted
        let all_pending_blocks = pending_ordered_blocks.get_all_pending_blocks();
        assert_eq!(all_pending_blocks.len(), num_pending_blocks);

        // Verify the pending blocks were inserted by round
        for pending_block in pending_blocks {
            // Get the round of the last block in the pending block
            let round = pending_block.blocks.last().unwrap().round();

            // Verify the pending block exists for the round
            assert_eq!(
                pending_block,
                pending_ordered_blocks.get_pending_block(round).unwrap()
            );
        }

        // Clear all pending blocks
        pending_ordered_blocks.clear_all_pending_blocks();

        // Verify all blocks were removed
        let all_pending_blocks = pending_ordered_blocks.get_all_pending_blocks();
        assert_eq!(all_pending_blocks.len(), 0);
    }

    #[test]
    pub fn test_get_last_pending_block() {
        // Create new pending ordered blocks
        let pending_ordered_blocks = PendingOrderedBlocks::new();

        // Insert several pending blocks
        let num_pending_blocks = 100;
        let pending_blocks =
            create_and_add_pending_blocks(&pending_ordered_blocks, num_pending_blocks);

        // Verify the last pending block is the one with the highest round
        let last_pending_block = pending_blocks.last().unwrap();
        let last_pending_block_info = last_pending_block.blocks.last().unwrap().block_info();
        assert_eq!(
            last_pending_block_info,
            pending_ordered_blocks.get_last_pending_block().unwrap()
        );
    }

    #[test]
    pub fn test_remove_blocks_for_commit() {
        // Create new pending ordered blocks
        let pending_ordered_blocks = PendingOrderedBlocks::new();

        // Insert several pending blocks
        let num_pending_blocks = 10;
        let pending_blocks =
            create_and_add_pending_blocks(&pending_ordered_blocks, num_pending_blocks);

        // Create a commit decision for the first pending block
        let first_pending_block = pending_blocks.first().unwrap();
        let first_pending_block_info = first_pending_block.blocks.last().unwrap().block_info();
        let commit_decision = CommitDecision::new(LedgerInfoWithSignatures::new(
            LedgerInfo::new(first_pending_block_info.clone(), HashValue::random()),
            AggregateSignature::empty(),
        ));

        // Remove the pending blocks for the commit decision
        pending_ordered_blocks.remove_blocks_for_commit(commit_decision.ledger_info());

        // Verify the first pending block was removed
        let all_pending_blocks = pending_ordered_blocks.get_all_pending_blocks();
        assert_eq!(all_pending_blocks.len(), num_pending_blocks - 1);
        assert!(!all_pending_blocks.contains_key(&first_pending_block_info.round()));

        // Create a commit decision for the last pending block
        let last_pending_block = pending_blocks.last().unwrap();
        let last_pending_block_info = last_pending_block.blocks.last().unwrap().block_info();
        let commit_decision = CommitDecision::new(LedgerInfoWithSignatures::new(
            LedgerInfo::new(last_pending_block_info.clone(), HashValue::random()),
            AggregateSignature::empty(),
        ));

        // Remove the pending blocks for the commit decision
        pending_ordered_blocks.remove_blocks_for_commit(commit_decision.ledger_info());

        // Verify all the pending blocks were removed
        let all_pending_blocks = pending_ordered_blocks.get_all_pending_blocks();
        assert!(all_pending_blocks.is_empty());
    }

    #[test]
    pub fn test_update_commit_decision() {
        // Create new pending ordered blocks
        let pending_ordered_blocks = PendingOrderedBlocks::new();

        // Insert several pending blocks
        let num_pending_blocks = 10;
        let pending_blocks =
            create_and_add_pending_blocks(&pending_ordered_blocks, num_pending_blocks);

        // Verify the pending blocks were all inserted
        let all_pending_blocks = pending_ordered_blocks.get_all_pending_blocks();
        assert_eq!(all_pending_blocks.len(), num_pending_blocks);

        // Verify the pending blocks don't have any commit decisions
        for (_, (_, commit_decision)) in all_pending_blocks.iter() {
            assert!(commit_decision.is_none());
        }

        // Create a commit decision for the first pending block
        let first_pending_block = pending_blocks.first().unwrap();
        let first_pending_block_info = first_pending_block.blocks.last().unwrap().block_info();
        let commit_decision = CommitDecision::new(LedgerInfoWithSignatures::new(
            LedgerInfo::new(first_pending_block_info.clone(), HashValue::random()),
            AggregateSignature::empty(),
        ));

        // Update the commit decision for the first pending block
        pending_ordered_blocks.update_commit_decision(&commit_decision);

        // Verify the commit decision was updated
        verify_commit_decision(
            &pending_ordered_blocks,
            &first_pending_block_info,
            commit_decision,
        );

        // Create a commit decision for the last pending block
        let last_pending_block = pending_blocks.last().unwrap();
        let last_pending_block_info = last_pending_block.blocks.last().unwrap().block_info();
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
        let all_pending_blocks = pending_ordered_blocks.get_all_pending_blocks();
        for i in 1..9 {
            let (_, commit_decision) = all_pending_blocks.get(&(i as u64)).unwrap();
            assert!(commit_decision.is_none());
        }
    }

    /// Creates and adds the specified number of pending blocks to the pending ordered blocks
    fn create_and_add_pending_blocks(
        pending_ordered_blocks: &PendingOrderedBlocks,
        num_pending_blocks: usize,
    ) -> Vec<OrderedBlock> {
        let mut pending_blocks = vec![];
        for i in 0..num_pending_blocks {
            // Create a new block info
            let block_info = BlockInfo::new(
                i as u64,
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
            let ordered_proof = LedgerInfoWithSignatures::new(
                LedgerInfo::new(BlockInfo::empty(), HashValue::random()),
                AggregateSignature::empty(),
            );
            let ordered_block = OrderedBlock {
                blocks,
                ordered_proof,
            };

            // Insert the ordered block into the pending ordered blocks
            pending_ordered_blocks.insert_ordered_block(ordered_block.clone());

            // Add the ordered block to the pending blocks
            pending_blocks.push(ordered_block);
        }

        pending_blocks
    }

    /// Verifies the commit decision for the specified block info
    fn verify_commit_decision(
        pending_ordered_blocks: &PendingOrderedBlocks,
        block_info: &BlockInfo,
        commit_decision: CommitDecision,
    ) {
        // Get the commit decision for the block
        let all_pending_blocks = pending_ordered_blocks.get_all_pending_blocks();
        let (_, updated_commit_decision) = all_pending_blocks.get(&block_info.round()).unwrap();

        // Verify the commit decision is expected
        assert_eq!(
            commit_decision,
            updated_commit_decision.as_ref().unwrap().clone()
        );
    }
}
