// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! QC Certification functions for Prefix Consensus
//!
//! This module implements the core certification functions from the paper:
//! - QC1Certify: Extract longest prefix with >1/3 stake from QC1
//! - QC2Certify: Compute maximum common prefix from QC2
//! - QC3Certify: Compute both v_low (mcp) and v_high (mce) from QC3

use crate::types::{Element, PrefixVector, QC1, QC2, QC3};
use crate::utils::{max_common_prefix, min_common_extension};
use aptos_types::validator_verifier::ValidatorVerifier;
use std::collections::HashMap;

// ============================================================================
// Trie Implementation for QC1Certify
// ============================================================================

/// A trie node for efficient prefix computation with stake-weighted voting
#[derive(Debug)]
struct TrieNode {
    /// Children of this node, keyed by element
    children: HashMap<Element, TrieNode>,

    /// Cumulative stake of validators whose vectors pass through this node
    stake: u128,
}

impl TrieNode {
    fn new() -> Self {
        Self {
            children: HashMap::new(),
            stake: 0,
        }
    }

    /// Insert a vector into the trie with the given voting power
    fn insert(&mut self, vector: &[Element], voting_power: u64) {
        self.stake += voting_power as u128;

        if vector.is_empty() {
            return;
        }

        let first = vector[0];
        let rest = &vector[1..];

        self.children
            .entry(first)
            .or_insert_with(TrieNode::new)
            .insert(rest, voting_power);
    }

    /// Find the longest prefix that has at least `stake_threshold` stake
    fn find_longest_prefix_with_threshold(
        &self,
        stake_threshold: u128,
        current_prefix: &mut Vec<Element>,
        best_prefix: &mut Vec<Element>,
    ) {
        // If this node doesn't meet the threshold, stop
        if self.stake < stake_threshold {
            return;
        }

        // Update best prefix if current is longer
        if current_prefix.len() > best_prefix.len() {
            *best_prefix = current_prefix.clone();
        }

        // Try to extend the prefix through children
        for (element, child) in &self.children {
            current_prefix.push(*element);
            child.find_longest_prefix_with_threshold(stake_threshold, current_prefix, best_prefix);
            current_prefix.pop();
        }
    }
}

/// Build a trie from votes with stake-weighted insertion
fn build_trie_with_stakes(qc1: &QC1, verifier: &ValidatorVerifier) -> TrieNode {
    let mut root = TrieNode::new();
    for vote in &qc1.votes {
        let voting_power = verifier
            .get_voting_power(&vote.author)
            .unwrap_or(0);
        root.insert(&vote.input_vector, voting_power);
    }
    root
}

// ============================================================================
// QC Certification Functions
// ============================================================================

/// QC1Certify: Extract the longest prefix with >1/3 stake agreement
///
/// From the paper (Algorithm 1, Round 1), adapted for proof-of-stake:
/// Find the longest prefix that is common to a subset of votes whose combined
/// stake exceeds 1/3 of total voting power (the minority threshold).
///
/// This ensures the certified prefix could have been seen by at least one honest
/// validator, since Byzantine validators control at most 1/3 of stake.
///
/// # Algorithm
///
/// Uses a trie data structure for O(total input size) complexity:
/// 1. Build trie from all vote vectors, tracking cumulative stake at each node
/// 2. Each node tracks total stake of validators whose vectors pass through it
/// 3. Find longest path where node.stake >= minority_threshold (>1/3)
///
/// # Parameters
///
/// - `qc1`: The QC1 containing votes from validators with >2/3 stake
/// - `verifier`: Validator verifier for looking up voting power
///
/// # Returns
///
/// The longest certified prefix (may be empty if no >1/3 stake agreement exists)
pub fn qc1_certify(qc1: &QC1, verifier: &ValidatorVerifier) -> PrefixVector {
    if qc1.votes.is_empty() {
        return Vec::new();
    }

    // Compute minority quorum threshold (>1/3 stake)
    // This is: total_voting_power - quorum_voting_power + 1
    // where quorum_voting_power = total * 2 / 3 + 1
    let total_voting_power = verifier.total_voting_power();
    let quorum_voting_power = verifier.quorum_voting_power();
    let minority_threshold = total_voting_power - quorum_voting_power + 1;

    // Build trie with stake tracking
    let trie = build_trie_with_stakes(qc1, verifier);

    // Find longest prefix with at least minority_threshold stake
    let mut current_prefix = Vec::new();
    let mut best_prefix = Vec::new();

    trie.find_longest_prefix_with_threshold(minority_threshold, &mut current_prefix, &mut best_prefix);

    best_prefix
}

