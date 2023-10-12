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

mod round_robin;

pub use round_robin::RoundRobinAnchorElection;
