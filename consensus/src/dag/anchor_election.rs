// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::common::Round;
use aptos_fallible::copy_from_slice::copy_slice_to_vec;
use aptos_infallible::Mutex;
use aptos_types::{validator_verifier::ValidatorVerifier, PeerId};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap},
};

pub trait AnchorElection: Send + Sync {
    fn get_round_anchor_peer_id(&self, round: u64) -> PeerId;

    fn record_anchor(&self, failed: Vec<PeerId>, success: PeerId);
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
    fn get_round_anchor_peer_id(&self, round: u64) -> PeerId {
        self.index_to_peer_id
            .get(&((round / 2) as usize % self.num_of_validators))
            .unwrap()
            .clone()
    }

    fn record_anchor(&self, _failed: Vec<PeerId>, _success: PeerId) {
        ()
    }
}

pub struct LeaderReputationElection {
    reputation: Mutex<Vec<u128>>,
    index: HashMap<PeerId, usize>,
    reverse_index: Vec<PeerId>,
}

fn next_in_range(state: Vec<u8>, max: u128) -> u128 {
    // hash = SHA-3-256(state)
    let hash = aptos_crypto::HashValue::sha3_256_of(&state).to_vec();
    let mut temp = [0u8; 16];
    copy_slice_to_vec(&hash[..16], &mut temp).expect("next failed");
    // return hash[0..16]
    u128::from_le_bytes(temp) % max
}

pub(crate) fn choose_index(mut weights: Vec<u128>, state: Vec<u8>) -> usize {
    let mut total_weight: u128 = 0;
    // Create cumulative weights vector
    // Since we own the vector, we can safely modify it in place
    for w in &mut weights {
        total_weight = total_weight
            .checked_add(*w)
            .expect("Total stake shouldn't exceed u128::MAX");
        *w = total_weight;
    }
    let chosen_weight = next_in_range(state, total_weight);
    weights
        .binary_search_by(|w| {
            if *w <= chosen_weight {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        })
        .unwrap_err()
}

impl LeaderReputationElection {
    pub fn new(verifier: &ValidatorVerifier) -> Self {
        let index: HashMap<PeerId, usize> = verifier.address_to_validator_index().clone();
        let reverse_index = verifier.get_ordered_account_addresses_iter().collect();
        let reputation = Mutex::new(vec![100; index.len()]);
        Self {
            reputation,
            index,
            reverse_index,
        }
    }
}

impl AnchorElection for LeaderReputationElection {
    fn get_round_anchor_peer_id(&self, round: u64) -> PeerId {
        let lock = self.reputation.lock();
        let weight = lock.clone();
        let chosen_index = choose_index(weight, round.to_le_bytes().to_vec());
        self.reverse_index[chosen_index]
    }

    fn record_anchor(&self, failed: Vec<PeerId>, success: PeerId) {
        let mut write = self.reputation.lock();
        for peer in failed {
            write[self.index[&peer]] = 1;
        }
        write[self.index[&success]] = 100;
    }
}
