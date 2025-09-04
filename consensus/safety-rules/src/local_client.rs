// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{ConsensusState, Error, SafetyRules, TSafetyRules};
use velor_consensus_types::{
    block_data::BlockData,
    order_vote::OrderVote,
    order_vote_proposal::OrderVoteProposal,
    timeout_2chain::{TwoChainTimeout, TwoChainTimeoutCertificate},
    vote::Vote,
    vote_proposal::VoteProposal,
};
use velor_crypto::bls12381;
use velor_infallible::RwLock;
use velor_types::{
    epoch_change::EpochChangeProof,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
};
use std::sync::Arc;

/// A local interface into SafetyRules. Constructed in such a way that the container / caller
/// cannot distinguish this API from an actual client/server process without being exposed to
/// the actual container instead the caller can access a Box<dyn TSafetyRules>.
pub struct LocalClient {
    internal: Arc<RwLock<SafetyRules>>,
}

impl LocalClient {
    pub fn new(internal: Arc<RwLock<SafetyRules>>) -> Self {
        Self { internal }
    }
}

impl TSafetyRules for LocalClient {
    fn consensus_state(&mut self) -> Result<ConsensusState, Error> {
        self.internal.write().consensus_state()
    }

    fn initialize(&mut self, proof: &EpochChangeProof) -> Result<(), Error> {
        self.internal.write().initialize(proof)
    }

    fn sign_proposal(&mut self, block_data: &BlockData) -> Result<bls12381::Signature, Error> {
        self.internal.write().sign_proposal(block_data)
    }

    fn sign_timeout_with_qc(
        &mut self,
        timeout: &TwoChainTimeout,
        timeout_cert: Option<&TwoChainTimeoutCertificate>,
    ) -> Result<bls12381::Signature, Error> {
        self.internal
            .write()
            .sign_timeout_with_qc(timeout, timeout_cert)
    }

    fn construct_and_sign_vote_two_chain(
        &mut self,
        vote_proposal: &VoteProposal,
        timeout_cert: Option<&TwoChainTimeoutCertificate>,
    ) -> Result<Vote, Error> {
        self.internal
            .write()
            .construct_and_sign_vote_two_chain(vote_proposal, timeout_cert)
    }

    fn construct_and_sign_order_vote(
        &mut self,
        order_vote_proposal: &OrderVoteProposal,
    ) -> Result<OrderVote, Error> {
        self.internal
            .write()
            .construct_and_sign_order_vote(order_vote_proposal)
    }

    fn sign_commit_vote(
        &mut self,
        ledger_info: LedgerInfoWithSignatures,
        new_ledger_info: LedgerInfo,
    ) -> Result<bls12381::Signature, Error> {
        self.internal
            .write()
            .sign_commit_vote(ledger_info, new_ledger_info)
    }
}
