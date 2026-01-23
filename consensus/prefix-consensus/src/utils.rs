// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Utility functions for prefix operations

use crate::types::PrefixVector;

/// Compute the maximum common prefix (mcp) of a collection of vectors
///
/// Returns the longest vector that is a prefix of all input vectors.
///
/// # Examples
///
/// ```ignore
/// let v1 = vec![1, 2, 3, 4];
/// let v2 = vec![1, 2, 3, 5];
/// let v3 = vec![1, 2, 6];
/// let mcp = max_common_prefix(&[v1, v2, v3]);
/// assert_eq!(mcp, vec![1, 2]);
/// ```
pub fn max_common_prefix(vectors: &[PrefixVector]) -> PrefixVector {
    if vectors.is_empty() {
        return Vec::new();
    }

    // Find the minimum length among all vectors
    let min_len = vectors.iter().map(|v| v.len()).min().unwrap_or(0);

    if min_len == 0 {
        return Vec::new();
    }

    let mut prefix = Vec::new();

    // For each position up to min_len
    for i in 0..min_len {
        // Get the element at position i from the first vector
        let first_elem = &vectors[0][i];

        // Check if all vectors have the same element at position i
        let all_match = vectors.iter().all(|v| &v[i] == first_elem);

        if all_match {
            prefix.push(*first_elem);
        } else {
            // First mismatch found, return prefix so far
            break;
        }
    }

    prefix
}

/// Compute the minimum common extension (mce) of a collection of vectors
///
/// Returns the shortest vector that extends all input vectors.
/// This is the vector that:
/// 1. Has all input vectors as prefixes
/// 2. Is minimal in length among such vectors
///
/// Returns `None` if the vectors are not mutually consistent (i.e., they diverge
/// and no single vector can extend all of them as prefixes).
///
/// # Implementation Note
///
/// The mce is computed by finding the longest vector among the inputs,
/// as this is the shortest vector that contains all inputs as prefixes.
///
/// # Examples
///
/// ```ignore
/// let v1 = vec![1, 2, 3];
/// let v2 = vec![1, 2, 3, 4];
/// let v3 = vec![1, 2, 3, 4, 5];
/// let mce = min_common_extension(&[v1, v2, v3]);
/// assert_eq!(mce, Some(vec![1, 2, 3, 4, 5]));
/// ```
pub fn min_common_extension(vectors: &[PrefixVector]) -> Option<PrefixVector> {
    if vectors.is_empty() {
        return Some(Vec::new());
    }

    // First verify that all vectors are consistent (one must be prefix of all others)
    if !all_consistent(vectors) {
        // If not consistent, return None
        return None;
    }

    // The mce is the longest vector among the inputs
    vectors
        .iter()
        .max_by_key(|v| v.len())
        .cloned()
        .map(Some)
        .unwrap_or(Some(Vec::new()))
}

/// Check if a vector is a prefix of another vector
///
/// Returns true if `prefix` is a prefix of `vector`.
pub fn is_prefix_of(prefix: &PrefixVector, vector: &PrefixVector) -> bool {
    if prefix.len() > vector.len() {
        return false;
    }

    prefix
        .iter()
        .zip(vector.iter())
        .all(|(a, b)| a == b)
}

/// Check if two vectors are consistent (one is a prefix of the other)
///
/// Two vectors are consistent if one is a prefix of the other, or they are equal.
pub fn are_consistent(v1: &PrefixVector, v2: &PrefixVector) -> bool {
    is_prefix_of(v1, v2) || is_prefix_of(v2, v1)
}

/// Check if all vectors in a collection are mutually consistent
///
/// A collection of vectors is mutually consistent if for any two vectors,
/// one is a prefix of the other.
pub fn all_consistent(vectors: &[PrefixVector]) -> bool {
    if vectors.len() <= 1 {
        return true;
    }

    // Check pairwise consistency using iterator combinators
    vectors.iter().enumerate().all(|(i, v1)| {
        vectors.iter().skip(i + 1).all(|v2| are_consistent(v1, v2))
    })
}

