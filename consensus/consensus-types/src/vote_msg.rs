// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    sync_info::{SyncInfo, VersionedSyncInfo},
    vote::Vote,
};
use anyhow::ensure;
use aptos_crypto::HashValue;
use aptos_types::validator_verifier::ValidatorVerifier;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// VoteMsg is the struct that is ultimately sent by the voter in response for
/// receiving a proposal.
/// VoteMsg carries the `LedgerInfo` of a block that is going to be committed in case this vote
/// is gathers QuorumCertificate (see the detailed explanation in the comments of `LedgerInfo`).
/// This struct is versioned to support upgrades.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum VersionedVoteMsg {
    V1(VoteMsg),
    V2(VoteMsgV2),
}

impl VersionedVoteMsg {
    pub fn vote(&self) -> &Vote {
        match self {
            VersionedVoteMsg::V1(msg) => msg.vote(),
            VersionedVoteMsg::V2(msg) => msg.vote(),
        }
    }

    pub fn sync_info(&self) -> VersionedSyncInfo {
        match self {
            VersionedVoteMsg::V1(msg) => VersionedSyncInfo::new_v1(msg.sync_info().clone()),
            VersionedVoteMsg::V2(msg) => msg.sync_info().clone(),
        }
    }

    pub fn epoch(&self) -> u64 {
        match self {
            VersionedVoteMsg::V1(msg) => msg.epoch(),
            VersionedVoteMsg::V2(msg) => msg.epoch(),
        }
    }

    pub fn proposed_block_id(&self) -> HashValue {
        match self {
            VersionedVoteMsg::V1(msg) => msg.proposed_block_id(),
            VersionedVoteMsg::V2(msg) => msg.proposed_block_id(),
        }
    }

    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        match self {
            VersionedVoteMsg::V1(msg) => msg.verify(validator),
            VersionedVoteMsg::V2(msg) => msg.verify(validator),
        }
    }
}

/// VoteMsg is the struct that is ultimately sent by the voter in response for
/// receiving a proposal.
/// VoteMsg carries the `LedgerInfo` of a block that is going to be committed in case this vote
/// is gathers QuorumCertificate (see the detailed explanation in the comments of `LedgerInfo`).
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct VoteMsg {
    /// The container for the vote (VoteData, LedgerInfo, Signature)
    vote: Vote,
    /// Sync info carries information about highest QC, TC and LedgerInfo
    sync_info: SyncInfo,
}

impl Display for VoteMsg {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "VoteMsg: [{}]", self.vote,)
    }
}

impl VoteMsg {
    pub fn new(vote: Vote, sync_info: SyncInfo) -> Self {
        Self { vote, sync_info }
    }

    /// Container for actual voting material
    pub fn vote(&self) -> &Vote {
        &self.vote
    }

    /// SyncInfo of the given vote message
    pub fn sync_info(&self) -> &SyncInfo {
        &self.sync_info
    }

    pub fn epoch(&self) -> u64 {
        self.vote.epoch()
    }

    pub fn proposed_block_id(&self) -> HashValue {
        self.vote.vote_data().proposed().id()
    }

    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        ensure!(
            self.vote().epoch() == self.sync_info.epoch(),
            "VoteMsg has different epoch"
        );
        ensure!(
            self.vote().vote_data().proposed().round() > self.sync_info.highest_round(),
            "Vote Round should be higher than SyncInfo"
        );
        if let Some((timeout, _)) = self.vote().two_chain_timeout() {
            ensure!(
                timeout.hqc_round() <= self.sync_info.highest_certified_round(),
                "2-chain Timeout hqc should be less or equal than the sync info hqc"
            );
        }
        // We're not verifying SyncInfo here yet: we are going to verify it only in case we need
        // it. This way we avoid verifying O(n) SyncInfo messages while aggregating the votes
        // (O(n^2) signature verifications).
        self.vote().verify(validator)
    }
}

// TODO: There is a lot of duplicaition of code between VoteMsg and VoteMsgV2.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct VoteMsgV2 {
    vote: Vote,
    sync_info: VersionedSyncInfo,
}

impl VoteMsgV2 {
    pub fn new(vote: Vote, sync_info: VersionedSyncInfo) -> Self {
        Self { vote, sync_info }
    }

    pub fn epoch(&self) -> u64 {
        self.vote.epoch()
    }

    pub fn vote(&self) -> &Vote {
        &self.vote
    }

    pub fn sync_info(&self) -> &VersionedSyncInfo {
        &self.sync_info
    }

    pub fn proposed_block_id(&self) -> HashValue {
        self.vote.vote_data().proposed().id()
    }

    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        ensure!(
            self.vote().epoch() == self.sync_info.epoch(),
            "VoteMsg has different epoch"
        );
        ensure!(
            self.vote().vote_data().proposed().round() > self.sync_info.highest_round(),
            "Vote Round should be higher than SyncInfo"
        );
        if let Some((timeout, _)) = self.vote().two_chain_timeout() {
            ensure!(
                timeout.hqc_round() <= self.sync_info.highest_certified_round(),
                "2-chain Timeout hqc should be less or equal than the sync info hqc"
            );
        }
        // We're not verifying SyncInfo here yet: we are going to verify it only in case we need
        // it. This way we avoid verifying O(n) SyncInfo messages while aggregating the votes
        // (O(n^2) signature verifications).
        self.vote().verify(validator)
    }
}
