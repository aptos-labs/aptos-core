// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensus_observer::{
        common::{
            error::Error,
            logging::{LogEntry, LogSchema},
        },
        network::observer_message::{BlockPayload, CommitDecision, OrderedBlock},
        observer::{
            execution_pool::ObservedOrderedBlock,
            ordered_blocks::OrderedBlockStore,
            payload_store::{BlockPayloadStatus, BlockPayloadStore},
            pending_blocks::{PendingBlockStore, PendingBlockWithMetadata},
        },
    },
    pipeline::pipeline_builder::PipelineBuilder,
    state_replication::StateComputerCommitCallBackType,
};
use velor_config::config::ConsensusObserverConfig;
use velor_consensus_types::{
    pipelined_block::{PipelineFutures, PipelinedBlock},
    wrapped_ledger_info::WrappedLedgerInfo,
};
use velor_executor_types::state_compute_result::StateComputeResult;
use velor_infallible::Mutex;
use velor_logger::{info, warn};
use velor_storage_interface::DbReader;
use velor_types::{
    block_info::{BlockInfo, Round},
    epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
};
use std::{collections::BTreeMap, sync::Arc};

/// All block data managed and used by the consensus observer.
/// This is grouped into a single struct to avoid inconsistencies
/// and race conditions when managing and updating the data.
pub struct ObserverBlockData {
    // The block payload store (containing the block transaction payloads)
    block_payload_store: BlockPayloadStore,

    // The ordered block store (containing ordered blocks that are ready for execution)
    ordered_block_store: OrderedBlockStore,

    // The pending block store (containing pending blocks that are without payloads)
    pending_block_store: PendingBlockStore,

    // The latest ledger info
    root: LedgerInfoWithSignatures,
}

impl ObserverBlockData {
    pub fn new(
        consensus_observer_config: ConsensusObserverConfig,
        db_reader: Arc<dyn DbReader>,
    ) -> Self {
        // Get the latest ledger info from storage
        let root = db_reader
            .get_latest_ledger_info()
            .expect("Failed to read latest ledger info from storage!");

        // Create the observer block data
        Self::new_with_root(consensus_observer_config, root)
    }

    /// Creates and returns a new observer block data with the given root ledger info
    fn new_with_root(
        consensus_observer_config: ConsensusObserverConfig,
        root: LedgerInfoWithSignatures,
    ) -> Self {
        // Create the various block stores
        let block_payload_store = BlockPayloadStore::new(consensus_observer_config);
        let ordered_block_store = OrderedBlockStore::new(consensus_observer_config);
        let pending_block_store = PendingBlockStore::new(consensus_observer_config);

        // Create the observer block data
        Self {
            block_payload_store,
            ordered_block_store,
            pending_block_store,
            root,
        }
    }

    /// Returns true iff all the payloads for the given blocks exist
    pub fn all_payloads_exist(&self, blocks: &[Arc<PipelinedBlock>]) -> bool {
        self.block_payload_store.all_payloads_exist(blocks)
    }

    /// Returns true iff the root epoch and round match the given values
    pub fn check_root_epoch_and_round(&self, epoch: u64, round: Round) -> bool {
        // Get the expected epoch and round
        let root = self.root();
        let expected_epoch = root.commit_info().epoch();
        let expected_round = root.commit_info().round();

        // Check if the expected epoch and round match the given values
        expected_epoch == epoch && expected_round == round
    }

    /// Clears all block data and returns the root ledger info
    pub fn clear_block_data(&mut self) -> LedgerInfoWithSignatures {
        // Clear the payload store
        self.block_payload_store.clear_all_payloads();

        // Clear the ordered blocks
        self.ordered_block_store.clear_all_ordered_blocks();

        // Clear the pending blocks
        self.pending_block_store.clear_missing_blocks();

        // Return the root ledger info
        self.root()
    }

    /// Returns true iff we already have a payload entry for the given block
    pub fn existing_payload_entry(&self, block_payload: &BlockPayload) -> bool {
        self.block_payload_store
            .existing_payload_entry(block_payload)
    }

    /// Returns true iff the pending block store contains an entry for the given block
    pub fn existing_pending_block(&self, ordered_block: &OrderedBlock) -> bool {
        self.pending_block_store
            .existing_pending_block(ordered_block)
    }

