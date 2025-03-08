// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::Author, opt_block_data::OptBlockData, proof_of_store::ProofCache, sync_info::SyncInfo,
};
use anyhow::{ensure, Result};
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

    pub fn verify(
        &self,
        sender: Author,
        validator: &ValidatorVerifier,
        proof_cache: &ProofCache,
        quorum_store_enabled: bool,
    ) -> Result<()> {
        ensure!(
            self.proposer() == sender,
            "Proposal author {:?} doesn't match sender {:?}",
            self.proposer(),
            sender
        );

        self.block_data().payload().map_or(Ok(()), |p| {
            p.verify(validator, proof_cache, quorum_store_enabled)
        })?;

        self.block_data()
            .grandparent_qc()
            .map_or(Ok(()), |qc| qc.verify(validator))?;

        // Optimistic proposal shouldn't have a timeout certificate
        ensure!(
            self.sync_info.highest_2chain_timeout_cert().is_none(),
            "Optimistic proposal shouldn't have a timeout certificate"
        );

        // Ensure the sync info has the grandparent QC
        ensure!(
            self.sync_info
                .highest_quorum_cert()
                .certified_block()
                .round()
                == self.block_data().round().saturating_sub(2),
            "Sync info doesn't have the grandparent QC"
        );

        // Note that we postpone the verification of SyncInfo until it's being used.
        self.block_data.verify()
    }
}
