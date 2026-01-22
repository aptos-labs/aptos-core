// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! QC Certification functions for Prefix Consensus
//!
//! This module implements the core certification functions from the paper:
//! - QC1Certify: Extract longest prefix in f+1 votes from QC1
//! - QC2Certify: Compute maximum common prefix from QC2
//! - QC3Certify: Compute both v_low (mcp) and v_high (mce) from QC3

use crate::types::{Element, PrefixVector, QC1, QC2, QC3};
use crate::utils::{max_common_prefix, min_common_extension};
use std::collections::HashMap;

// ============================================================================
// Trie Implementation for QC1Certify
// ============================================================================

/// A trie node for efficient prefix computation
#[derive(Debug)]
struct TrieNode {
    /// Children of this node, keyed by element
    children: HashMap<Element, TrieNode>,

    /// Number of votes that pass through this node
    vote_count: usize,
}

impl TrieNode {
    fn new() -> Self {
        Self {
            children: HashMap::new(),
            vote_count: 0,
        }
    }

    /// Insert a vector into the trie
    fn insert(&mut self, vector: &[Element]) {
        self.vote_count += 1;

        if vector.is_empty() {
            return;
        }

        let first = vector[0];
        let rest = &vector[1..];

        self.children
            .entry(first)
            .or_insert_with(TrieNode::new)
            .insert(rest);
    }

    /// Find the longest prefix that has at least `threshold` votes
    fn find_longest_prefix_with_threshold(
        &self,
        threshold: usize,
        current_prefix: &mut Vec<Element>,
        best_prefix: &mut Vec<Element>,
    ) {
        // If this node doesn't meet the threshold, stop
        if self.vote_count < threshold {
            return;
        }

        // Update best prefix if current is longer
        if current_prefix.len() > best_prefix.len() {
            *best_prefix = current_prefix.clone();
        }

        // Try to extend the prefix through children
        for (element, child) in &self.children {
            current_prefix.push(*element);
            child.find_longest_prefix_with_threshold(threshold, current_prefix, best_prefix);
            current_prefix.pop();
        }
    }
}

/// Build a trie from a collection of vectors
fn build_trie(vectors: &[PrefixVector]) -> TrieNode {
    let mut root = TrieNode::new();
    for vector in vectors {
        root.insert(vector);
    }
    root
}

// ============================================================================
// QC Certification Functions
// ============================================================================

