// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;
use crate::dag::storage::CommitEvent;
use aptos_consensus_types::common::{Author, Round};

pub trait AnchorElection: Send + Sync {
    fn get_anchor(&self, round: Round) -> Author;

    fn update_reputation(&self, commit_event: CommitEvent);
}

pub trait TChainHealthBackoff: Send + Sync {
    fn get_round_backoff(&self, round: Round) -> Option<Duration>;

    fn get_round_payload_limits(&self, round: Round) -> Option<(u64, u64)>;
}

mod leader_reputation_adapter;
mod round_robin;

pub use leader_reputation_adapter::{LeaderReputationAdapter, MetadataBackendAdapter};
pub use round_robin::RoundRobinAnchorElection;
