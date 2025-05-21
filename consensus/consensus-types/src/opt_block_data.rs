// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{Author, Payload, Round},
    proposal_ext::OptProposalExt,
    quorum_cert::QuorumCert,
};
use anyhow::{ensure, Result};
use aptos_crypto::HashValue;
use aptos_crypto_derive::CryptoHasher;
use aptos_infallible::duration_since_epoch;
use aptos_types::{block_info::BlockInfo, validator_txn::ValidatorTransaction};
use serde::{Deserialize, Serialize};
use std::ops::Deref;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, CryptoHasher)]
/// Same as BlockData, without QC and with parent id
pub struct OptBlockData {
    pub epoch: u64,
    pub round: Round,
    pub timestamp_usecs: u64,
    pub parent_id: HashValue,
    pub proposal: OptProposalExt,
}

impl OptBlockData {
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn parent_id(&self) -> HashValue {
        self.parent_id
    }

    pub fn timestamp_usecs(&self) -> u64 {
        self.timestamp_usecs
    }

    pub fn round(&self) -> Round {
        self.round
    }

    pub fn new(
        validator_txns: Vec<ValidatorTransaction>,
        payload: Payload,
        author: Author,
        failed_authors: Vec<(Round, Author)>,
        epoch: u64,
        round: Round,
        timestamp_usecs: u64,
        parent_id: HashValue,
        grandparent_qc: QuorumCert,
    ) -> Self {
        Self {
            epoch,
            round,
            timestamp_usecs,
            parent_id,
            proposal: OptProposalExt::V0 {
                validator_txns,
                payload,
                author,
                failed_authors,
                grandparent_qc,
            },
        }
    }

    pub fn verify(&self) -> Result<()> {
        if let Some(payload) = self.payload() {
            payload.verify_epoch(self.epoch())?;
        }

        let current_ts = duration_since_epoch();
        // we can say that too far is 5 minutes in the future
        const TIMEBOUND: u64 = 300_000_000;
        ensure!(
            self.timestamp_usecs() <= (current_ts.as_micros() as u64).saturating_add(TIMEBOUND),
            "Blocks must not be too far in the future"
        );

        Ok(())
    }
}

impl Deref for OptBlockData {
    type Target = OptProposalExt;

    fn deref(&self) -> &Self::Target {
        &self.proposal
    }
}
