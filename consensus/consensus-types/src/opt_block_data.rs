// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{Author, Payload, Round},
    proposal_ext::OptProposalBody,
    quorum_cert::QuorumCert,
};
use anyhow::{ensure, Result};
use aptos_crypto::HashValue;
use aptos_crypto_derive::CryptoHasher;
use aptos_infallible::duration_since_epoch;
use aptos_types::{block_info::BlockInfo, validator_txn::ValidatorTransaction};
use mirai_annotations::debug_checked_verify_eq;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display, Formatter},
    ops::Deref,
};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, CryptoHasher)]
/// Same as BlockData, without QC and with parent id
pub struct OptBlockData {
    pub epoch: u64,
    pub round: Round,
    pub timestamp_usecs: u64,
    pub parent: BlockInfo,
    pub proposal_body: OptProposalBody,
}

impl OptBlockData {
    pub fn new(
        validator_txns: Vec<ValidatorTransaction>,
        payload: Payload,
        author: Author,
        epoch: u64,
        round: Round,
        timestamp_usecs: u64,
        parent: BlockInfo,
        grandparent_qc: QuorumCert,
    ) -> Self {
        Self {
            epoch,
            round,
            timestamp_usecs,
            parent,
            proposal_body: OptProposalBody::V0 {
                validator_txns,
                payload,
                author,
                grandparent_qc,
            },
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn parent_id(&self) -> HashValue {
        self.parent.id()
    }

    pub fn parent(&self) -> &BlockInfo {
        &self.parent
    }

    pub fn timestamp_usecs(&self) -> u64 {
        self.timestamp_usecs
    }

    pub fn round(&self) -> Round {
        self.round
    }

    pub fn verify_well_formed(&self) -> anyhow::Result<()> {
        let parent = self.parent();
        let grandparent_qc = self.grandparent_qc().certified_block();
        ensure!(
            grandparent_qc.round() + 1 == parent.round(),
            "Block's parent's round {} must be one more than grandparent's round {}",
            parent.round(),
            grandparent_qc.round(),
        );
        ensure!(
            parent.round() + 1 == self.round(),
            "Block's round {} must be one more than parent's round {}",
            self.round(),
            parent.round(),
        );
        ensure!(
            grandparent_qc.epoch() == self.epoch() && parent.epoch() == self.epoch(),
            "Block's parent and grantparent should be in the same epoch"
        );
        ensure!(
            !grandparent_qc.has_reconfiguration(),
            "Optimistic proposals are disallowed after the reconfiguration block"
        );

        self.payload().verify_epoch(self.epoch())?;

        ensure!(
            self.timestamp_usecs() > parent.timestamp_usecs()
                && parent.timestamp_usecs() > grandparent_qc.timestamp_usecs(),
            "Blocks must have strictly increasing timestamps"
        );

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
    type Target = OptProposalBody;

    fn deref(&self) -> &Self::Target {
        &self.proposal_body
    }
}

impl Display for OptBlockData {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "[author: {}, epoch: {}, round: {:02}, parent_id: {}, timestamp: {}]",
            self.author(),
            self.epoch(),
            self.round(),
            self.parent_id(),
            self.timestamp_usecs(),
        )
    }
}