/// Perform a consistency check on a collection of vectors
///
/// This is a more thorough check that verifies mutual consistency
/// and returns detailed information about any inconsistencies.
pub fn consistency_check(vectors: &[PrefixVector]) -> Result<(), String> {
    if vectors.is_empty() {
        return Ok(());
    }

    if vectors.len() == 1 {
        return Ok(());
    }

    // Check pairwise consistency
    for (i, v1) in vectors.iter().enumerate() {
        for (j, v2) in vectors.iter().enumerate() {
            if i >= j {
                continue;
            }

            if !are_consistent(v1, v2) {
                return Err(format!(
                    "Vectors at indices {} and {} are inconsistent: {:?} and {:?}",
                    i, j, v1, v2
                ));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::HashValue;

    fn hash(i: u64) -> HashValue {
        HashValue::sha3_256_of(&i.to_le_bytes())
    }

    #[test]
    fn test_max_common_prefix_empty() {
        let vectors: Vec<PrefixVector> = vec![];
        assert_eq!(max_common_prefix(&vectors), vec![]);
    }

    #[test]
    fn test_max_common_prefix_single() {
        let vectors = vec![vec![hash(1), hash(2), hash(3)]];
        assert_eq!(max_common_prefix(&vectors), vec![hash(1), hash(2), hash(3)]);
    }

    #[test]
    fn test_max_common_prefix_identical() {
        let v = vec![hash(1), hash(2), hash(3)];
        let vectors = vec![v.clone(), v.clone(), v.clone()];
        assert_eq!(max_common_prefix(&vectors), v);
    }

    #[test]
    fn test_max_common_prefix_different_lengths() {
        let v1 = vec![hash(1), hash(2), hash(3), hash(4)];
        let v2 = vec![hash(1), hash(2), hash(3)];
        let v3 = vec![hash(1), hash(2)];
        let vectors = vec![v1, v2, v3];
        assert_eq!(max_common_prefix(&vectors), vec![hash(1), hash(2)]);
    }

    #[test]
    fn test_max_common_prefix_diverge_early() {
        let v1 = vec![hash(1), hash(2), hash(3)];
        let v2 = vec![hash(1), hash(3), hash(4)];
        let v3 = vec![hash(1), hash(2), hash(5)];
        let vectors = vec![v1, v2, v3];
        assert_eq!(max_common_prefix(&vectors), vec![hash(1)]);
    }

    #[test]
    fn test_max_common_prefix_no_common() {
        let v1 = vec![hash(1), hash(2)];
        let v2 = vec![hash(3), hash(4)];
        let vectors = vec![v1, v2];
        assert_eq!(max_common_prefix(&vectors), vec![]);
    }

    #[test]
    fn test_min_common_extension_empty() {
        let vectors: Vec<PrefixVector> = vec![];
        assert_eq!(min_common_extension(&vectors), Some(vec![]));
    }

    #[test]
    fn test_min_common_extension_consistent() {
        let v1 = vec![hash(1), hash(2)];
        let v2 = vec![hash(1), hash(2), hash(3)];
        let v3 = vec![hash(1), hash(2), hash(3), hash(4)];
        let vectors = vec![v1, v2.clone(), v3.clone()];
        assert_eq!(min_common_extension(&vectors), Some(v3));
    }

    #[test]
    fn test_min_common_extension_inconsistent() {
        // When vectors are inconsistent, should return None
        let v1 = vec![hash(1), hash(2), hash(3)];
        let v2 = vec![hash(1), hash(2), hash(4)];
        let vectors = vec![v1, v2];
        assert_eq!(min_common_extension(&vectors), None);
    }

    #[test]
    fn test_is_prefix_of() {
        let v1 = vec![hash(1), hash(2)];
        let v2 = vec![hash(1), hash(2), hash(3)];
        assert!(is_prefix_of(&v1, &v2));
        assert!(!is_prefix_of(&v2, &v1));
        assert!(is_prefix_of(&v1, &v1));
    }

    #[test]
    fn test_are_consistent() {
        let v1 = vec![hash(1), hash(2)];
        let v2 = vec![hash(1), hash(2), hash(3)];
        let v3 = vec![hash(1), hash(3)];

        assert!(are_consistent(&v1, &v2));
        assert!(are_consistent(&v2, &v1));
        assert!(!are_consistent(&v1, &v3));
    }

    #[test]
    fn test_all_consistent() {
        let v1 = vec![hash(1)];
        let v2 = vec![hash(1), hash(2)];
        let v3 = vec![hash(1), hash(2), hash(3)];
        let v4 = vec![hash(1), hash(3)];

        assert!(all_consistent(&[v1.clone(), v2.clone(), v3.clone()]));
        assert!(!all_consistent(&[v1, v2, v3, v4]));
    }

    #[test]
    fn test_consistency_check() {
        let v1 = vec![hash(1), hash(2)];
        let v2 = vec![hash(1), hash(2), hash(3)];
        let v3 = vec![hash(1), hash(3)];

        assert!(consistency_check(&[v1.clone(), v2.clone()]).is_ok());
        assert!(consistency_check(&[v1, v2, v3]).is_err());
    }
}
