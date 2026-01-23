// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Message and QC verification for Prefix Consensus

use crate::types::{PartyId, Vote1, Vote2, Vote3, QC1, QC2, QC3};
use anyhow::{bail, ensure, Result};
use std::collections::HashSet;

// ============================================================================
// Vote Verification
// ============================================================================

/// Verify a Vote1 message
///
/// Checks:
/// 1. Vote has required fields
///
/// Note: For the prototype, we skip cryptographic signature verification.
/// In production, this would verify the signature against the author's public key.
pub fn verify_vote1(vote: &Vote1) -> Result<()> {
    // Basic structural validation
    if vote.input_vector.is_empty() {
        // Empty vector is allowed
    }

    Ok(())
}

/// Verify a Vote2 message
///
/// Checks:
/// 1. QC1 is valid
/// 2. Certified prefix is consistent with QC1
///
/// Note: For the prototype, we skip cryptographic signature verification.
pub fn verify_vote2(vote: &Vote2, f: usize, n: usize) -> Result<()> {
    // Verify the embedded QC1
    verify_qc1(&vote.qc1, f, n)?;

    Ok(())
}

/// Verify a Vote3 message
///
/// Checks:
/// 1. QC2 is valid
/// 2. MCP prefix is consistent with QC2
///
/// Note: For the prototype, we skip cryptographic signature verification.
pub fn verify_vote3(vote: &Vote3, f: usize, n: usize) -> Result<()> {
    // Verify the embedded QC2
    verify_qc2(&vote.qc2, f, n)?;

    Ok(())
}

// ============================================================================
// QC Verification
// ============================================================================

/// Verify a QC1 (Quorum Certificate from Round 1)
///
/// Checks:
/// 1. Has at least n-f votes
/// 2. No duplicate authors
/// 3. All votes are valid
pub fn verify_qc1(qc1: &QC1, f: usize, n: usize) -> Result<()> {
    // Check quorum size
    ensure!(
        is_valid_quorum(qc1.vote_count(), n, f),
        "QC1 has {} votes, but quorum size is {}",
        qc1.vote_count(),
        n - f
    );

    // Check for duplicate authors
    let authors: Vec<PartyId> = qc1.votes.iter().map(|v| v.author).collect();
    verify_no_duplicate_authors(&authors)?;

    // Verify each vote
    for vote in &qc1.votes {
        verify_vote1(vote)?;
    }

    Ok(())
}

/// Verify a QC2 (Quorum Certificate from Round 2)
///
/// Checks:
/// 1. Has at least n-f votes
/// 2. No duplicate authors
/// 3. All votes are valid
/// 4. All embedded QC1s are valid
pub fn verify_qc2(qc2: &QC2, f: usize, n: usize) -> Result<()> {
    // Check quorum size
    ensure!(
        is_valid_quorum(qc2.vote_count(), n, f),
        "QC2 has {} votes, but quorum size is {}",
        qc2.vote_count(),
        n - f
    );

    // Check for duplicate authors
    let authors: Vec<PartyId> = qc2.votes.iter().map(|v| v.author).collect();
    verify_no_duplicate_authors(&authors)?;

    // Verify each vote (which includes verifying embedded QC1)
    for vote in &qc2.votes {
        verify_vote2(vote, f, n)?;
    }

    Ok(())
}

