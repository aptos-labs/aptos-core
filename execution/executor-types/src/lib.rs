// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0
#![forbid(unsafe_code)]

use crate::state_checkpoint_output::StateCheckpointOutput;
use anyhow::Result;
use aptos_crypto::{
    hash::{TransactionAccumulatorHasher, ACCUMULATOR_PLACEHOLDER_HASH},
    HashValue,
};
use aptos_scratchpad::{ProofRead, SparseMerkleTree};
use aptos_types::{
    account_config::NEW_EPOCH_EVENT_MOVE_TYPE_TAG,
    block_executor::{config::BlockExecutorConfigFromOnchain, partitioner::ExecutableBlock},
    contract_event::ContractEvent,
    dkg::DKG_START_EVENT_MOVE_TYPE_TAG,
    epoch_state::EpochState,
    jwks::OBSERVED_JWK_UPDATED_MOVE_TYPE_TAG,
    ledger_info::LedgerInfoWithSignatures,
    proof::{AccumulatorExtensionProof, SparseMerkleProofExt},
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{
        block_epilogue::{BlockEndInfo, BlockEpiloguePayload},
        ExecutionStatus, Transaction, TransactionInfo, TransactionListWithProof,
        TransactionOutputListWithProof, TransactionStatus, Version,
    },
    write_set::WriteSet,
};
pub use error::{ExecutorError, ExecutorResult};
pub use executed_chunk::ExecutedChunk;
pub use ledger_update_output::LedgerUpdateOutput;
pub use parsed_transaction_output::ParsedTransactionOutput;
use serde::{Deserialize, Serialize};
use std::{
    cmp::max,
    collections::{BTreeSet, HashMap},
    fmt::Debug,
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

mod error;
mod executed_chunk;
pub mod execution_output;
mod ledger_update_output;
pub mod parsed_transaction_output;
pub mod state_checkpoint_output;

pub trait ChunkExecutorTrait: Send + Sync {
    /// Verifies the transactions based on the provided proofs and ledger info. If the transactions
    /// are valid, executes them and returns the executed result for commit.
    #[cfg(any(test, feature = "fuzzing"))]
    fn execute_chunk(
        &self,
        txn_list_with_proof: TransactionListWithProof,
        // Target LI that has been verified independently: the proofs are relative to this version.
        verified_target_li: &LedgerInfoWithSignatures,
        epoch_change_li: Option<&LedgerInfoWithSignatures>,
    ) -> Result<()> {
        self.enqueue_chunk_by_execution(txn_list_with_proof, verified_target_li, epoch_change_li)?;

        self.update_ledger()
    }

    /// Similar to `execute_chunk`, but instead of executing transactions, apply the transaction
    /// outputs directly to get the executed result.
    #[cfg(any(test, feature = "fuzzing"))]
    fn apply_chunk(
        &self,
        txn_output_list_with_proof: TransactionOutputListWithProof,
        // Target LI that has been verified independently: the proofs are relative to this version.
        verified_target_li: &LedgerInfoWithSignatures,
        epoch_change_li: Option<&LedgerInfoWithSignatures>,
    ) -> Result<()> {
        self.enqueue_chunk_by_transaction_outputs(
            txn_output_list_with_proof,
            verified_target_li,
            epoch_change_li,
        )?;

        self.update_ledger()
    }

    /// Verifies the transactions based on the provided proofs and ledger info. If the transactions
    /// are valid, executes them and make state checkpoint, so that a later chunk of transaction can
    /// be applied on top of it. This stage calculates the state checkpoint, but not the top level
    /// transaction accumulator.
    fn enqueue_chunk_by_execution(
        &self,
        txn_list_with_proof: TransactionListWithProof,
        // Target LI that has been verified independently: the proofs are relative to this version.
        verified_target_li: &LedgerInfoWithSignatures,
        epoch_change_li: Option<&LedgerInfoWithSignatures>,
    ) -> Result<()>;

    /// Similar to `enqueue_chunk_by_execution`, but instead of executing transactions, apply the
    /// transaction outputs directly to get the executed result.
    fn enqueue_chunk_by_transaction_outputs(
        &self,
        txn_output_list_with_proof: TransactionOutputListWithProof,
        // Target LI that has been verified independently: the proofs are relative to this version.
        verified_target_li: &LedgerInfoWithSignatures,
        epoch_change_li: Option<&LedgerInfoWithSignatures>,
    ) -> Result<()>;

    /// As a separate stage, calculate the transaction accumulator changes, prepare for db commission.
    fn update_ledger(&self) -> Result<()>;

    /// Commit a previously executed chunk. Returns a chunk commit notification.
    fn commit_chunk(&self) -> Result<ChunkCommitNotification>;

    /// Resets the chunk executor by synchronizing state with storage.
    fn reset(&self) -> Result<()>;

    /// Finishes the chunk executor by releasing memory held by inner data structures(SMT).
    fn finish(&self);
}

pub struct StateSnapshotDelta {
    pub version: Version,
    pub smt: SparseMerkleTree<StateValue>,
    pub jmt_updates: Vec<(HashValue, (HashValue, StateKey))>,
}

pub trait BlockExecutorTrait: Send + Sync {
    /// Get the latest committed block id
    fn committed_block_id(&self) -> HashValue;

    /// Reset the internal state including cache with newly fetched latest committed block from storage.
    fn reset(&self) -> Result<()>;

    /// Executes a block - TBD, this API will be removed in favor of `execute_and_state_checkpoint`, followed
    /// by `ledger_update` once we have ledger update as a separate pipeline phase.
    #[cfg(any(test, feature = "fuzzing"))]
    fn execute_block(
        &self,
        block: ExecutableBlock,
        parent_block_id: HashValue,
        onchain_config: BlockExecutorConfigFromOnchain,
    ) -> ExecutorResult<StateComputeResult> {
        let block_id = block.block_id;
        let state_checkpoint_output =
            self.execute_and_state_checkpoint(block, parent_block_id, onchain_config)?;
        self.ledger_update(block_id, parent_block_id, state_checkpoint_output)
    }

    /// Executes a block and returns the state checkpoint output.
    fn execute_and_state_checkpoint(
        &self,
        block: ExecutableBlock,
        parent_block_id: HashValue,
        onchain_config: BlockExecutorConfigFromOnchain,
    ) -> ExecutorResult<StateCheckpointOutput>;

    fn ledger_update(
        &self,
        block_id: HashValue,
        parent_block_id: HashValue,
        state_checkpoint_output: StateCheckpointOutput,
    ) -> ExecutorResult<StateComputeResult>;

    #[cfg(any(test, feature = "fuzzing"))]
    fn commit_blocks(
        &self,
        block_ids: Vec<HashValue>,
        ledger_info_with_sigs: LedgerInfoWithSignatures,
    ) -> ExecutorResult<()> {
        let mut parent_block_id = self.committed_block_id();
        for block_id in block_ids {
            self.pre_commit_block(block_id, parent_block_id)?;
            parent_block_id = block_id;
        }
        self.commit_ledger(ledger_info_with_sigs)
    }

    fn pre_commit_block(
        &self,
        block_id: HashValue,
        parent_block_id: HashValue,
    ) -> ExecutorResult<()>;

    fn commit_ledger(&self, ledger_info_with_sigs: LedgerInfoWithSignatures) -> ExecutorResult<()>;

    /// Finishes the block executor by releasing memory held by inner data structures(SMT).
    fn finish(&self);
}

#[derive(Clone)]
pub enum VerifyExecutionMode {
    NoVerify,
    Verify {
        txns_to_skip: Arc<BTreeSet<Version>>,
        lazy_quit: bool,
        seen_error: Arc<AtomicBool>,
    },
}

impl VerifyExecutionMode {
    pub fn verify_all() -> Self {
        Self::Verify {
            txns_to_skip: Arc::new(BTreeSet::new()),
            lazy_quit: false,
            seen_error: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn verify_except(txns_to_skip: Vec<Version>) -> Self {
        Self::Verify {
            txns_to_skip: Arc::new(txns_to_skip.into_iter().collect()),
            lazy_quit: false,
            seen_error: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn txns_to_skip(&self) -> Arc<BTreeSet<Version>> {
        match self {
            VerifyExecutionMode::NoVerify => Arc::new(BTreeSet::new()),
            VerifyExecutionMode::Verify { txns_to_skip, .. } => txns_to_skip.clone(),
        }
    }

    pub fn set_lazy_quit(mut self, is_lazy_quit: bool) -> Self {
        if let Self::Verify {
            ref mut lazy_quit, ..
        } = self
        {
            *lazy_quit = is_lazy_quit
        }
        self
    }

    pub fn is_lazy_quit(&self) -> bool {
        match self {
            VerifyExecutionMode::NoVerify => false,
            VerifyExecutionMode::Verify { lazy_quit, .. } => *lazy_quit,
        }
    }

    pub fn mark_seen_error(&self) {
        match self {
            VerifyExecutionMode::NoVerify => unreachable!("Should not call in no-verify mode."),
            VerifyExecutionMode::Verify { seen_error, .. } => {
                seen_error.store(true, Ordering::Relaxed)
            },
        }
    }

    pub fn should_verify(&self) -> bool {
        !matches!(self, Self::NoVerify)
    }

    pub fn seen_error(&self) -> bool {
        match self {
            VerifyExecutionMode::NoVerify => false,
            VerifyExecutionMode::Verify { seen_error, .. } => seen_error.load(Ordering::Relaxed),
        }
    }
}

pub trait TransactionReplayer: Send {
    fn replay(
        &self,
        transactions: Vec<Transaction>,
        transaction_infos: Vec<TransactionInfo>,
        write_sets: Vec<WriteSet>,
        event_vecs: Vec<Vec<ContractEvent>>,
        verify_execution_mode: &VerifyExecutionMode,
    ) -> Result<()>;

    fn commit(&self) -> Result<ExecutedChunk>;
}

/// A structure that holds relevant information about a chunk that was committed.
#[derive(Clone)]
pub struct ChunkCommitNotification {
    pub subscribable_events: Vec<ContractEvent>,
    pub committed_transactions: Vec<Transaction>,
    pub reconfiguration_occurred: bool,
}

/// A structure that summarizes the result of the execution needed for consensus to agree on.
/// The execution is responsible for generating the ID of the new state, which is returned in the
/// result.
///
/// Not every transaction in the payload succeeds: the returned vector keeps the boolean status
/// of success / failure of the transactions.
/// Note that the specific details of compute_status are opaque to StateMachineReplication,
/// which is going to simply pass the results between StateComputer and PayloadClient.
#[derive(Debug, Default, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct StateComputeResult {
    /// transaction accumulator root hash is identified as `state_id` in Consensus.
    root_hash: HashValue,
    /// Represents the roots of all the full subtrees from left to right in this accumulator
    /// after the execution. For details, please see [`InMemoryAccumulator`](aptos_types::proof::accumulator::InMemoryAccumulator).
    frozen_subtree_roots: Vec<HashValue>,

    /// The frozen subtrees roots of the parent block,
    parent_frozen_subtree_roots: Vec<HashValue>,

    /// The number of leaves of the transaction accumulator after executing a proposed block.
    /// This state must be persisted to ensure that on restart that the version is calculated correctly.
    num_leaves: u64,

    /// The number of leaves after executing the parent block,
    parent_num_leaves: u64,

    /// If set, this is the new epoch info that should be changed to if this block is committed.
    epoch_state: Option<EpochState>,
    /// The compute status (success/failure) of the given payload. The specific details are opaque
    /// for StateMachineReplication, which is merely passing it between StateComputer and
    /// PayloadClient.
    ///
    /// Here, only input transactions statuses are kept, and in their order.
    /// Input includes BlockMetadata, but doesn't include StateCheckpoint/BlockEpilogue
    compute_status_for_input_txns: Vec<TransactionStatus>,

    /// The transaction info hashes of all success txns.
    transaction_info_hashes: Vec<HashValue>,

    subscribable_events: Vec<ContractEvent>,

    block_end_info: Option<BlockEndInfo>,
}

impl StateComputeResult {
    pub fn new(
        root_hash: HashValue,
        frozen_subtree_roots: Vec<HashValue>,
        num_leaves: u64,
        parent_frozen_subtree_roots: Vec<HashValue>,
        parent_num_leaves: u64,
        epoch_state: Option<EpochState>,
        compute_status_for_input_txns: Vec<TransactionStatus>,
        transaction_info_hashes: Vec<HashValue>,
        subscribable_events: Vec<ContractEvent>,
        block_end_info: Option<BlockEndInfo>,
    ) -> Self {
        Self {
            root_hash,
            frozen_subtree_roots,
            num_leaves,
            parent_frozen_subtree_roots,
            parent_num_leaves,
            epoch_state,
            compute_status_for_input_txns,
            transaction_info_hashes,
            subscribable_events,
            block_end_info,
        }
    }

    /// generate a new dummy state compute result with a given root hash.
    /// this function is used in RandomComputeResultStateComputer to assert that the compute
    /// function is really called.
    pub fn new_dummy_with_root_hash(root_hash: HashValue) -> Self {
        Self {
            root_hash,
            frozen_subtree_roots: vec![],
            num_leaves: 0,
            parent_frozen_subtree_roots: vec![],
            parent_num_leaves: 0,
            epoch_state: None,
            compute_status_for_input_txns: vec![],
            transaction_info_hashes: vec![],
            subscribable_events: vec![],
            block_end_info: None,
        }
    }

    pub fn new_dummy_with_num_txns(num_txns: usize) -> Self {
        Self {
            root_hash: HashValue::zero(),
            frozen_subtree_roots: vec![],
            num_leaves: 0,
            parent_frozen_subtree_roots: vec![],
            parent_num_leaves: 0,
            epoch_state: None,
            compute_status_for_input_txns: vec![
                TransactionStatus::Keep(ExecutionStatus::Success);
                num_txns
            ],
            transaction_info_hashes: vec![],
            subscribable_events: vec![],
            block_end_info: None,
        }
    }

    /// generate a new dummy state compute result with ACCUMULATOR_PLACEHOLDER_HASH as the root hash.
    /// this function is used in ordering_state_computer as a dummy state compute result,
    /// where the real compute result is generated after ordering_state_computer.commit pushes
    /// the blocks and the finality proof to the execution phase.
    pub fn new_dummy() -> Self {
        StateComputeResult::new_dummy_with_root_hash(*ACCUMULATOR_PLACEHOLDER_HASH)
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_dummy_with_compute_status(compute_status: Vec<TransactionStatus>) -> Self {
        let mut ret = Self::new_dummy();
        ret.compute_status_for_input_txns = compute_status;
        ret
    }

    pub fn version(&self) -> Version {
        max(self.num_leaves, 1)
            .checked_sub(1)
            .expect("Integer overflow occurred")
    }

    pub fn root_hash(&self) -> HashValue {
        self.root_hash
    }

    pub fn compute_status_for_input_txns(&self) -> &Vec<TransactionStatus> {
        &self.compute_status_for_input_txns
    }

    pub fn transactions_to_commit_len(&self) -> usize {
        self.compute_status_for_input_txns()
            .iter()
            .filter(|status| matches!(status, TransactionStatus::Keep(_)))
            .count()
            // StateCheckpoint/BlockEpilogue is added if there is no reconfiguration
            + (if self.has_reconfiguration() { 0 } else { 1 })
    }

    /// On top of input transactions (which contain BlockMetadata and Validator txns),
    /// filter out those that should be committed, and add StateCheckpoint/BlockEpilogue if needed.
    pub fn transactions_to_commit(
        &self,
        input_txns: Vec<Transaction>,
        block_id: HashValue,
    ) -> Vec<Transaction> {
        assert_eq!(
            input_txns.len(),
            self.compute_status_for_input_txns().len(),
            "{:?} != {:?}",
            input_txns.iter().map(|t| t.type_name()).collect::<Vec<_>>(),
            self.compute_status_for_input_txns()
        );
        let output = itertools::zip_eq(input_txns, self.compute_status_for_input_txns())
            .filter_map(|(txn, status)| {
                assert!(
                    !txn.is_non_reconfig_block_ending(),
                    "{:?}: {:?}",
                    txn,
                    status
                );
                match status {
                    TransactionStatus::Keep(_) => Some(txn),
                    _ => None,
                }
            })
            .chain(
                (!self.has_reconfiguration()).then_some(self.block_end_info.clone().map_or(
                    Transaction::StateCheckpoint(block_id),
                    |block_end_info| {
                        Transaction::BlockEpilogue(BlockEpiloguePayload::V0 {
                            block_id,
                            block_end_info,
                        })
                    },
                )),
            )
            .collect::<Vec<_>>();

        assert!(
            self.has_reconfiguration()
                || output
                    .last()
                    .map_or(false, Transaction::is_non_reconfig_block_ending),
            "{:?}",
            output.last()
        );

        output
    }

    pub fn epoch_state(&self) -> &Option<EpochState> {
        &self.epoch_state
    }

    pub fn extension_proof(&self) -> AccumulatorExtensionProof<TransactionAccumulatorHasher> {
        AccumulatorExtensionProof::<TransactionAccumulatorHasher>::new(
            self.parent_frozen_subtree_roots.clone(),
            self.parent_num_leaves(),
            self.transaction_info_hashes().clone(),
        )
    }

    pub fn transaction_info_hashes(&self) -> &Vec<HashValue> {
        &self.transaction_info_hashes
    }

    pub fn num_leaves(&self) -> u64 {
        self.num_leaves
    }

    pub fn frozen_subtree_roots(&self) -> &Vec<HashValue> {
        &self.frozen_subtree_roots
    }

    pub fn parent_num_leaves(&self) -> u64 {
        self.parent_num_leaves
    }

    pub fn parent_frozen_subtree_roots(&self) -> &Vec<HashValue> {
        &self.parent_frozen_subtree_roots
    }

    pub fn has_reconfiguration(&self) -> bool {
        self.epoch_state.is_some()
    }

    pub fn subscribable_events(&self) -> &[ContractEvent] {
        &self.subscribable_events
    }
}

pub struct ProofReader {
    proofs: HashMap<HashValue, SparseMerkleProofExt>,
}

impl ProofReader {
    pub fn new(proofs: HashMap<HashValue, SparseMerkleProofExt>) -> Self {
        ProofReader { proofs }
    }

    pub fn new_empty() -> Self {
        Self::new(HashMap::new())
    }
}

impl ProofRead for ProofReader {
    fn get_proof(&self, key: HashValue) -> Option<&SparseMerkleProofExt> {
        self.proofs.get(&key)
    }
}

/// Used in both state sync and consensus to filter the txn events that should be subscribable by node components.
pub fn should_forward_to_subscription_service(event: &ContractEvent) -> bool {
    let type_tag = event.type_tag();
    type_tag == OBSERVED_JWK_UPDATED_MOVE_TYPE_TAG.deref()
        || type_tag == DKG_START_EVENT_MOVE_TYPE_TAG.deref()
        || type_tag == NEW_EPOCH_EVENT_MOVE_TYPE_TAG.deref()
}

#[cfg(feature = "bench")]
pub fn should_forward_to_subscription_service_old(event: &ContractEvent) -> bool {
    matches!(
        event.type_tag().to_string().as_str(),
        "0x1::reconfiguration::NewEpochEvent"
            | "0x1::dkg::DKGStartEvent"
            | "\
            0x1::jwks::ObservedJWKsUpdated"
    )
}
