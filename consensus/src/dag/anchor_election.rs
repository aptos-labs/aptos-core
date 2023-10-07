// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::common::{Author, Round};

pub trait AnchorElection: Send + Sync {
    fn get_anchor(&self, round: Round) -> Author;

    fn update_reputation(
        &mut self,
        round: Round,
        author: &Author,
        parents: Vec<Author>,
        failed_authors: Vec<Author>,
    );
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

    fn update_reputation(
        &mut self,
        _round: Round,
        _author: &Author,
        _parents: Vec<Author>,
        _failed_authors: Vec<Author>,
    ) {
    }
}
