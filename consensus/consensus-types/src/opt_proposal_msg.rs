// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::Author, opt_block_data::OptBlockData, proof_of_store::ProofCache, sync_info::SyncInfo,
};
use anyhow::{ensure, Context, Result};
use aptos_types::validator_verifier::ValidatorVerifier;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OptProposalMsg {
    block_data: OptBlockData,
    sync_info: SyncInfo,
}

impl OptProposalMsg {
    pub fn new(block_data: OptBlockData, sync_info: SyncInfo) -> Self {
        Self {
            block_data,
            sync_info,
        }
    }

    pub fn block_data(&self) -> &OptBlockData {
        &self.block_data
    }

    pub fn take_block_data(self) -> OptBlockData {
        self.block_data
    }

    pub fn epoch(&self) -> u64 {
        self.block_data.epoch()
    }

    pub fn round(&self) -> u64 {
        self.block_data.round()
    }

    pub fn timestamp_usecs(&self) -> u64 {
        self.block_data.timestamp_usecs()
    }

    pub fn proposer(&self) -> Author {
        *self.block_data.author()
    }

    pub fn sync_info(&self) -> &SyncInfo {
        &self.sync_info
    }

    /// Verifies that the ProposalMsg is well-formed.
    pub fn verify_well_formed(&self) -> Result<()> {
        self.block_data
            .verify_well_formed()
            .context("Fail to verify OptProposalMsg's data")?;
        ensure!(
            self.block_data.round() > 1,
            "Proposal for {} has round <= 1",
            self.block_data,
        );
        ensure!(
            self.block_data.epoch() == self.sync_info.epoch(),
            "ProposalMsg has different epoch number from SyncInfo"
        );
        // Ensure the sync info has the grandparent QC
        ensure!(
            self.block_data.grandparent_qc().certified_block().id()
                == self.sync_info.highest_quorum_cert().certified_block().id(),
            "Proposal HQC in SyncInfo certifies {}, but block grandparent id is {}",
            self.sync_info.highest_quorum_cert().certified_block().id(),
            self.block_data.grandparent_qc().certified_block().id(),
        );
        let grandparent_round = self
            .block_data
            .round()
            .checked_sub(2)
            .ok_or_else(|| anyhow::anyhow!("proposal round overflowed!"))?;

        let highest_certified_round = self.block_data.grandparent_qc().certified_block().round();
        ensure!(
            grandparent_round == highest_certified_round,
            "Proposal {} does not have a certified round {}",
            self.block_data,
            grandparent_round
        );
        // Optimistic proposal shouldn't have a timeout certificate
        ensure!(
            self.sync_info.highest_2chain_timeout_cert().is_none(),
            "Optimistic proposal shouldn't have a timeout certificate"
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
            self.proposer() == sender,
            "OptProposal author {:?} doesn't match sender {:?}",
            self.proposer(),
            sender
        );

        self.block_data()
            .payload()
            .verify(validator, proof_cache, quorum_store_enabled)?;

        self.block_data().grandparent_qc().verify(validator)?;

        // Note that we postpone the verification of SyncInfo until it's being used.
        self.block_data.verify_well_formed()
    }
}
