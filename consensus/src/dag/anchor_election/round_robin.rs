// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::CommitHistory;
use crate::{
    dag::{anchor_election::AnchorElection, storage::CommitEvent},
    liveness::leader_reputation::VotingPowerRatio,
};
use aptos_consensus_types::common::{Author, Round};

pub struct RoundRobinAnchorElection {
    validators: Vec<Author>,
}

impl RoundRobinAnchorElection {
    pub fn new(validators: Vec<Author>) -> Self {
        Self { validators }
    }
}

impl AnchorElection for RoundRobinAnchorElection {
    fn get_anchor(&self, round: Round) -> Author {
        self.validators[(round / 2) as usize % self.validators.len()]
    }

    fn update_reputation(&self, _event: CommitEvent) {}
}

impl CommitHistory for RoundRobinAnchorElection {
    fn get_voting_power_participation_ratio(&self, _round: Round) -> VotingPowerRatio {
        1.0
    }
}
