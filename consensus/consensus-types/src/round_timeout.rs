// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{Author, Round},
    sync_info::SyncInfo,
    timeout_2chain::TwoChainTimeout,
};
use anyhow::{ensure, Context};
use velor_bitvec::BitVec;
use velor_crypto::bls12381;
use velor_short_hex_str::AsShortHexStr;
use velor_types::validator_verifier::ValidatorVerifier;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug)]
pub enum RoundTimeoutReason {
    Unknown,
    ProposalNotReceived,
    PayloadUnavailable { missing_authors: BitVec },
    NoQC,
}

impl std::fmt::Display for RoundTimeoutReason {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RoundTimeoutReason::Unknown => write!(f, "Unknown"),
            RoundTimeoutReason::ProposalNotReceived => write!(f, "ProposalNotReceived"),
            RoundTimeoutReason::PayloadUnavailable { .. } => {
                write!(f, "PayloadUnavailable",)
            },
            RoundTimeoutReason::NoQC => write!(f, "NoQC"),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct RoundTimeout {
    // The timeout
    timeout: TwoChainTimeout,
    author: Author,
    reason: RoundTimeoutReason,
    /// Signature on the Timeout
    signature: bls12381::Signature,
}

// this is required by structured log
impl std::fmt::Debug for RoundTimeout {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl std::fmt::Display for RoundTimeout {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "RoundTimeoutV2: [timeout: {}, author: {}, reason: {}]",
            self.timeout,
            self.author.short_str(),
            self.reason
        )
    }
}

impl RoundTimeout {
    pub fn new(
        timeout: TwoChainTimeout,
        author: Author,
        reason: RoundTimeoutReason,
        signature: bls12381::Signature,
    ) -> Self {
        Self {
            timeout,
            author,
            reason,
            signature,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.timeout.epoch()
    }

    pub fn round(&self) -> Round {
        self.timeout.round()
    }

    pub fn two_chain_timeout(&self) -> &TwoChainTimeout {
        &self.timeout
    }

    pub fn author(&self) -> Author {
        self.author
    }

    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        self.timeout.verify(validator)?;
        validator
            .verify(
                self.author(),
                &self.timeout.signing_format(),
                &self.signature,
            )
            .context("Failed to verify 2-chain timeout signature")?;
        Ok(())
    }

    pub fn reason(&self) -> &RoundTimeoutReason {
        &self.reason
    }

    pub fn signature(&self) -> &bls12381::Signature {
        &self.signature
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct RoundTimeoutMsg {
    /// The container for the vote (VoteData, LedgerInfo, Signature)
    round_timeout: RoundTimeout,
    /// Sync info carries information about highest QC, TC and LedgerInfo
    sync_info: SyncInfo,
}

impl std::fmt::Display for RoundTimeoutMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "RoundTimeoutV2Msg: [{}], SyncInfo: [{}]",
            self.round_timeout, self.sync_info
        )
    }
}

impl RoundTimeoutMsg {
    pub fn new(round_timeout: RoundTimeout, sync_info: SyncInfo) -> Self {
        Self {
            round_timeout,
            sync_info,
        }
    }

    /// SyncInfo of the given vote message
    pub fn sync_info(&self) -> &SyncInfo {
        &self.sync_info
    }

    pub fn epoch(&self) -> u64 {
        self.round_timeout.epoch()
    }

    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        ensure!(
            self.round_timeout.epoch() == self.sync_info.epoch(),
            "RoundTimeoutV2Msg has different epoch"
        );
        ensure!(
            self.round_timeout.round() > self.sync_info.highest_round(),
            "Timeout Round should be higher than SyncInfo"
        );
        ensure!(
            self.round_timeout.two_chain_timeout().hqc_round()
                <= self.sync_info.highest_certified_round(),
            "2-chain Timeout hqc should be less or equal than the sync info hqc"
        );
        // We're not verifying SyncInfo here yet: we are going to verify it only in case we need
        // it. This way we avoid verifying O(n) SyncInfo messages while aggregating the votes
        // (O(n^2) signature verifications).
        self.round_timeout.verify(validator)
    }

    pub fn round(&self) -> u64 {
        self.round_timeout.round()
    }

    pub fn author(&self) -> Author {
        self.round_timeout.author()
    }

    pub fn timeout(&self) -> RoundTimeout {
        self.round_timeout.clone()
    }
}
