// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Message types for proxy primary consensus.
//!
//! These messages are exchanged between proxy validators during proxy consensus,
//! and between proxies and primaries for block forwarding.

use crate::{
    block::Block,
    common::{Author, Round},
    opt_block_data::OptBlockData,
    order_vote::OrderVote,
    primary_consensus_proof::PrimaryConsensusProof,
    proof_of_store::ProofCache,
    proxy_sync_info::ProxySyncInfo,
    quorum_cert::QuorumCert,
    vote::Vote,
};
use anyhow::{ensure, format_err, Context, Result};
use aptos_short_hex_str::AsShortHexStr;
use aptos_types::validator_verifier::ValidatorVerifier;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};

// ============================================================================
// ProxyProposalMsg - Regular proxy proposal with parent QC
// ============================================================================

/// ProxyProposalMsg contains a proxy block with its parent QC and sync info.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProxyProposalMsg {
    proposal: Block,
    sync_info: ProxySyncInfo,
}

impl ProxyProposalMsg {
    pub fn new(proposal: Block, sync_info: ProxySyncInfo) -> Self {
        Self {
            proposal,
            sync_info,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.proposal.epoch()
    }

    pub fn proposal(&self) -> &Block {
        &self.proposal
    }

    pub fn take_proposal(self) -> Block {
        self.proposal
    }

    pub fn sync_info(&self) -> &ProxySyncInfo {
        &self.sync_info
    }

    pub fn proposer(&self) -> Author {
        self.proposal
            .author()
            .expect("Proxy proposal should have an author")
    }

    /// Verifies that the ProxyProposalMsg is well-formed.
    pub fn verify_well_formed(&self) -> Result<()> {
        ensure!(
            self.proposal.block_data().is_proxy_block(),
            "ProxyProposalMsg must contain a proxy block"
        );
        ensure!(
            !self.proposal.is_nil_block(),
            "Proxy proposal {} cannot be a NIL block",
            self.proposal
        );
        self.proposal
            .verify_well_formed()
            .context("Failed to verify proxy proposal block")?;
        ensure!(
            self.proposal.round() > 0,
            "Proxy proposal for {} has an incorrect round of 0",
            self.proposal,
        );
        ensure!(
            self.proposal.epoch() == self.sync_info.epoch(),
            "ProxyProposalMsg has different epoch number from ProxySyncInfo"
        );
        ensure!(
            self.proposal.parent_id()
                == self.sync_info.highest_proxy_qc().certified_block().id(),
            "Proxy proposal parent id {} doesn't match sync info highest proxy QC block {}",
            self.proposal.parent_id(),
            self.sync_info.highest_proxy_qc().certified_block().id(),
        );
        ensure!(
            self.proposal.author().is_some(),
            "Proxy proposal {} does not define an author",
            self.proposal
        );
        Ok(())
    }

    pub fn verify(
        &self,
        sender: Author,
        validator: &ValidatorVerifier,
        proof_cache: &ProofCache,
        quorum_store_enabled: bool,
    ) -> Result<()> {
        if let Some(proposal_author) = self.proposal.author() {
            ensure!(
                proposal_author == sender,
                "Proxy proposal author {:?} doesn't match sender {:?}",
                proposal_author,
                sender
            );
        }

        let (payload_result, sig_result) = rayon::join(
            || {
                self.proposal().payload().map_or(Ok(()), |p| {
                    p.verify(validator, proof_cache, quorum_store_enabled, false)
                })
            },
            || {
                self.proposal()
                    .validate_signature(validator)
                    .map_err(|e| format_err!("{:?}", e))
            },
        );
        payload_result?;
        sig_result?;

        if let Some(tc) = self.sync_info.highest_proxy_timeout_cert() {
            tc.verify(validator).map_err(|e| format_err!("{:?}", e))?;
        }

        self.verify_well_formed()
    }
}

impl Display for ProxyProposalMsg {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "[proxy proposal {} from ", self.proposal)?;
        match self.proposal.author() {
            Some(author) => write!(f, "{}]", author.short_str()),
            None => write!(f, "NIL]"),
        }
    }
}

// ============================================================================
// OptProxyProposalMsg - Optimistic proxy proposal (grandparent QC only)
// ============================================================================

/// OptProxyProposalMsg contains an optimistic proxy block data (OptBlockData with
/// OptBlockBody::ProxyV0) and sync info. Used for 1-message-delay block time in
/// proxy consensus.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OptProxyProposalMsg {
    block_data: OptBlockData,
    sync_info: ProxySyncInfo,
}

