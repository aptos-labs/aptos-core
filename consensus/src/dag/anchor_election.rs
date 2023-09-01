// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::common::{Author, Round};

pub trait AnchorElection: Send {
    fn get_anchor(&self, round: Round) -> Author;

    fn commit(&mut self, round: Round);
}

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

    fn commit(&mut self, _round: Round) {}
}