    /// Returns a copy of the ordered blocks
    pub fn get_all_ordered_blocks(
        &self,
    ) -> BTreeMap<(u64, Round), (ObservedOrderedBlock, Option<CommitDecision>)> {
        self.ordered_block_store.get_all_ordered_blocks()
    }

    /// Returns a reference to the block payloads
    pub fn get_block_payloads(&self) -> Arc<Mutex<BTreeMap<(u64, Round), BlockPayloadStatus>>> {
        self.block_payload_store.get_block_payloads()
    }

    /// Returns the highest committed block epoch and round
    pub fn get_highest_committed_epoch_round(&self) -> (u64, Round) {
        if let Some(epoch_round) = self.ordered_block_store.get_highest_committed_epoch_round() {
            // Return the highest committed epoch and round
            epoch_round
        } else {
            // Return the root epoch and round
            let root_block_info = self.root.commit_info().clone();
            (root_block_info.epoch(), root_block_info.round())
        }
    }

    /// Returns the last ordered block
    pub fn get_last_ordered_block(&self) -> BlockInfo {
        if let Some(last_ordered_block) = self.ordered_block_store.get_last_ordered_block() {
            // Return the last ordered block
            last_ordered_block.block_info()
        } else {
            // Return the root block
            self.root.commit_info().clone()
        }
    }

    /// Returns the ordered block for the given epoch and round (if any)
    pub fn get_ordered_block(
        &self,
        epoch: u64,
        round: velor_consensus_types::common::Round,
    ) -> Option<OrderedBlock> {
        self.ordered_block_store.get_ordered_block(epoch, round)
    }

    /// Returns the parent block's pipeline futures
    pub fn get_parent_pipeline_futs(
        &self,
        block: &PipelinedBlock,
        pipeline_builder: &PipelineBuilder,
    ) -> Option<PipelineFutures> {
        if let Some(last_ordered_block) = self
            .ordered_block_store
            .get_ordered_block(block.epoch(), block.quorum_cert().certified_block().round())
        {
            // Return the parent block's pipeline futures
            last_ordered_block.last_block().pipeline_futs()
        } else {
            // Return the root block's pipeline futures
            Some(pipeline_builder.build_root(StateComputeResult::new_dummy(), self.root.clone()))
        }
    }