impl OptProxyProposalMsg {
    pub fn new(block_data: OptBlockData, sync_info: ProxySyncInfo) -> Self {
        Self {
            block_data,
            sync_info,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.block_data.epoch()
    }

    pub fn block_data(&self) -> &OptBlockData {
        &self.block_data
    }

    pub fn take_block_data(self) -> OptBlockData {
        self.block_data
    }

    pub fn sync_info(&self) -> &ProxySyncInfo {
        &self.sync_info
    }

    pub fn proposer(&self) -> &Author {
        self.block_data.author()
    }

    pub fn verify_well_formed(&self) -> Result<()> {
        self.block_data
            .verify_well_formed()
            .context("Failed to verify OptBlockData (proxy)")?;
        ensure!(
            self.block_data.epoch() == self.sync_info.epoch(),
            "OptProxyProposalMsg has different epoch number from ProxySyncInfo"
        );
        Ok(())
    }

    pub fn verify(
        &self,
        sender: Author,
        validator: &ValidatorVerifier,
        proof_cache: &ProofCache,
        quorum_store_enabled: bool,
    ) -> Result<()> {
        ensure!(
            *self.block_data.author() == sender,
            "Opt proxy proposal author {:?} doesn't match sender {:?}",
            self.block_data.author(),
            sender
        );

        // Verify grandparent QC
        self.block_data
            .grandparent_qc()
            .verify(validator)
            .context("Failed to verify grandparent QC")?;

        // Verify payload if present
        self.block_data
            .payload()
            .verify(validator, proof_cache, quorum_store_enabled, false)
            .context("Failed to verify payload")?;

        // Verify primary proof (QC or TC) if attached
        if let Some(primary_proof) = self.block_data.primary_proof() {
            primary_proof
                .verify(validator)
                .context("Failed to verify attached primary proof")?;
        }

        self.verify_well_formed()
    }
}

impl Display for OptProxyProposalMsg {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "[opt proxy proposal round {} from {}]",
            self.block_data.round(),
            self.block_data.author().short_str()
        )
    }
}

// ============================================================================
// ProxyVoteMsg - Vote on proxy block
// ============================================================================

/// ProxyVoteMsg is sent by validators in response to receiving a proxy proposal.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct ProxyVoteMsg {
    vote: Vote,
    sync_info: ProxySyncInfo,
}

impl ProxyVoteMsg {
    pub fn new(vote: Vote, sync_info: ProxySyncInfo) -> Self {
        Self { vote, sync_info }
    }

    pub fn vote(&self) -> &Vote {
        &self.vote
    }

    pub fn sync_info(&self) -> &ProxySyncInfo {
        &self.sync_info
    }

    pub fn epoch(&self) -> u64 {
        self.vote.epoch()
    }

    pub fn verify(&self, sender: Author, validator: &ValidatorVerifier) -> Result<()> {
        ensure!(
            self.vote.author() == sender,
            "Proxy vote author {:?} is different from sender {:?}",
            self.vote.author(),
            sender,
        );
        ensure!(
            self.vote.epoch() == self.sync_info.epoch(),
            "ProxyVoteMsg has different epoch"
        );
        ensure!(
            self.vote.vote_data().proposed().round() > self.sync_info.highest_proxy_round(),
            "Proxy vote round should be higher than sync info highest proxy round"
        );
        self.vote.verify(validator)
    }
}

impl Display for ProxyVoteMsg {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "ProxyVoteMsg: [{}], ProxySyncInfo: [{}]",
            self.vote, self.sync_info
        )
    }
}

// ============================================================================
// ProxyOrderVoteMsg - Order vote on proxy block
// ============================================================================

/// ProxyOrderVoteMsg is broadcasted by a proxy validator when it receives a QC on a proxy block.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProxyOrderVoteMsg {
    order_vote: OrderVote,
    quorum_cert: QuorumCert,
}

impl ProxyOrderVoteMsg {
    pub fn new(order_vote: OrderVote, quorum_cert: QuorumCert) -> Self {
        Self {
            order_vote,
            quorum_cert,
        }
    }

    pub fn order_vote(&self) -> &OrderVote {
        &self.order_vote
    }

    pub fn quorum_cert(&self) -> &QuorumCert {
        &self.quorum_cert
    }

    pub fn epoch(&self) -> u64 {
        self.order_vote.epoch()
    }

    pub fn verify_order_vote(
        &self,
        sender: Author,
        validator: &ValidatorVerifier,
    ) -> Result<()> {
        ensure!(
            self.order_vote.author() == sender,
            "Proxy order vote author {:?} is different from sender {:?}",
            self.order_vote.author(),
            sender
        );
        ensure!(
            self.quorum_cert.certified_block() == self.order_vote.ledger_info().commit_info(),
            "QuorumCert and OrderVote do not match"
        );
        self.order_vote
            .verify(validator)
            .context("[ProxyOrderVoteMsg] OrderVote verification failed")?;
        Ok(())
    }
}

impl Display for ProxyOrderVoteMsg {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "ProxyOrderVoteMsg: [{}], QuorumCert: [{}]",
            self.order_vote, self.quorum_cert
        )
    }
}

// ============================================================================
// ProxyRoundTimeoutMsg - Proxy variant of RoundTimeoutMsg
// ============================================================================

/// Proxy variant of RoundTimeoutMsg for network routing to the proxy RoundManager.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct ProxyRoundTimeoutMsg {
    round_timeout: crate::round_timeout::RoundTimeoutMsg,
}

impl ProxyRoundTimeoutMsg {
    pub fn new(round_timeout: crate::round_timeout::RoundTimeoutMsg) -> Self {
        Self { round_timeout }
    }

    pub fn round_timeout(&self) -> &crate::round_timeout::RoundTimeoutMsg {
        &self.round_timeout
    }

