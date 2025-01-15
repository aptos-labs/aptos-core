// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{block::Block, common::Author, proof_of_store::ProofCache, sync_info::SyncInfo};
use anyhow::{anyhow, ensure, format_err, Context, Result};
use aptos_short_hex_str::AsShortHexStr;
use aptos_types::validator_verifier::ValidatorVerifier;
use serde::{Deserialize, Serialize};
use std::fmt;

/// ProposalMsg contains the required information for the proposer election protocol to make its
/// choice (typically depends on round and proposer info).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProposalMsg {
    proposal: Block,
    sync_info: SyncInfo,
}

impl ProposalMsg {
    /// Creates a new proposal.
    pub fn new(proposal: Block, sync_info: SyncInfo) -> Self {
        Self {
            proposal,
            sync_info,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.proposal.epoch()
    }

    /// Verifies that the ProposalMsg is well-formed.
    pub fn verify_well_formed(&self) -> Result<()> {
        ensure!(
            !self.proposal.is_nil_block(),
            "Proposal {} for a NIL block",
            self.proposal
        );
        self.proposal
            .verify_well_formed()
            .context("Fail to verify ProposalMsg's block")?;
        ensure!(
            self.proposal.round() > 0,
            "Proposal for {} has an incorrect round of 0",
            self.proposal,
        );
        ensure!(
            self.proposal.epoch() == self.sync_info.epoch(),
            "ProposalMsg has different epoch number from SyncInfo"
        );
        ensure!(
            self.proposal.parent_id()
                == self.sync_info.highest_quorum_cert().certified_block().id(),
            "Proposal HQC in SyncInfo certifies {}, but block parent id is {}",
            self.sync_info.highest_quorum_cert().certified_block().id(),
            self.proposal.parent_id(),
        );
        let previous_round = self
            .proposal
            .round()
            .checked_sub(1)
            .ok_or_else(|| anyhow!("proposal round overflowed!"))?;

        let highest_certified_round = std::cmp::max(
            self.proposal.quorum_cert().certified_block().round(),
            self.sync_info.highest_timeout_round(),
        );
        ensure!(
            previous_round == highest_certified_round,
            "Proposal {} does not have a certified round {}",
            self.proposal,
            previous_round
        );
        ensure!(
            self.proposal.author().is_some(),
            "Proposal {} does not define an author",
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
                "Proposal author {:?} doesn't match sender {:?}",
                proposal_author,
                sender
            );
        }
        self.proposal().payload().map_or(Ok(()), |p| {
            p.verify(validator, proof_cache, quorum_store_enabled)
        })?;

        self.proposal()
            .validate_signature(validator)
            .map_err(|e| format_err!("{:?}", e))?;
        // if there is a timeout certificate, verify its signatures
        if let Some(tc) = self.sync_info.highest_2chain_timeout_cert() {
            tc.verify(validator).map_err(|e| format_err!("{:?}", e))?;
        }
        // Note that we postpone the verification of SyncInfo until it's being used.
        self.verify_well_formed()
    }

    pub fn proposal(&self) -> &Block {
        &self.proposal
    }

    pub fn take_proposal(self) -> Block {
        self.proposal
    }

    pub fn sync_info(&self) -> &SyncInfo {
        &self.sync_info
    }

    pub fn proposer(&self) -> Author {
        self.proposal
            .author()
            .expect("Proposal should be verified having an author")
    }
}

impl fmt::Display for ProposalMsg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[proposal {} from ", self.proposal)?;
        match self.proposal.author() {
            Some(author) => write!(f, "{}]", author.short_str()),
            None => write!(f, "NIL]"),
        }
    }
}