    /// Handles commited blocks up to the given ledger info
    fn handle_committed_blocks(&mut self, ledger_info: LedgerInfoWithSignatures) {
        // Remove the committed blocks from the payload and ordered block stores
        self.block_payload_store.remove_blocks_for_epoch_round(
            ledger_info.commit_info().epoch(),
            ledger_info.commit_info().round(),
        );
        self.ordered_block_store
            .remove_blocks_for_commit(&ledger_info);

        // Verify the ledger info is for the same epoch
        let root_commit_info = self.root.commit_info();
        if ledger_info.commit_info().epoch() != root_commit_info.epoch() {
            warn!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Received commit callback for a different epoch! Ledger info: {:?}, Root: {:?}",
                    ledger_info.commit_info(),
                    root_commit_info
                ))
            );
            return;
        }

        // Update the root ledger info. Note: we only want to do this if
        // the new ledger info round is greater than the current root
        // round. Otherwise, this can race with the state sync process.
        if ledger_info.commit_info().round() > root_commit_info.round() {
            info!(
            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                "Updating the root ledger info! Old root: (epoch: {:?}, round: {:?}). New root: (epoch: {:?}, round: {:?})",
                root_commit_info.epoch(),
                root_commit_info.round(),
                ledger_info.commit_info().epoch(),
                ledger_info.commit_info().round(),
            ))
        );
            self.root = ledger_info;
        }
    }

    /// Inserts the given block payload data into the payload store
    pub fn insert_block_payload(
        &mut self,
        block_payload: BlockPayload,
        verified_payload_signatures: bool,
    ) {
        self.block_payload_store
            .insert_block_payload(block_payload, verified_payload_signatures);
    }

    /// Inserts the observed ordered block into the ordered blocks
    pub fn insert_ordered_block(&mut self, observed_ordered_block: ObservedOrderedBlock) {
        self.ordered_block_store
            .insert_ordered_block(observed_ordered_block);
    }

    /// Inserts a pending block (without payloads) into the store
    pub fn insert_pending_block(&mut self, pending_block: Arc<PendingBlockWithMetadata>) {
        self.pending_block_store.insert_pending_block(pending_block);
    }

    /// Removes and returns the pending block from the store that is now
    /// ready to be processed (after the new payload has been received).
    pub fn remove_ready_pending_block(
        &mut self,
        received_payload_epoch: u64,
        received_payload_round: Round,
    ) -> Option<Arc<PendingBlockWithMetadata>> {
        self.pending_block_store.remove_ready_block(
            received_payload_epoch,
            received_payload_round,
            &mut self.block_payload_store,
        )
    }

    /// Returns a clone of the current root ledger info
    pub fn root(&self) -> LedgerInfoWithSignatures {
        self.root.clone()
    }

    /// Updates the metrics for the processed blocks
    pub fn update_block_metrics(&self) {
        // Update the payload store metrics
        self.block_payload_store.update_payload_store_metrics();

        // Update the ordered block metrics
        self.ordered_block_store.update_ordered_blocks_metrics();

        // Update the pending block metrics
        self.pending_block_store.update_pending_blocks_metrics();
    }

    /// Updates the block data for the given commit decision
    /// that will be used by state sync to catch us up.
    pub fn update_blocks_for_state_sync_commit(&mut self, commit_decision: &CommitDecision) {
        // Get the commit proof, epoch and round
        let commit_proof = commit_decision.commit_proof();
        let commit_epoch = commit_decision.epoch();
        let commit_round = commit_decision.round();

        // Update the root
        self.update_root(commit_proof.clone());

        // Update the block payload store
        self.block_payload_store
            .remove_blocks_for_epoch_round(commit_epoch, commit_round);

        // Update the ordered block store
        self.ordered_block_store
            .remove_blocks_for_commit(commit_proof);
    }

    /// Updates the commit decision of the ordered block
    pub fn update_ordered_block_commit_decision(&mut self, commit_decision: &CommitDecision) {
        self.ordered_block_store
            .update_commit_decision(commit_decision);
    }

    /// Updates the root ledger info
    pub fn update_root(&mut self, new_root: LedgerInfoWithSignatures) {
        self.root = new_root;
    }

    /// Verifies all block payloads against the given ordered block
    pub fn verify_payloads_against_ordered_block(
        &mut self,
        ordered_block: &OrderedBlock,
    ) -> Result<(), Error> {
        self.block_payload_store
            .verify_payloads_against_ordered_block(ordered_block)
    }

    /// Verifies the block payload signatures against the given epoch state
    pub fn verify_payload_signatures(
        &mut self,
        epoch_state: &EpochState,
    ) -> Vec<velor_consensus_types::common::Round> {
        self.block_payload_store
            .verify_payload_signatures(epoch_state)
    }
}

/// Creates and returns a commit callback. This will update the
/// root ledger info and remove the blocks from the given stores.
pub fn create_commit_callback(
    observer_block_data: Arc<Mutex<ObserverBlockData>>,
) -> Box<dyn FnOnce(WrappedLedgerInfo, LedgerInfoWithSignatures) + Send + Sync> {
    Box::new(move |_, ledger_info: LedgerInfoWithSignatures| {
        observer_block_data
            .lock()
            .handle_committed_blocks(ledger_info);
    })
}

