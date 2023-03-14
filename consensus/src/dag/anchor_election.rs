// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::common::Round;
use aptos_types::{validator_verifier::ValidatorVerifier, PeerId};
use std::collections::HashMap;

pub trait AnchorElection: Send + Sync {
    fn get_next_anchor(&self, round: Round) -> PeerId;
}

pub struct RoundRobinAnchorElection {
    num_of_validators: usize,
    index_to_peer_id: HashMap<usize, PeerId>,
}

impl RoundRobinAnchorElection {
    pub fn new(verifier: &ValidatorVerifier) -> Self {
        let index_to_peer_id: HashMap<usize, PeerId> = verifier
            .address_to_validator_index()
            .iter()
            .map(|(peer_id, index)| (*index, *peer_id))
            .collect();

        Self {
            num_of_validators: index_to_peer_id.len(),
            index_to_peer_id,
        }
    }
}

impl AnchorElection for RoundRobinAnchorElection {
    fn get_next_anchor(&self, round: Round) -> PeerId {
        self.index_to_peer_id
            .get(&(round as usize % self.num_of_validators))
            .unwrap()
            .clone()
    }
}
