// Copyright © Aptos Foundation
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
use aptos_config::config::ConsensusObserverConfig;
use aptos_consensus_types::{
    pipelined_block::{PipelineFutures, PipelinedBlock},
    wrapped_ledger_info::WrappedLedgerInfo,
};
use aptos_executor_types::state_compute_result::StateComputeResult;
use aptos_infallible::Mutex;
use aptos_logger::{info, warn};
use aptos_storage_interface::DbReader;
use aptos_types::{
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
        round: aptos_consensus_types::common::Round,
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
    ) -> Vec<aptos_consensus_types::common::Round> {
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
