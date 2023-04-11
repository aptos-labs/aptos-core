// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block::Block,
    common::{Payload, Round},
    quorum_cert::QuorumCert,
    vote_proposal::VoteProposal,
};
use aptos_crypto::hash::HashValue;
use aptos_executor_types::StateComputeResult;
use aptos_types::{
    account_address::AccountAddress,
    block_info::BlockInfo,
    contract_event::ContractEvent,
    transaction::{SignedTransaction, Transaction, TransactionStatus},
};
use std::fmt::{Debug, Display, Formatter};

/// ExecutedBlocks are managed in a speculative tree, the committed blocks form a chain. Besides
/// block data, each executed block also has other derived meta data which could be regenerated from
/// blocks.
#[derive(Clone, Eq, PartialEq)]
pub struct ExecutedBlock {
    /// Block data that cannot be regenerated.
    block: Block,
    /// The state_compute_result is calculated for all the pending blocks prior to insertion to
    /// the tree. The execution results are not persisted: they're recalculated again for the
    /// pending blocks upon restart.
    state_compute_result: StateComputeResult,
}

impl Debug for ExecutedBlock {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for ExecutedBlock {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self.block())
    }
}

impl ExecutedBlock {
    pub fn new(block: Block, state_compute_result: StateComputeResult) -> Self {
        Self {
            block,
            state_compute_result,
        }
    }

    pub fn block(&self) -> &Block {
        &self.block
    }

    pub fn id(&self) -> HashValue {
        self.block().id()
    }

    pub fn epoch(&self) -> u64 {
        self.block.epoch()
    }

    pub fn payload(&self) -> Option<&Payload> {
        self.block().payload()
    }

    pub fn parent_id(&self) -> HashValue {
        self.quorum_cert().certified_block().id()
    }

    pub fn quorum_cert(&self) -> &QuorumCert {
        self.block().quorum_cert()
    }

    pub fn round(&self) -> Round {
        self.block().round()
    }

    pub fn timestamp_usecs(&self) -> u64 {
        self.block().timestamp_usecs()
    }

    pub fn compute_result(&self) -> &StateComputeResult {
        &self.state_compute_result
    }

    pub fn block_info(&self) -> BlockInfo {
        self.block().gen_block_info(
            self.compute_result().root_hash(),
            self.compute_result().version(),
            self.compute_result().epoch_state().clone(),
        )
    }

    pub fn vote_proposal(&self, decoupled_execution: bool) -> VoteProposal {
        VoteProposal::new(
            self.compute_result().extension_proof(),
            self.block.clone(),
            self.compute_result().epoch_state().clone(),
            decoupled_execution,
        )
    }

    pub fn transactions_to_commit(
        &self,
        validators: &[AccountAddress],
        txns: Vec<SignedTransaction>,
        block_gas_limit: Option<u64>,
    ) -> Vec<Transaction> {
        // reconfiguration suffix don't execute

        if self.is_reconfiguration_suffix() {
            return vec![];
        }

        let mut txns_with_state_checkpoint =
            self.block
                .transactions_to_execute(validators, txns, block_gas_limit);
        if block_gas_limit.is_some() && !self.state_compute_result.has_reconfiguration() {
            // After the per-block gas limit change,
            // insert state checkpoint at the position
            // 1) after last txn if there is no Retry
            // 2) before the first Retry
            if let Some(pos) = self
                .state_compute_result
                .compute_status()
                .iter()
                .position(|s| s.is_retry())
            {
                txns_with_state_checkpoint.insert(pos, Transaction::StateCheckpoint(self.id()));
            } else {
                txns_with_state_checkpoint.push(Transaction::StateCheckpoint(self.id()));
            }
        }

        itertools::zip_eq(
            txns_with_state_checkpoint,
            self.state_compute_result.compute_status(),
        )
        .filter_map(|(txn, status)| match status {
            TransactionStatus::Keep(_) => Some(txn),
            _ => None,
        })
        .collect()
    }

    pub fn reconfig_event(&self) -> Vec<ContractEvent> {
        // reconfiguration suffix don't count, the state compute result is carried over from parents
        if self.is_reconfiguration_suffix() {
            return vec![];
        }
        self.state_compute_result.reconfig_events().to_vec()
    }

    /// The block is suffix of a reconfiguration block if the state result carries over the epoch state
    /// from parent but has no transaction.
    pub fn is_reconfiguration_suffix(&self) -> bool {
        self.state_compute_result.has_reconfiguration()
            && self.state_compute_result.compute_status().is_empty()
    }
}
