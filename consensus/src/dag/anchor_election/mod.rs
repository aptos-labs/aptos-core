// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{dag::storage::CommitEvent, liveness::leader_reputation::VotingPowerRatio};
use velor_consensus_types::common::{Author, Round};

pub trait AnchorElection: Send + Sync {
    fn get_anchor(&self, round: Round) -> Author;

    fn update_reputation(&self, commit_event: CommitEvent);
}

pub trait CommitHistory: Send + Sync {
    fn get_voting_power_participation_ratio(&self, round: Round) -> VotingPowerRatio;
}

mod leader_reputation_adapter;
mod round_robin;

pub use leader_reputation_adapter::{LeaderReputationAdapter, MetadataBackendAdapter};
pub use round_robin::RoundRobinAnchorElection;