    pub fn into_round_timeout(self) -> crate::round_timeout::RoundTimeoutMsg {
        self.round_timeout
    }
}

impl Display for ProxyRoundTimeoutMsg {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "ProxyRoundTimeoutMsg: [{}]", self.round_timeout)
    }
}

// ============================================================================
// OrderedProxyBlocksMsg - Forwarded to all primaries
// ============================================================================

/// OrderedProxyBlocksMsg contains ordered proxy blocks ending at a cutting point.
/// This is broadcast to ALL primaries (not just proxies) when proxy blocks are ordered.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OrderedProxyBlocksMsg {
    /// Ordered proxy blocks linked by parent hashes.
    /// last_primary_proof_round is non-decreasing across blocks.
    proxy_blocks: Vec<Block>,
    /// The primary consensus proof (QC or TC) that "cut" these proxy blocks.
    /// Primary round for this batch = proof.proof_round() + 1.
    primary_proof: PrimaryConsensusProof,
}

impl OrderedProxyBlocksMsg {
    pub fn new(
        proxy_blocks: Vec<Block>,
        primary_proof: PrimaryConsensusProof,
    ) -> Self {
        Self {
            proxy_blocks,
            primary_proof,
        }
    }

    pub fn proxy_blocks(&self) -> &[Block] {
        &self.proxy_blocks
    }

    pub fn take_proxy_blocks(self) -> Vec<Block> {
        self.proxy_blocks
    }

    /// The primary round this batch belongs to, derived from the proof.
    pub fn primary_round(&self) -> Round {
        self.primary_proof.proof_round() + 1
    }

    pub fn primary_proof(&self) -> &PrimaryConsensusProof {
        &self.primary_proof
    }

    pub fn epoch(&self) -> u64 {
        self.primary_proof.epoch()
    }

    /// Verify the ordered proxy blocks are well-formed and properly linked.
    pub fn verify(&self, proxy_verifier: &ValidatorVerifier) -> Result<()> {
        ensure!(
            !self.proxy_blocks.is_empty(),
            "OrderedProxyBlocksMsg must contain at least one proxy block"
        );

        // Verify all blocks are proxy blocks with non-decreasing last_primary_proof_round
        let mut prev_lppr = 0;
        for block in &self.proxy_blocks {
            ensure!(
                block.block_data().is_proxy_block(),
                "OrderedProxyBlocksMsg contains non-proxy block"
            );
            let lppr = block.block_data().last_primary_proof_round().unwrap_or(0);
            ensure!(
                lppr >= prev_lppr,
                "last_primary_proof_round must be non-decreasing: {} < {}",
                lppr,
                prev_lppr,
            );
            prev_lppr = lppr;
        }

        // Verify blocks are linked by parent hashes (chain structure)
        for i in 1..self.proxy_blocks.len() {
            ensure!(
                self.proxy_blocks[i].parent_id() == self.proxy_blocks[i - 1].id(),
                "Proxy blocks are not properly linked: block {} parent {} != block {} id {}",
                i,
                self.proxy_blocks[i].parent_id(),
                i - 1,
                self.proxy_blocks[i - 1].id(),
            );
        }

        // Verify the last block has the primary proof attached
        let last_block = self.proxy_blocks.last().unwrap();
        ensure!(
            last_block.block_data().primary_proof().is_some(),
            "Last proxy block must have primary proof attached"
        );
        ensure!(
            last_block.block_data().primary_proof().unwrap().proof_round()
                == self.primary_proof.proof_round(),
            "Last proxy block's primary proof round {} doesn't match message's primary proof round {}",
            last_block.block_data().primary_proof().unwrap().proof_round(),
            self.primary_proof.proof_round(),
        );

        // Verify block signatures
        for block in &self.proxy_blocks {
            block
                .validate_signature(proxy_verifier)
                .context("Failed to verify proxy block signature")?;
        }

        Ok(())
    }
}

impl Display for OrderedProxyBlocksMsg {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "OrderedProxyBlocksMsg: [primary_round: {}, num_blocks: {}, proof: {}]",
            self.primary_round(),
            self.proxy_blocks.len(),
            self.primary_proof,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ordered_proxy_blocks_msg_creation() {
        use crate::vote_data::VoteData;
        use aptos_crypto::HashValue;
        use aptos_types::{
            aggregate_signature::AggregateSignature,
            block_info::BlockInfo,
            ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
        };

        let block_info = BlockInfo::new(1, 0, HashValue::random(), HashValue::random(), 0, 0, None);
        let vote_data = VoteData::new(block_info.clone(), block_info.clone());
        let ledger_info = LedgerInfo::new(block_info, HashValue::zero());
        let li_sig = LedgerInfoWithSignatures::new(ledger_info, AggregateSignature::empty());
        let qc = QuorumCert::new(vote_data, li_sig);

        let msg = OrderedProxyBlocksMsg::new(vec![], PrimaryConsensusProof::QC(qc));
        assert_eq!(msg.primary_round(), 1); // proof.round=0, so primary_round=1
        assert!(msg.proxy_blocks().is_empty());
    }
}
