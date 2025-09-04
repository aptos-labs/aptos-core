// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{ConsensusState, Error};
use velor_consensus_types::{
    block_data::BlockData,
    order_vote::OrderVote,
    order_vote_proposal::OrderVoteProposal,
    timeout_2chain::{TwoChainTimeout, TwoChainTimeoutCertificate},
    vote::Vote,
    vote_proposal::VoteProposal,
};
use velor_crypto::bls12381;
use velor_types::{
    epoch_change::EpochChangeProof,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
};

/// Interface for SafetyRules
pub trait TSafetyRules {
    /// Provides the internal state of SafetyRules for monitoring / debugging purposes. This does
    /// not include sensitive data like private keys.
    fn consensus_state(&mut self) -> Result<ConsensusState, Error>;

    /// Initialize SafetyRules using an Epoch ending LedgerInfo, this should map to what was
    /// provided in consensus_state. It will be used to initialize the ValidatorSet.
    /// This uses a EpochChangeProof because there's a possibility that consensus migrated to a
    /// new epoch but SafetyRules did not.
    fn initialize(&mut self, proof: &EpochChangeProof) -> Result<(), Error>;

    /// As the holder of the private key, SafetyRules also signs proposals or blocks.
    /// A Block is a signed BlockData along with some additional metadata.
    fn sign_proposal(&mut self, block_data: &BlockData) -> Result<bls12381::Signature, Error>;

    /// Sign the timeout together with highest qc for 2-chain protocol.
    fn sign_timeout_with_qc(
        &mut self,
        timeout: &TwoChainTimeout,
        timeout_cert: Option<&TwoChainTimeoutCertificate>,
    ) -> Result<bls12381::Signature, Error>;

    /// Attempts to vote for a given proposal following the 2-chain protocol.
    fn construct_and_sign_vote_two_chain(
        &mut self,
        vote_proposal: &VoteProposal,
        timeout_cert: Option<&TwoChainTimeoutCertificate>,
    ) -> Result<Vote, Error>;

    /// Attempts to create an order vote for a block given the quroum certificate for the block.
    fn construct_and_sign_order_vote(
        &mut self,
        order_vote_proposal: &OrderVoteProposal,
    ) -> Result<OrderVote, Error>;

    /// As the holder of the private key, SafetyRules also signs a commit vote.
    /// This returns the signature for the commit vote.
    fn sign_commit_vote(
        &mut self,
        ledger_info: LedgerInfoWithSignatures,
        new_ledger_info: LedgerInfo,
    ) -> Result<bls12381::Signature, Error>;
}