/// Creates and returns the commit callback used by the old pipeline
pub fn create_commit_callback_deprecated(
    observer_block_data: Arc<Mutex<ObserverBlockData>>,
) -> StateComputerCommitCallBackType {
    Box::new(move |_, ledger_info| {
        observer_block_data
            .lock()
            .handle_committed_blocks(ledger_info);
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::consensus_observer::{
        network::observer_message::{BlockPayload, BlockTransactionPayload, OrderedBlock},
        observer::execution_pool::ObservedOrderedBlock,
    };
    use velor_config::network_id::PeerNetworkId;
    use velor_consensus_types::{
        block::Block,
        block_data::{BlockData, BlockType},
        pipelined_block::{OrderedBlockWindow, PipelinedBlock},
        quorum_cert::QuorumCert,
    };
    use velor_crypto::HashValue;
    use velor_types::{
        aggregate_signature::AggregateSignature, block_info::BlockInfo, ledger_info::LedgerInfo,
        transaction::Version, validator_verifier::ValidatorVerifier,
    };
    use std::time::Instant;

    #[test]
    fn test_all_payloads_exist() {
        // Create a root ledger info
        let epoch = 10;
        let round = 5;
        let root = create_ledger_info(epoch, round);

        // Create the observer block data
        let mut observer_block_data =
            ObserverBlockData::new_with_root(ConsensusObserverConfig::default(), root.clone());

        // Create a pipelined block and payload
        let pipelined_block = create_pipelined_block(root.commit_info().clone());
        let block_payload = BlockPayload::new(
            pipelined_block.block_info(),
            BlockTransactionPayload::empty(),
        );

        // Verify that the payload does not exist
        assert!(!observer_block_data.all_payloads_exist(&[pipelined_block.clone()]));

        // Insert the block payload into the store
        observer_block_data.insert_block_payload(block_payload.clone(), true);

        // Verify that the inserted payload exists
        assert!(observer_block_data.all_payloads_exist(&[pipelined_block]));
    }

    #[test]
    fn test_check_root_epoch_and_round() {
        // Create a root ledger info
        let epoch = 10;
        let round = 5;
        let root = create_ledger_info(epoch, round);

        // Create the observer block data
        let mut observer_block_data =
            ObserverBlockData::new_with_root(ConsensusObserverConfig::default(), root);

        // Check the root epoch and round
        assert!(observer_block_data.check_root_epoch_and_round(epoch, round));
        assert!(!observer_block_data.check_root_epoch_and_round(epoch, round + 1));
        assert!(!observer_block_data.check_root_epoch_and_round(epoch + 1, round));

        // Update the root ledger info
        let new_epoch = epoch + 10;
        let new_round = round + 100;
        let new_root = create_ledger_info(new_epoch, new_round);
        observer_block_data.update_root(new_root.clone());

        // Check the updated root epoch and round
        assert!(!observer_block_data.check_root_epoch_and_round(epoch, round));
        assert!(observer_block_data.check_root_epoch_and_round(new_epoch, new_round));
    }

    #[test]
    fn test_clear_block_data() {
        // Create a root ledger info
        let epoch = 10;
        let round = 5;
        let root = create_ledger_info(epoch, round);

        // Create the observer block data
        let mut observer_block_data =
            ObserverBlockData::new_with_root(ConsensusObserverConfig::default(), root.clone());

        // Create a pipelined block
        let pipelined_block = create_pipelined_block(root.commit_info().clone());

        // Create an observed ordered block
        let ordered_block = OrderedBlock::new(
            vec![pipelined_block.clone()],
            create_ledger_info(epoch, round),
        );
        let observed_ordered_block = ObservedOrderedBlock::new_for_testing(ordered_block.clone());

        // Creating a pending block
        let pending_block_with_metadata = PendingBlockWithMetadata::new_with_arc(
            PeerNetworkId::random(),
            Instant::now(),
            observed_ordered_block.clone(),
        );

        // Create a block payload
        let block_payload = BlockPayload::new(
            pipelined_block.block_info(),
            BlockTransactionPayload::empty(),
        );

        // Insert the block into all stores
        observer_block_data.insert_block_payload(block_payload.clone(), true);
        observer_block_data.insert_ordered_block(observed_ordered_block);
        observer_block_data.insert_pending_block(pending_block_with_metadata);

        // Verify the block data is inserted
        assert!(observer_block_data.existing_payload_entry(&block_payload));
        assert!(observer_block_data
            .get_ordered_block(epoch, round)
            .is_some());
        assert!(observer_block_data.existing_pending_block(&ordered_block));

        // Clear the block data
        let cleared_root = observer_block_data.clear_block_data();

        // Verify that the block data is cleared
        assert!(!observer_block_data.existing_payload_entry(&block_payload));
        assert!(observer_block_data
            .get_ordered_block(epoch, round)
            .is_none());
        assert!(!observer_block_data.existing_pending_block(&ordered_block));

        // Verify the root ledger info and that all stores are empty
        assert_eq!(cleared_root, root);
        assert_eq!(observer_block_data.get_block_payloads().lock().len(), 0);
        assert_eq!(observer_block_data.get_all_ordered_blocks().len(), 0);
    }

    #[test]
    fn test_get_and_update_root() {
        // Create a root ledger info
        let epoch = 100;
        let round = 50;
        let root = create_ledger_info(epoch, round);

        // Create the observer block data
        let mut observer_block_data =
            ObserverBlockData::new_with_root(ConsensusObserverConfig::default(), root.clone());

        // Check the root ledger info
        assert_eq!(observer_block_data.root(), root);

        // Update the root ledger info
        let new_root = create_ledger_info(epoch, round + 1000);
        observer_block_data.update_root(new_root.clone());

        // Check the updated root ledger info
        assert_eq!(observer_block_data.root(), new_root);
    }

    #[test]
    fn test_get_highest_committed_epoch_round() {
        // Create a root ledger info
        let epoch = 10;
        let round = 5;
        let root = create_ledger_info(epoch, round);

        // Create the observer block data
        let mut observer_block_data =
            ObserverBlockData::new_with_root(ConsensusObserverConfig::default(), root.clone());

        // Verify the highest committed epoch and round (it should be the root)
        let (highest_epoch, highest_round) =
            observer_block_data.get_highest_committed_epoch_round();
        assert_eq!(highest_epoch, epoch);
        assert_eq!(highest_round, round);

        // Add an ordered block (with a higher round)
        let new_round = round + 1;
        let _ = create_and_add_ordered_blocks(&mut observer_block_data, 1, epoch, new_round);

        // Verify the highest committed epoch and round (it should still be the root)
        let (highest_epoch, highest_round) =
            observer_block_data.get_highest_committed_epoch_round();
        assert_eq!(highest_epoch, epoch);
        assert_eq!(highest_round, round);

        // Update the commit decision for the ordered block
        let commit_decision = CommitDecision::new(create_ledger_info(epoch, new_round));
        observer_block_data.update_ordered_block_commit_decision(&commit_decision);

        // Verify the highest committed epoch and round (it should be the new block)
        let (highest_epoch, highest_round) =
            observer_block_data.get_highest_committed_epoch_round();
        assert_eq!(highest_epoch, epoch);
        assert_eq!(highest_round, new_round);

        // Add an ordered block (with a higher epoch)
        let new_epoch = epoch + 1;
        let new_round = 0;
        let _ = create_and_add_ordered_blocks(&mut observer_block_data, 1, new_epoch, 0);

        // Update the commit decision for the ordered block
        let commit_decision = CommitDecision::new(create_ledger_info(new_epoch, new_round));
        observer_block_data.update_ordered_block_commit_decision(&commit_decision);

        // Verify the highest committed epoch and round (it should be the new block)
        let (highest_epoch, highest_round) =
            observer_block_data.get_highest_committed_epoch_round();
        assert_eq!(highest_epoch, new_epoch);
        assert_eq!(highest_round, new_round);
    }

    #[test]
    fn test_get_last_ordered_block() {
        // Create a root ledger info
        let epoch = 10;
        let round = 5;
        let root = create_ledger_info(epoch, round);

        // Create the observer block data
        let mut observer_block_data =
            ObserverBlockData::new_with_root(ConsensusObserverConfig::default(), root.clone());

        // Verify the last ordered block (it should be the root)
        let last_ordered_block = observer_block_data.get_last_ordered_block();
        assert_eq!(last_ordered_block.epoch(), epoch);
        assert_eq!(last_ordered_block.round(), round);

        // Add an ordered block (with a higher round)
        let new_round = round + 1;
        let ordered_block =
            create_and_add_ordered_blocks(&mut observer_block_data, 1, epoch, new_round);

        // Verify the last ordered block (it should be the new block)
        let last_ordered_block = observer_block_data.get_last_ordered_block();
        assert_eq!(
            last_ordered_block,
            ordered_block[0].last_block().block_info()
        );

        // Add an ordered block (with a higher epoch)
        let new_epoch = epoch + 1;
        let new_round = 0;
        let ordered_block =
            create_and_add_ordered_blocks(&mut observer_block_data, 1, new_epoch, new_round);

        // Verify the last ordered block (it should be the new block)
        let last_ordered_block = observer_block_data.get_last_ordered_block();
        assert_eq!(
            last_ordered_block,
            ordered_block[0].last_block().block_info()
        );
    }

    #[test]
    fn test_handle_committed_blocks() {
        // Create a root ledger info
        let epoch = 10;
        let round = 5;
        let root = create_ledger_info(epoch, round);

        // Create the observer block data
        let mut observer_block_data =
            ObserverBlockData::new_with_root(ConsensusObserverConfig::default(), root);

        // Handle the committed blocks at the wrong epoch and verify the root is not updated
        observer_block_data.handle_committed_blocks(create_ledger_info(epoch + 1, round + 1));
        assert_eq!(observer_block_data.root().commit_info().epoch(), epoch);

        // Handle the committed blocks at the wrong round and verify the root is not updated
        observer_block_data.handle_committed_blocks(create_ledger_info(epoch, round - 1));
        assert_eq!(observer_block_data.root().commit_info().round(), round);

        // Add pending ordered blocks
        let num_ordered_blocks = 10;
        let ordered_blocks = create_and_add_ordered_blocks(
            &mut observer_block_data,
            num_ordered_blocks,
            epoch,
            round,
        );

        // Add block payloads for the ordered blocks
        for ordered_block in &ordered_blocks {
            create_and_add_payloads_for_ordered_block(&mut observer_block_data, ordered_block);
        }

        // Create the commit ledger info (for the second to last block)
        let commit_round = round + (num_ordered_blocks as Round) - 2;
        let committed_ledger_info = create_ledger_info(epoch, commit_round);

        // Create the committed blocks and ledger info
        let mut committed_blocks = vec![];
        for ordered_block in ordered_blocks.iter().take(num_ordered_blocks - 1) {
            let pipelined_block = create_pipelined_block(ordered_block.blocks()[0].block_info());
            committed_blocks.push(pipelined_block);
        }

        // Handle the committed blocks
        observer_block_data.handle_committed_blocks(committed_ledger_info.clone());

        // Verify the committed blocks are removed from the stores, and the root is updated
        assert_eq!(observer_block_data.get_all_ordered_blocks().len(), 1);
        assert_eq!(observer_block_data.get_block_payloads().lock().len(), 1);
        assert_eq!(observer_block_data.root(), committed_ledger_info);
    }

    #[test]
    fn test_remove_ready_pending_block() {
        // Create a root ledger info
        let epoch = 100;
        let round = 5;
        let root = create_ledger_info(epoch, round);

        // Create the observer block data
        let mut observer_block_data =
            ObserverBlockData::new_with_root(ConsensusObserverConfig::default(), root.clone());

        // Create an observed ordered block
        let pipelined_block = create_pipelined_block(root.commit_info().clone());
        let ordered_block = OrderedBlock::new(
            vec![pipelined_block.clone()],
            create_ledger_info(epoch, round),
        );
        let observed_ordered_block = ObservedOrderedBlock::new_for_testing(ordered_block.clone());

        // Creating a pending block
        let pending_block_with_metadata = PendingBlockWithMetadata::new_with_arc(
            PeerNetworkId::random(),
            Instant::now(),
            observed_ordered_block.clone(),
        );

        // Insert the pending block into the store
        observer_block_data.insert_pending_block(pending_block_with_metadata);

        // Create a block payload
        let block_payload = BlockPayload::new(
            pipelined_block.block_info(),
            BlockTransactionPayload::empty(),
        );

        // Insert the block payload into the store
        observer_block_data.insert_block_payload(block_payload.clone(), true);

        // Remove the ready pending block
        let removed_pending_block = observer_block_data
            .remove_ready_pending_block(epoch, round)
            .unwrap();

        // Verify the removed pending block is the same as the inserted one
        let last_removed_block = removed_pending_block.ordered_block().last_block();
        assert_eq!(
            last_removed_block.block_info(),
            ordered_block.last_block().block_info()
        );
    }

    #[test]
    fn test_update_blocks_for_state_sync_commit() {
        // Create a root ledger info
        let epoch = 10;
        let round = 5;
        let root = create_ledger_info(epoch, round);

        // Create the observer block data
        let mut observer_block_data =
            ObserverBlockData::new_with_root(ConsensusObserverConfig::default(), root.clone());

        // Create a pipelined block
        let pipelined_block = create_pipelined_block(root.commit_info().clone());

        // Create an observed ordered block
        let ordered_block = OrderedBlock::new(
            vec![pipelined_block.clone()],
            create_ledger_info(epoch, round),
        );
        let observed_ordered_block = ObservedOrderedBlock::new_for_testing(ordered_block.clone());

        // Create a block payload
        let block_payload = BlockPayload::new(
            pipelined_block.block_info(),
            BlockTransactionPayload::empty(),
        );

        // Insert the block and payload into the stores
        observer_block_data.insert_block_payload(block_payload.clone(), true);
        observer_block_data.insert_ordered_block(observed_ordered_block.clone());

        // Verify the block and payload are inserted
        assert!(observer_block_data.existing_payload_entry(&block_payload));
        assert!(observer_block_data
            .get_ordered_block(epoch, round)
            .is_some());

        // Update the blocks for a state sync commit
        let commit_decision = CommitDecision::new(create_ledger_info(epoch, round));
        observer_block_data.update_blocks_for_state_sync_commit(&commit_decision);

        // Verify the root ledger info is updated
        assert_eq!(&observer_block_data.root(), commit_decision.commit_proof());

        // Verify the block and payload are removed
        assert!(!observer_block_data.existing_payload_entry(&block_payload));
        assert!(observer_block_data
            .get_ordered_block(epoch, round)
            .is_none());
    }

    #[test]
    fn test_verify_payloads_against_ordered_block() {
        // Create a root ledger info
        let epoch = 10;
        let round = 5;
        let root = create_ledger_info(epoch, round);

        // Create the observer block data
        let mut observer_block_data =
            ObserverBlockData::new_with_root(ConsensusObserverConfig::default(), root.clone());

        // Create an observed ordered block
        let pipelined_block = create_pipelined_block(root.commit_info().clone());
        let ordered_block = OrderedBlock::new(
            vec![pipelined_block.clone()],
            create_ledger_info(epoch, round),
        );

        // Verify the payloads against the ordered block (payload not inserted yet)
        assert!(observer_block_data
            .verify_payloads_against_ordered_block(&ordered_block)
            .is_err());
    }

    #[test]
    fn test_verify_payload_signatures() {
        // Create a root ledger info
        let epoch = 10;
        let round = 5;
        let root = create_ledger_info(epoch, round);

        // Create the observer block data
        let mut observer_block_data =
            ObserverBlockData::new_with_root(ConsensusObserverConfig::default(), root.clone());

        // Verify the payload signatures (no payloads inserted yet)
        assert!(observer_block_data
            .verify_payload_signatures(&EpochState::new(epoch, ValidatorVerifier::new(vec![])))
            .is_empty());
    }

    /// Creates and adds the specified number of ordered blocks to the ordered blocks
    fn create_and_add_ordered_blocks(
        observer_block_data: &mut ObserverBlockData,
        num_ordered_blocks: usize,
        epoch: u64,
        starting_round: Round,
    ) -> Vec<OrderedBlock> {
        let mut ordered_blocks = vec![];
        for i in 0..num_ordered_blocks {
            // Create a new block info
            let round = starting_round + (i as Round);
            let block_info = BlockInfo::new(
                epoch,
                round,
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
            let pipelined_block = Arc::new(PipelinedBlock::new_ordered(
                block,
                OrderedBlockWindow::empty(),
            ));

            // Create an ordered block
            let blocks = vec![pipelined_block];
            let ordered_proof =
                create_ledger_info(epoch, i as velor_consensus_types::common::Round);
            let ordered_block = OrderedBlock::new(blocks, ordered_proof);

            // Create an observed ordered block
            let observed_ordered_block =
                ObservedOrderedBlock::new_for_testing(ordered_block.clone());

            // Insert the block into the ordered block store
            observer_block_data.insert_ordered_block(observed_ordered_block.clone());

            // Add the block to the ordered blocks
            ordered_blocks.push(ordered_block);
        }

        ordered_blocks
    }

    /// Creates and adds payloads for the ordered block
    fn create_and_add_payloads_for_ordered_block(
        observer_block_data: &mut ObserverBlockData,
        ordered_block: &OrderedBlock,
    ) {
        for block in ordered_block.blocks() {
            let block_payload =
                BlockPayload::new(block.block_info(), BlockTransactionPayload::empty());

            observer_block_data.insert_block_payload(block_payload, true);
        }
    }

    /// Creates and returns a new ledger info with the specified epoch and round
    fn create_ledger_info(
        epoch: u64,
        round: velor_consensus_types::common::Round,
    ) -> LedgerInfoWithSignatures {
        LedgerInfoWithSignatures::new(
            LedgerInfo::new(
                BlockInfo::random_with_epoch(epoch, round),
                HashValue::random(),
            ),
            AggregateSignature::empty(),
        )
    }

    /// Creates and returns a new pipelined block with the given block info
    fn create_pipelined_block(block_info: BlockInfo) -> Arc<PipelinedBlock> {
        let block_data = BlockData::new_for_testing(
            block_info.epoch(),
            block_info.round(),
            block_info.timestamp_usecs(),
            QuorumCert::dummy(),
            BlockType::Genesis,
        );
        let block = Block::new_for_testing(block_info.id(), block_data, None);

        Arc::new(PipelinedBlock::new_ordered(
            block,
            OrderedBlockWindow::empty(),
        ))
    }
}