/// QC2Certify: Compute the maximum common prefix of all certified prefixes in QC2
///
/// From the paper (Algorithm 1, Round 2):
/// "xp := mcp({x ∈ QC2})"
///
/// This computes the maximum common prefix of all the certified prefixes
/// that parties voted for in Round 2.
///
/// # Parameters
///
/// - `qc2`: The QC2 containing votes with >2/3 of total stake
///
/// # Returns
///
/// The maximum common prefix of all certified prefixes
pub fn qc2_certify(qc2: &QC2) -> PrefixVector {
    // Extract all certified prefixes from votes
    let prefixes: Vec<PrefixVector> = qc2
        .votes
        .iter()
        .map(|v| v.certified_prefix.clone())
        .collect();

    if prefixes.is_empty() {
        return Vec::new();
    }

    // Compute maximum common prefix
    max_common_prefix(&prefixes)
}

/// QC3Certify: Compute both v_low (mcp) and v_high (mce) from QC3
///
/// From the paper (Algorithm 1, Round 3):
/// "v_low := mcp({xp ∈ QC3})"
/// "v_high := mce({xp ∈ QC3})"
///
/// This computes the final outputs:
/// - v_low: Maximum common prefix (safe to commit)
/// - v_high: Minimum common extension (safe to extend)
///
/// # Parameters
///
/// - `qc3`: The QC3 containing votes with >2/3 of total stake
///
/// # Returns
///
/// A tuple (v_low, v_high)
///
/// # Panics
///
/// Panics if the prefixes in QC3 are not mutually consistent. By the Consistency Lemma,
/// all certified prefixes from Round 2 are guaranteed to be consistent (even with Byzantine
/// parties). If this invariant is violated, it indicates an implementation bug in the
/// certification logic, verification, or QC formation.
pub fn qc3_certify(qc3: &QC3) -> (PrefixVector, PrefixVector) {
    // Extract all mcp prefixes from votes
    let prefixes: Vec<PrefixVector> = qc3.votes.iter().map(|v| v.mcp_prefix.clone()).collect();

    if prefixes.is_empty() {
        return (Vec::new(), Vec::new());
    }

    // Compute v_low (maximum common prefix)
    let v_low = max_common_prefix(&prefixes);

    // Compute v_high (minimum common extension)
    // By the Consistency Lemma, all Round 2 certified prefixes must be consistent,
    // so min_common_extension should always return Some(_). If it returns None,
    // there is a bug in our implementation.
    let v_high = min_common_extension(&prefixes).expect(
        "IMPLEMENTATION BUG: QC3 prefixes are inconsistent, violating the Consistency Lemma. \
         This indicates a bug in qc2_certify, verification logic, or QC formation."
    );

    (v_low, v_high)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Brute force approach to find longest prefix with stake threshold
///
/// This is an alternative implementation for testing/validation.
/// It tries all subsets and finds the longest mcp where subset stake >= minority threshold.
#[cfg(test)]
fn qc1_certify_brute_force(qc1: &QC1, verifier: &ValidatorVerifier) -> PrefixVector {
    if qc1.votes.is_empty() {
        return Vec::new();
    }

    // Compute minority threshold
    let total_voting_power = verifier.total_voting_power();
    let quorum_voting_power = verifier.quorum_voting_power();
    let minority_threshold = total_voting_power - quorum_voting_power + 1;

    let mut best_prefix = Vec::new();

    // Try all non-empty subsets
    let n = qc1.votes.len();
    for mask in 1..(1u64 << n) {
        // Compute stake and vectors for this subset
        let mut subset_stake: u128 = 0;
        let mut subset_vectors = Vec::new();

        for i in 0..n {
            if mask & (1 << i) != 0 {
                let vote = &qc1.votes[i];
                let stake = verifier.get_voting_power(&vote.author).unwrap_or(0) as u128;
                subset_stake += stake;
                subset_vectors.push(vote.input_vector.clone());
            }
        }

        // Check if this subset meets the stake threshold
        if subset_stake >= minority_threshold {
            let mcp = max_common_prefix(&subset_vectors);
            if mcp.len() > best_prefix.len() {
                best_prefix = mcp;
            }
        }
    }

    best_prefix
}

/// Generate all combinations of size k from a slice
#[cfg(test)]
fn combinations<T: Clone>(items: &[T], k: usize) -> Vec<Vec<T>> {
    if k == 0 {
        return vec![vec![]];
    }
    if items.is_empty() {
        return vec![];
    }

    let mut result = Vec::new();

    // Include first item
    let first = items[0].clone();
    let rest = &items[1..];
    for mut combo in combinations(rest, k - 1) {
        combo.insert(0, first.clone());
        result.push(combo);
    }

    // Exclude first item
    result.extend(combinations(rest, k));

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PartyId, Vote1, Vote2, Vote3};
    use aptos_crypto::HashValue;
    use aptos_types::validator_verifier::ValidatorConsensusInfo;

    fn hash(i: u64) -> HashValue {
        HashValue::sha3_256_of(&i.to_le_bytes())
    }

    fn dummy_party_id(i: u8) -> PartyId {
        PartyId::new([i; 32])
    }

    fn dummy_signature() -> aptos_crypto::bls12381::Signature {
        aptos_crypto::bls12381::Signature::dummy_signature()
    }

    /// Create a test validator verifier with equal voting power for each validator
    fn create_test_verifier(count: usize) -> ValidatorVerifier {
        let validator_infos: Vec<_> = (0..count)
            .map(|i| {
                let signer = aptos_types::validator_signer::ValidatorSigner::random(None);
                ValidatorConsensusInfo::new(dummy_party_id(i as u8), signer.public_key(), 1)
            })
            .collect();
        ValidatorVerifier::new(validator_infos)
    }

    #[test]
    fn test_trie_insert_and_stake() {
        let mut trie = TrieNode::new();

        let v1 = vec![hash(1), hash(2), hash(3)];
        let v2 = vec![hash(1), hash(2), hash(4)];
        let v3 = vec![hash(1), hash(5)];

        // Each vote has weight 1
        trie.insert(&v1, 1);
        trie.insert(&v2, 1);
        trie.insert(&v3, 1);

        // Root should have stake 3
        assert_eq!(trie.stake, 3);

        // First level should have 1 child (hash(1)) with stake 3
        assert_eq!(trie.children.len(), 1);
        let child1 = trie.children.get(&hash(1)).unwrap();
        assert_eq!(child1.stake, 3);
    }

    #[test]
    fn test_qc1_certify_empty() {
        let verifier = create_test_verifier(4);
        let qc1 = QC1::new(vec![]);
        let result = qc1_certify(&qc1, &verifier);
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_qc1_certify_simple() {
        // 4 validators with equal weight (1 each)
        // Total = 4, quorum = 3, minority threshold = 4 - 3 + 1 = 2
        let verifier = create_test_verifier(4);

        let votes = vec![
            Vote1::new(
                dummy_party_id(0),
                vec![hash(1), hash(2), hash(3)],
                0, 0, 1,
                dummy_signature(),
            ),
            Vote1::new(
                dummy_party_id(1),
                vec![hash(1), hash(2), hash(4)],
                0, 0, 1,
                dummy_signature(),
            ),
            Vote1::new(dummy_party_id(2), vec![hash(1), hash(5)], 0, 0, 1, dummy_signature()),
            Vote1::new(dummy_party_id(3), vec![hash(2), hash(3)], 0, 0, 1, dummy_signature()),
        ];

        let qc1 = QC1::new(votes);
        let result = qc1_certify(&qc1, &verifier);

        // Validators 0, 1, 2 all start with hash(1) (stake 3 >= 2)
        // Validators 0, 1 share [hash(1), hash(2)] (stake 2 >= 2)
        assert_eq!(result, vec![hash(1), hash(2)]);
    }

    #[test]
    fn test_qc1_certify_all_same() {
        // 3 validators with equal weight
        // Total = 3, quorum = 3, minority threshold = 3 - 3 + 1 = 1
        let verifier = create_test_verifier(3);

        let common_vec = vec![hash(1), hash(2), hash(3)];
        let votes = vec![
            Vote1::new(dummy_party_id(0), common_vec.clone(), 0, 0, 1, dummy_signature()),
            Vote1::new(dummy_party_id(1), common_vec.clone(), 0, 0, 1, dummy_signature()),
            Vote1::new(dummy_party_id(2), common_vec.clone(), 0, 0, 1, dummy_signature()),
        ];

        let qc1 = QC1::new(votes);
        let result = qc1_certify(&qc1, &verifier);

        // All votes identical, so entire vector should be certified
        assert_eq!(result, common_vec);
    }

    #[test]
    fn test_qc1_certify_vs_brute_force() {
        // 4 validators with equal weight
        let verifier = create_test_verifier(4);

        let votes = vec![
            Vote1::new(
                dummy_party_id(0),
                vec![hash(1), hash(2), hash(3)],
                0, 0, 1,
                dummy_signature(),
            ),
            Vote1::new(
                dummy_party_id(1),
                vec![hash(1), hash(2), hash(4)],
                0, 0, 1,
                dummy_signature(),
            ),
            Vote1::new(dummy_party_id(2), vec![hash(1), hash(5)], 0, 0, 1, dummy_signature()),
            Vote1::new(dummy_party_id(3), vec![hash(2), hash(3)], 0, 0, 1, dummy_signature()),
        ];

        let qc1 = QC1::new(votes);
        let trie_result = qc1_certify(&qc1, &verifier);
        let brute_result = qc1_certify_brute_force(&qc1, &verifier);

        assert_eq!(trie_result, brute_result);
    }

    #[test]
    fn test_qc2_certify() {
        let votes = vec![
            Vote2::new(
                dummy_party_id(0),
                vec![hash(1), hash(2), hash(3)],
                QC1::new(vec![]),
                0, 0, 1,
                dummy_signature(),
            ),
            Vote2::new(
                dummy_party_id(1),
                vec![hash(1), hash(2)],
                QC1::new(vec![]),
                0, 0, 1,
                dummy_signature(),
            ),
            Vote2::new(
                dummy_party_id(2),
                vec![hash(1), hash(2), hash(3), hash(4)],
                QC1::new(vec![]),
                0, 0, 1,
                dummy_signature(),
            ),
        ];

        let qc2 = QC2::new(votes);
        let result = qc2_certify(&qc2);

        // mcp of all should be [hash(1), hash(2)]
        assert_eq!(result, vec![hash(1), hash(2)]);
    }

    #[test]
    fn test_qc3_certify() {
        let votes = vec![
            Vote3::new(
                dummy_party_id(0),
                vec![hash(1), hash(2)],
                QC2::new(vec![]),
                0, 0, 1,
                dummy_signature(),
            ),
            Vote3::new(
                dummy_party_id(1),
                vec![hash(1), hash(2), hash(3)],
                QC2::new(vec![]),
                0, 0, 1,
                dummy_signature(),
            ),
            Vote3::new(
                dummy_party_id(2),
                vec![hash(1), hash(2), hash(3), hash(4)],
                QC2::new(vec![]),
                0, 0, 1,
                dummy_signature(),
            ),
        ];

        let qc3 = QC3::new(votes);
        let (v_low, v_high) = qc3_certify(&qc3);

        // v_low should be mcp = [hash(1), hash(2)]
        assert_eq!(v_low, vec![hash(1), hash(2)]);

        // v_high should be mce = longest = [hash(1), hash(2), hash(3), hash(4)]
        assert_eq!(v_high, vec![hash(1), hash(2), hash(3), hash(4)]);
    }

    #[test]
    fn test_combinations() {
        let items = vec![1, 2, 3, 4];
        let combos = combinations(&items, 2);

        assert_eq!(combos.len(), 6); // C(4,2) = 6
        assert!(combos.contains(&vec![1, 2]));
        assert!(combos.contains(&vec![1, 3]));
        assert!(combos.contains(&vec![1, 4]));
        assert!(combos.contains(&vec![2, 3]));
        assert!(combos.contains(&vec![2, 4]));
        assert!(combos.contains(&vec![3, 4]));
    }

    // ============================================================================
    // Edge Case Tests
    // ============================================================================

    #[test]
    fn test_qc1_certify_edge_case_all_empty_vectors() {
        // Use party_id 0, 1, 2 to match verifier
        let verifier = create_test_verifier(4);
        let votes = vec![
            Vote1::new(dummy_party_id(0), vec![], 0, 0, 1, dummy_signature()),
            Vote1::new(dummy_party_id(1), vec![], 0, 0, 1, dummy_signature()),
            Vote1::new(dummy_party_id(2), vec![], 0, 0, 1, dummy_signature()),
        ];
        let qc1 = QC1::new(votes);
        let result = qc1_certify(&qc1, &verifier);
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_qc1_certify_edge_case_all_different_prefixes() {
        // 4 validators, minority threshold = 2
        // All different prefixes, no subset with stake >= 2 has common prefix
        let verifier = create_test_verifier(4);
        let votes = vec![
            Vote1::new(dummy_party_id(0), vec![hash(1), hash(2)], 0, 0, 1, dummy_signature()),
            Vote1::new(dummy_party_id(1), vec![hash(10), hash(20)], 0, 0, 1, dummy_signature()),
            Vote1::new(dummy_party_id(2), vec![hash(100), hash(200)], 0, 0, 1, dummy_signature()),
            Vote1::new(dummy_party_id(3), vec![hash(1000), hash(2000)], 0, 0, 1, dummy_signature()),
        ];
        let qc1 = QC1::new(votes);
        let result = qc1_certify(&qc1, &verifier);
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_qc3_certify_edge_case_long_prefix_chain() {
        let votes = vec![
            Vote3::new(dummy_party_id(1), vec![hash(1)], QC2::new(vec![]), 0, 0, 1, dummy_signature()),
            Vote3::new(dummy_party_id(2), vec![hash(1), hash(2)], QC2::new(vec![]), 0, 0, 1, dummy_signature()),
            Vote3::new(dummy_party_id(3), vec![hash(1), hash(2), hash(3)], QC2::new(vec![]), 0, 0, 1, dummy_signature()),
        ];
        let qc3 = QC3::new(votes);
        let (v_low, v_high) = qc3_certify(&qc3);
        assert_eq!(v_low, vec![hash(1)]);
        assert_eq!(v_high, vec![hash(1), hash(2), hash(3)]);
    }

    #[test]
    #[should_panic(expected = "IMPLEMENTATION BUG")]
    fn test_qc3_certify_edge_case_inconsistent_prefixes_panics() {
        let votes = vec![
            Vote3::new(dummy_party_id(1), vec![hash(1), hash(2), hash(3)], QC2::new(vec![]), 0, 0, 1, dummy_signature()),
            Vote3::new(dummy_party_id(2), vec![hash(1), hash(2), hash(99)], QC2::new(vec![]), 0, 0, 1, dummy_signature()),
        ];
        let qc3 = QC3::new(votes);
        qc3_certify(&qc3);
    }
}
