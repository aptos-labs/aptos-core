// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::cmp::Ordering;

use consensus_types::common::{Author, Round};
use fallible::copy_from_slice::copy_slice_to_vec;
use num_traits::CheckedAdd;

/// ProposerElection incorporates the logic of choosing a leader among multiple candidates.
pub trait ProposerElection {
    /// If a given author is a valid candidate for being a proposer, generate the info,
    /// otherwise return None.
    /// Note that this function is synchronous.
    fn is_valid_proposer(&self, author: Author, round: Round) -> bool {
        self.get_valid_proposer(round) == author
    }

    /// Return the valid proposer for a given round (this information can be
    /// used by e.g., voters for choosing the destinations for sending their votes to).
    fn get_valid_proposer(&self, round: Round) -> Author;
}

// next consumes seed and returns random deterministic u64 value in [0, max) range
fn next_in_range(state: Vec<u8>, max: u128) -> u128 {
    // hash = SHA-3-256(state)
    let hash = aptos_crypto::HashValue::sha3_256_of(&state).to_vec();
    let mut temp = [0u8; 16];
    copy_slice_to_vec(&hash[..16], &mut temp).expect("next failed");
    // return hash[0..16]
    u128::from_le_bytes(temp) % max
}

// chose index randomly, with given weight distribution
pub(crate) fn choose_index(mut weights: Vec<u128>, state: Vec<u8>) -> usize {
    let mut total_weight = 0;
    // Create cumulative weights vector
    // Since we own the vector, we can safely modify it in place
    for w in &mut weights {
        total_weight = total_weight
            .checked_add(w)
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

#[test]
fn test_bounds() {
    // check that bounds are correct, and both first and last weight can be selected.
    let mut selected = [0, 0];
    let weights = [u64::MAX as u128 * 1000, u64::MAX as u128 * 1000].to_vec();
    // 10 is enough to get one of each.
    for i in 0..10 {
        selected[choose_index(weights.clone(), (i as i32).to_le_bytes().to_vec())] += 1;
    }

    assert!(selected[0] >= 1);
    assert!(selected[1] >= 1);
}