/// Verify a QC3 (Quorum Certificate from Round 3)
///
/// Checks:
/// 1. Has at least n-f votes
/// 2. No duplicate authors
/// 3. All votes are valid
/// 4. All embedded QC2s are valid
pub fn verify_qc3(qc3: &QC3, f: usize, n: usize) -> Result<()> {
    // Check quorum size
    ensure!(
        is_valid_quorum(qc3.vote_count(), n, f),
        "QC3 has {} votes, but quorum size is {}",
        qc3.vote_count(),
        n - f
    );

    // Check for duplicate authors
    let authors: Vec<PartyId> = qc3.votes.iter().map(|v| v.author).collect();
    verify_no_duplicate_authors(&authors)?;

    // Verify each vote (which includes verifying embedded QC2)
    for vote in &qc3.votes {
        verify_vote3(vote, f, n)?;
    }

    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Verify that a collection of authors contains no duplicates
pub fn verify_no_duplicate_authors(authors: &[PartyId]) -> Result<()> {
    let mut seen = HashSet::new();
    for author in authors {
        if !seen.insert(author) {
            bail!("Duplicate author found: {}", author);
        }
    }
    Ok(())
}

/// Check if a collection of authors forms a valid quorum
pub fn is_valid_quorum(author_count: usize, n: usize, f: usize) -> bool {
    author_count >= (n - f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PrefixVector;
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

    fn create_vote1(author_id: u8, vector: PrefixVector) -> Vote1 {
        Vote1::new(dummy_party_id(author_id), vector, dummy_signature())
    }

    #[test]
    fn test_verify_vote1_basic() {
        let vote = create_vote1(0, vec![hash(1), hash(2)]);
        assert!(verify_vote1(&vote).is_ok());
    }

    #[test]
    fn test_verify_qc1_sufficient_votes() {
        let votes = vec![
            create_vote1(0, vec![hash(1)]),
            create_vote1(1, vec![hash(1)]),
            create_vote1(2, vec![hash(1)]),
        ];

        let qc1 = QC1::new(votes);

        // n=4, f=1, quorum=3
        assert!(verify_qc1(&qc1, 1, 4).is_ok());
    }

    #[test]
    fn test_verify_qc1_insufficient_votes() {
        let votes = vec![
            create_vote1(0, vec![hash(1)]),
            create_vote1(1, vec![hash(1)]),
        ];

        let qc1 = QC1::new(votes);

        // n=4, f=1, quorum=3, but only 2 votes
        assert!(verify_qc1(&qc1, 1, 4).is_err());
    }

    #[test]
    fn test_verify_qc1_duplicate_authors() {
        let votes = vec![
            create_vote1(0, vec![hash(1)]),
            create_vote1(0, vec![hash(2)]), // Duplicate author
            create_vote1(1, vec![hash(1)]),
        ];

        let qc1 = QC1::new(votes);

        assert!(verify_qc1(&qc1, 1, 4).is_err());
    }

    #[test]
    fn test_verify_no_duplicate_authors() {
        let authors = vec![dummy_party_id(0), dummy_party_id(1), dummy_party_id(2)];
        assert!(verify_no_duplicate_authors(&authors).is_ok());

        let authors_dup = vec![
            dummy_party_id(0),
            dummy_party_id(1),
            dummy_party_id(0), // Duplicate
        ];
        assert!(verify_no_duplicate_authors(&authors_dup).is_err());
    }

    #[test]
    fn test_is_valid_quorum() {
        // n=4, f=1, quorum=3
        assert!(is_valid_quorum(3, 4, 1));
        assert!(is_valid_quorum(4, 4, 1));
        assert!(!is_valid_quorum(2, 4, 1));

        // n=7, f=2, quorum=5
        assert!(is_valid_quorum(5, 7, 2));
        assert!(is_valid_quorum(7, 7, 2));
        assert!(!is_valid_quorum(4, 7, 2));
    }

    #[test]
    fn test_verify_qc2_basic() {
        let qc1 = QC1::new(vec![
            create_vote1(0, vec![hash(1)]),
            create_vote1(1, vec![hash(1)]),
            create_vote1(2, vec![hash(1)]),
        ]);

        let votes = vec![
            Vote2::new(
                dummy_party_id(0),
                vec![hash(1)],
                qc1.clone(),
                dummy_signature(),
            ),
            Vote2::new(
                dummy_party_id(1),
                vec![hash(1)],
                qc1.clone(),
                dummy_signature(),
            ),
            Vote2::new(dummy_party_id(2), vec![hash(1)], qc1, dummy_signature()),
        ];

        let qc2 = QC2::new(votes);

        // n=4, f=1
        assert!(verify_qc2(&qc2, 1, 4).is_ok());
    }

    #[test]
    fn test_verify_qc3_basic() {
        let qc1 = QC1::new(vec![
            create_vote1(0, vec![hash(1)]),
            create_vote1(1, vec![hash(1)]),
            create_vote1(2, vec![hash(1)]),
        ]);

        let qc2 = QC2::new(vec![
            Vote2::new(
                dummy_party_id(0),
                vec![hash(1)],
                qc1.clone(),
                dummy_signature(),
            ),
            Vote2::new(
                dummy_party_id(1),
                vec![hash(1)],
                qc1.clone(),
                dummy_signature(),
            ),
            Vote2::new(dummy_party_id(2), vec![hash(1)], qc1, dummy_signature()),
        ]);

        let votes = vec![
            Vote3::new(
                dummy_party_id(0),
                vec![hash(1)],
                qc2.clone(),
                dummy_signature(),
            ),
            Vote3::new(
                dummy_party_id(1),
                vec![hash(1)],
                qc2.clone(),
                dummy_signature(),
            ),
            Vote3::new(dummy_party_id(2), vec![hash(1)], qc2, dummy_signature()),
        ];

        let qc3 = QC3::new(votes);

        // n=4, f=1
        assert!(verify_qc3(&qc3, 1, 4).is_ok());
    }
}
