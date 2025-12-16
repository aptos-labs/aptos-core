// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::liveness::proposer_election::ProposerElection;
use aptos_consensus_types::common::{Author, Round};
use std::collections::HashMap;

/// The round proposer maps a round to author
pub struct RoundProposer {
    // A pre-defined map specifying proposers per round
    proposers: HashMap<Round, Author>,
    // Default proposer to use if proposer for a round is unspecified.
    // We hardcode this to the first proposer
    default_proposer: Author,
}

impl RoundProposer {
    pub fn new(proposers: HashMap<Round, Author>, default_proposer: Author) -> Self {
        Self {
            proposers,
            default_proposer,
        }
    }
}

impl ProposerElection for RoundProposer {
    fn get_valid_proposer(&self, round: Round) -> Author {
        match self.proposers.get(&round) {
            None => self.default_proposer,
            Some(round_proposer) => *round_proposer,
        }
    }
}