/// QC1Certify: Extract the longest prefix that appears in at least f+1 votes
///
/// From the paper (Algorithm 1, Round 1):
/// "x := max{ mcp({v_i : i ∈ S}) : S ⊆ {votes in QC1}, |S|=f+1 }"
///
/// This finds the longest vector that is a common prefix of at least f+1 input vectors.
///
/// # Algorithm
///
/// Uses a trie data structure for O(total input size) complexity:
/// 1. Build trie from all vote vectors
/// 2. Each node tracks how many votes pass through it
/// 3. Find longest path where node.vote_count >= f+1
///
/// # Parameters
///
/// - `qc1`: The QC1 containing n-f votes
/// - `f`: Maximum number of Byzantine parties
///
/// # Returns
///
/// The longest certified prefix (may be empty if no f+1 agreement exists)
pub fn qc1_certify(qc1: &QC1, f: usize) -> PrefixVector {
    let threshold = f + 1;

    // Extract all input vectors from votes
    let vectors: Vec<PrefixVector> = qc1.votes.iter().map(|v| v.input_vector.clone()).collect();

    if vectors.is_empty() {
        return Vec::new();
    }

    // Build trie from all vectors
    let trie = build_trie(&vectors);

    // Find longest prefix with at least f+1 votes
    let mut current_prefix = Vec::new();
    let mut best_prefix = Vec::new();

    trie.find_longest_prefix_with_threshold(threshold, &mut current_prefix, &mut best_prefix);

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
/// - `qc2`: The QC2 containing n-f votes
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
/// - `qc3`: The QC3 containing n-f votes
///
/// # Returns
///
/// A tuple (v_low, v_high)
pub fn qc3_certify(qc3: &QC3) -> (PrefixVector, PrefixVector) {
    // Extract all mcp prefixes from votes
    let prefixes: Vec<PrefixVector> = qc3.votes.iter().map(|v| v.mcp_prefix.clone()).collect();

    if prefixes.is_empty() {
        return (Vec::new(), Vec::new());
    }

    // Compute v_low (maximum common prefix)
    let v_low = max_common_prefix(&prefixes);

    // Compute v_high (minimum common extension)
    let v_high = min_common_extension(&prefixes);

    (v_low, v_high)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Brute force approach to find longest prefix with threshold
///
/// This is an alternative implementation for testing/validation.
/// It has O(n choose f+1) complexity but is simpler to verify correctness.
#[cfg(test)]
fn qc1_certify_brute_force(qc1: &QC1, f: usize) -> PrefixVector {
    let vectors: Vec<PrefixVector> = qc1.votes.iter().map(|v| v.input_vector.clone()).collect();

    if vectors.len() < f + 1 {
        return Vec::new();
    }

    let mut best_prefix = Vec::new();

    // Generate all subsets of size f+1
    let indices: Vec<usize> = (0..vectors.len()).collect();
    for subset in combinations(&indices, f + 1) {
        let subset_vectors: Vec<PrefixVector> =
            subset.iter().map(|&i| vectors[i].clone()).collect();
        let mcp = max_common_prefix(&subset_vectors);

        if mcp.len() > best_prefix.len() {
            best_prefix = mcp;
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
    use aptos_crypto::{ed25519::Ed25519Signature, HashValue};

    fn hash(i: u64) -> HashValue {
        HashValue::sha3_256_of(&i.to_le_bytes())
    }

    fn dummy_party_id(i: u8) -> PartyId {
        PartyId::new([i; 32])
    }

    fn dummy_signature() -> Ed25519Signature {
        Ed25519Signature::try_from(&[0u8; 64][..]).unwrap()
    }

    #[test]
    fn test_trie_insert_and_count() {
        let mut trie = TrieNode::new();

        let v1 = vec![hash(1), hash(2), hash(3)];
        let v2 = vec![hash(1), hash(2), hash(4)];
        let v3 = vec![hash(1), hash(5)];

        trie.insert(&v1);
        trie.insert(&v2);
        trie.insert(&v3);

        // Root should have 3 votes
        assert_eq!(trie.vote_count, 3);

        // First level should have 1 child (hash(1)) with 3 votes
        assert_eq!(trie.children.len(), 1);
        let child1 = trie.children.get(&hash(1)).unwrap();
        assert_eq!(child1.vote_count, 3);
    }

    #[test]
    fn test_qc1_certify_empty() {
        let qc1 = QC1::new(vec![]);
        let result = qc1_certify(&qc1, 1);
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_qc1_certify_simple() {
        // Create 4 votes with n=4, f=1 (need 2 matching votes)
        let votes = vec![
            Vote1::new(
                dummy_party_id(0),
                vec![hash(1), hash(2), hash(3)],
                dummy_signature(),
            ),
            Vote1::new(
                dummy_party_id(1),
                vec![hash(1), hash(2), hash(4)],
                dummy_signature(),
            ),
            Vote1::new(dummy_party_id(2), vec![hash(1), hash(5)], dummy_signature()),
            Vote1::new(dummy_party_id(3), vec![hash(2), hash(3)], dummy_signature()),
        ];

        let qc1 = QC1::new(votes);
        let result = qc1_certify(&qc1, 1);

        // At least 2 votes should share prefix [hash(1)]
        // Votes 0, 1, 2 all start with hash(1)
        // Votes 0, 1 share [hash(1), hash(2)]
        assert_eq!(result, vec![hash(1), hash(2)]);
    }

    #[test]
    fn test_qc1_certify_all_same() {
        let common_vec = vec![hash(1), hash(2), hash(3)];
        let votes = vec![
            Vote1::new(dummy_party_id(0), common_vec.clone(), dummy_signature()),
            Vote1::new(dummy_party_id(1), common_vec.clone(), dummy_signature()),
            Vote1::new(dummy_party_id(2), common_vec.clone(), dummy_signature()),
        ];

        let qc1 = QC1::new(votes);
        let result = qc1_certify(&qc1, 1);

        // All votes identical, so entire vector should be certified
        assert_eq!(result, common_vec);
    }

    #[test]
    fn test_qc1_certify_vs_brute_force() {
        // Test that trie-based and brute force give same result
        let votes = vec![
            Vote1::new(
                dummy_party_id(0),
                vec![hash(1), hash(2), hash(3)],
                dummy_signature(),
            ),
            Vote1::new(
                dummy_party_id(1),
                vec![hash(1), hash(2), hash(4)],
                dummy_signature(),
            ),
            Vote1::new(dummy_party_id(2), vec![hash(1), hash(5)], dummy_signature()),
            Vote1::new(dummy_party_id(3), vec![hash(2), hash(3)], dummy_signature()),
        ];

        let qc1 = QC1::new(votes);
        let trie_result = qc1_certify(&qc1, 1);
        let brute_result = qc1_certify_brute_force(&qc1, 1);

        assert_eq!(trie_result, brute_result);
    }

    #[test]
    fn test_qc2_certify() {
        let votes = vec![
            Vote2::new(
                dummy_party_id(0),
                vec![hash(1), hash(2), hash(3)],
                QC1::new(vec![]),
                dummy_signature(),
            ),
            Vote2::new(
                dummy_party_id(1),
                vec![hash(1), hash(2)],
                QC1::new(vec![]),
                dummy_signature(),
            ),
            Vote2::new(
                dummy_party_id(2),
                vec![hash(1), hash(2), hash(3), hash(4)],
                QC1::new(vec![]),
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
                dummy_signature(),
            ),
            Vote3::new(
                dummy_party_id(1),
                vec![hash(1), hash(2), hash(3)],
                QC2::new(vec![]),
                dummy_signature(),
            ),
            Vote3::new(
                dummy_party_id(2),
                vec![hash(1), hash(2), hash(3), hash(4)],
                QC2::new(vec![]),
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
}
