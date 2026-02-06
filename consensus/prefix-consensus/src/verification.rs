// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Message and QC verification for Prefix Consensus

use crate::{
    signing::{verify_vote1_signature, verify_vote2_signature, verify_vote3_signature},
    types::{PartyId, Vote1, Vote2, Vote3, QC1, QC2, QC3},
};
use anyhow::{bail, Result};
use aptos_types::validator_verifier::ValidatorVerifier;
use std::collections::HashSet;

// ============================================================================
// Vote Verification
// ============================================================================

/// Verify a Vote1 message
///
/// Checks:
/// 1. Signature is valid
pub fn verify_vote1(vote: &Vote1, verifier: &ValidatorVerifier) -> Result<()> {
    verify_vote1_signature(vote, &vote.author, verifier)?;
    Ok(())
}

/// Verify a Vote2 message
///
/// Checks:
/// 1. Embedded QC1 is valid (including all signatures)
/// 2. Signature is valid
pub fn verify_vote2(vote: &Vote2, verifier: &ValidatorVerifier) -> Result<()> {
    // Verify the embedded QC1
    verify_qc1(&vote.qc1, verifier)?;

    // Verify cryptographic signature
    verify_vote2_signature(vote, &vote.author, verifier)?;

    Ok(())
}

/// Verify a Vote3 message
///
/// Checks:
/// 1. Embedded QC2 is valid (including all signatures)
/// 2. Signature is valid
pub fn verify_vote3(vote: &Vote3, verifier: &ValidatorVerifier) -> Result<()> {
    // Verify the embedded QC2
    verify_qc2(&vote.qc2, verifier)?;

    // Verify cryptographic signature
    verify_vote3_signature(vote, &vote.author, verifier)?;

    Ok(())
}

// ============================================================================
// QC Verification
// ============================================================================

/// Verify a QC1 (Quorum Certificate from Round 1)
///
/// Checks:
/// 1. Has sufficient voting power (>2/3 stake)
/// 2. No duplicate authors
/// 3. All votes are valid (including signatures)
pub fn verify_qc1(qc1: &QC1, verifier: &ValidatorVerifier) -> Result<()> {
    // Check for duplicate authors first (before stake check)
    let authors: Vec<PartyId> = qc1.votes.iter().map(|v| v.author).collect();
    verify_no_duplicate_authors(&authors)?;

    // Check voting power (super majority = >2/3 stake)
    verifier
        .check_voting_power(authors.iter(), true)
        .map_err(|e| anyhow::anyhow!("QC1 insufficient voting power: {}", e))?;

    // Verify each vote (including signature)
    for vote in &qc1.votes {
        verify_vote1(vote, verifier)?;
    }

    Ok(())
}

/// Verify a QC2 (Quorum Certificate from Round 2)
///
/// Checks:
/// 1. Has sufficient voting power (>2/3 stake)
/// 2. No duplicate authors
/// 3. All votes are valid (including signatures)
/// 4. All embedded QC1s are valid (including signatures)
pub fn verify_qc2(qc2: &QC2, verifier: &ValidatorVerifier) -> Result<()> {
    // Check for duplicate authors first (before stake check)
    let authors: Vec<PartyId> = qc2.votes.iter().map(|v| v.author).collect();
    verify_no_duplicate_authors(&authors)?;

    // Check voting power (super majority = >2/3 stake)
    verifier
        .check_voting_power(authors.iter(), true)
        .map_err(|e| anyhow::anyhow!("QC2 insufficient voting power: {}", e))?;

    // Verify each vote (which includes verifying embedded QC1 with signatures)
    for vote in &qc2.votes {
        verify_vote2(vote, verifier)?;
    }

    Ok(())
}

/// Verify a QC3 (Quorum Certificate from Round 3)
///
/// Checks:
/// 1. Has sufficient voting power (>2/3 stake)
/// 2. No duplicate authors
/// 3. All votes are valid (including signatures)
/// 4. All embedded QC2s are valid (including all nested signatures)
pub fn verify_qc3(qc3: &QC3, verifier: &ValidatorVerifier) -> Result<()> {
    // Check for duplicate authors first (before stake check)
    let authors: Vec<PartyId> = qc3.votes.iter().map(|v| v.author).collect();
    verify_no_duplicate_authors(&authors)?;

    // Check voting power (super majority = >2/3 stake)
    verifier
        .check_voting_power(authors.iter(), true)
        .map_err(|e| anyhow::anyhow!("QC3 insufficient voting power: {}", e))?;

    // Verify each vote (which includes verifying embedded QC2 with all nested signatures)
    for vote in &qc3.votes {
        verify_vote3(vote, verifier)?;
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

// ============================================================================
// Proof Verification (for Verifiable Prefix Consensus)
// ============================================================================

/// Verify that a v_low output is correctly derived from a QC3 proof
///
/// This is a standalone helper for verifying individual outputs when you don't
/// have the full PrefixConsensusOutput struct (e.g., verifying certificates in
/// Strong Prefix Consensus).
///
/// **WARNING**: This function does NOT verify the QC3 structure itself. You must
/// call `verify_qc3()` separately to validate quorum size, signatures, etc.
///
/// # Arguments
/// * `v_low` - The claimed low output
/// * `qc3` - The QC3 certificate serving as proof
///
/// # Returns
/// True if v_low equals mcp of all mcp_prefix values in QC3
#[allow(dead_code)] // Part of public API for Strong Prefix Consensus
pub fn verify_low_proof(v_low: &crate::types::PrefixVector, qc3: &QC3) -> bool {
    use crate::certify::qc3_certify;
    let (computed_v_low, _) = qc3_certify(qc3);
    v_low == &computed_v_low
}

/// Verify that a v_high output is correctly derived from a QC3 proof
///
/// This is a standalone helper for verifying individual outputs when you don't
/// have the full PrefixConsensusOutput struct (e.g., verifying certificates in
/// Strong Prefix Consensus).
///
/// **WARNING**: This function does NOT verify the QC3 structure itself. You must
/// call `verify_qc3()` separately to validate quorum size, signatures, etc.
///
/// # Arguments
/// * `v_high` - The claimed high output
/// * `qc3` - The QC3 certificate serving as proof
///
/// # Returns
/// True if v_high equals mce of all mcp_prefix values in QC3
#[allow(dead_code)] // Part of public API for Strong Prefix Consensus
pub fn verify_high_proof(v_high: &crate::types::PrefixVector, qc3: &QC3) -> bool {
    use crate::certify::qc3_certify;
    let (_, computed_v_high) = qc3_certify(qc3);
    v_high == &computed_v_high
}

/// Verify both v_low and v_high outputs against a QC3 proof
///
/// This is a standalone helper for verifying outputs when you don't have the
/// full PrefixConsensusOutput struct.
///
/// **WARNING**: This function does NOT verify the QC3 structure itself. You must
/// call `verify_qc3()` separately to validate quorum size, signatures, etc.
///
/// # Arguments
/// * `v_low` - The claimed low output
/// * `v_high` - The claimed high output
/// * `qc3` - The QC3 certificate serving as proof
///
/// # Returns
/// True if BOTH v_low and v_high are correctly derived from QC3
#[allow(dead_code)] // Part of public API for Strong Prefix Consensus
pub fn verify_output_proofs(
    v_low: &crate::types::PrefixVector,
    v_high: &crate::types::PrefixVector,
    qc3: &QC3,
) -> bool {
    use crate::certify::qc3_certify;
    let (computed_v_low, computed_v_high) = qc3_certify(qc3);
    v_low == &computed_v_low && v_high == &computed_v_high
}

// ============================================================================
// View Extraction (for Strong Prefix Consensus replay protection)
// ============================================================================

/// Extract the view number from a QC3
///
/// Returns Some(view) if all votes in the QC3 have the same view number.
/// Returns None if the QC3 is empty or votes have inconsistent views.
///
/// This is used by certificate validation to verify that the proof
/// matches the claimed view number, preventing replay attacks.
pub fn qc3_view(qc3: &QC3) -> Option<u64> {
    if qc3.votes.is_empty() {
        return None;
    }

    let first_view = qc3.votes[0].view;

    // Verify all votes have the same view
    for vote in &qc3.votes {
        if vote.view != first_view {
            return None;
        }
    }

    Some(first_view)
}

/// Extract the view number from a QC2
///
/// Returns Some(view) if all votes in the QC2 have the same view number.
/// Returns None if the QC2 is empty or votes have inconsistent views.
pub fn qc2_view(qc2: &QC2) -> Option<u64> {
    if qc2.votes.is_empty() {
        return None;
    }

    let first_view = qc2.votes[0].view;

    for vote in &qc2.votes {
        if vote.view != first_view {
            return None;
        }
    }

    Some(first_view)
}

/// Extract the view number from a QC1
///
/// Returns Some(view) if all votes in the QC1 have the same view number.
/// Returns None if the QC1 is empty or votes have inconsistent views.
pub fn qc1_view(qc1: &QC1) -> Option<u64> {
    if qc1.votes.is_empty() {
        return None;
    }

    let first_view = qc1.votes[0].view;

    for vote in &qc1.votes {
        if vote.view != first_view {
            return None;
        }
    }

    Some(first_view)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PrefixVector;
    use aptos_crypto::HashValue;
    use aptos_types::validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier};

    fn hash(i: u64) -> HashValue {
        HashValue::sha3_256_of(&i.to_le_bytes())
    }

    fn dummy_party_id(i: u8) -> PartyId {
        PartyId::new([i; 32])
    }

    fn dummy_signature() -> aptos_crypto::bls12381::Signature {
        aptos_crypto::bls12381::Signature::dummy_signature()
    }

    fn create_test_verifier(count: usize) -> ValidatorVerifier {
        let validator_infos: Vec<_> = (0..count)
            .map(|i| {
                let signer = aptos_types::validator_signer::ValidatorSigner::random(None);
                ValidatorConsensusInfo::new(dummy_party_id(i as u8), signer.public_key(), 1)
            })
            .collect();
        ValidatorVerifier::new(validator_infos)
    }

    fn create_vote1(author_id: u8, vector: PrefixVector) -> Vote1 {
        Vote1::new(dummy_party_id(author_id), vector, 0, 0, 1, dummy_signature())
    }

    #[test]
    fn test_verify_vote1_basic() {
        let verifier = create_test_verifier(4);
        let vote = create_vote1(0, vec![hash(1), hash(2)]);
        // Will fail due to dummy signature not matching verifier's keys
        assert!(verify_vote1(&vote, &verifier).is_err());
    }

    #[test]
    fn test_verify_qc1_sufficient_votes() {
        let verifier = create_test_verifier(4);
        let votes = vec![
            create_vote1(0, vec![hash(1)]),
            create_vote1(1, vec![hash(1)]),
            create_vote1(2, vec![hash(1)]),
        ];

        let qc1 = QC1::new(votes);

        // 3 of 4 validators = 75% stake, but will fail due to dummy signatures
        assert!(verify_qc1(&qc1, &verifier).is_err());
    }

    #[test]
    fn test_verify_qc1_insufficient_votes() {
        let verifier = create_test_verifier(4);
        let votes = vec![
            create_vote1(0, vec![hash(1)]),
            create_vote1(1, vec![hash(1)]),
        ];

        let qc1 = QC1::new(votes);

        // 2 of 4 validators = 50% stake, insufficient for >2/3 quorum
        assert!(verify_qc1(&qc1, &verifier).is_err());
    }

    #[test]
    fn test_verify_qc1_duplicate_authors() {
        let verifier = create_test_verifier(4);
        let votes = vec![
            create_vote1(0, vec![hash(1)]),
            create_vote1(0, vec![hash(2)]), // Duplicate author
            create_vote1(1, vec![hash(1)]),
        ];

        let qc1 = QC1::new(votes);

        // Will fail due to duplicate authors (before stake check)
        assert!(verify_qc1(&qc1, &verifier).is_err());
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
                0, 0, 1,
                dummy_signature(),
            ),
            Vote2::new(
                dummy_party_id(1),
                vec![hash(1)],
                qc1.clone(),
                0, 0, 1,
                dummy_signature(),
            ),
            Vote2::new(dummy_party_id(2), vec![hash(1)], qc1, 0, 0, 1, dummy_signature()),
        ];

        let qc2 = QC2::new(votes);

        // 3 of 4 validators, but will fail due to dummy signatures
        let verifier = create_test_verifier(4);
        assert!(verify_qc2(&qc2, &verifier).is_err());
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
                0, 0, 1,
                dummy_signature(),
            ),
            Vote2::new(
                dummy_party_id(1),
                vec![hash(1)],
                qc1.clone(),
                0, 0, 1,
                dummy_signature(),
            ),
            Vote2::new(dummy_party_id(2), vec![hash(1)], qc1, 0, 0, 1, dummy_signature()),
        ]);

        let votes = vec![
            Vote3::new(
                dummy_party_id(0),
                vec![hash(1)],
                qc2.clone(),
                0, 0, 1,
                dummy_signature(),
            ),
            Vote3::new(
                dummy_party_id(1),
                vec![hash(1)],
                qc2.clone(),
                0, 0, 1,
                dummy_signature(),
            ),
            Vote3::new(dummy_party_id(2), vec![hash(1)], qc2, 0, 0, 1, dummy_signature()),
        ];

        let qc3 = QC3::new(votes);

        // 3 of 4 validators, but will fail due to dummy signatures
        let verifier = create_test_verifier(4);
        assert!(verify_qc3(&qc3, &verifier).is_err());
    }

    fn create_dummy_qc2() -> QC2 {
        let qc1 = QC1::new(vec![
            create_vote1(0, vec![hash(1)]),
            create_vote1(1, vec![hash(1)]),
            create_vote1(2, vec![hash(1)]),
        ]);

        QC2::new(vec![
            Vote2::new(
                dummy_party_id(0),
                vec![hash(1)],
                qc1.clone(),
                0,
                0,
                1,
                dummy_signature(),
            ),
            Vote2::new(
                dummy_party_id(1),
                vec![hash(1)],
                qc1.clone(),
                0,
                0,
                1,
                dummy_signature(),
            ),
            Vote2::new(dummy_party_id(2), vec![hash(1)], qc1, 0, 0, 1, dummy_signature()),
        ])
    }

    #[test]
    fn test_verify_low_proof_standalone() {
        let hash1 = hash(1);
        let hash2 = hash(2);

        let qc2 = create_dummy_qc2();
        let votes = vec![
            Vote3::new(
                dummy_party_id(0),
                vec![hash1, hash2],
                qc2.clone(),
                0,
                0,
                1,
                dummy_signature(),
            ),
            Vote3::new(
                dummy_party_id(1),
                vec![hash1, hash2],
                qc2.clone(),
                0,
                0,
                1,
                dummy_signature(),
            ),
            Vote3::new(
                dummy_party_id(2),
                vec![hash1, hash2],
                qc2,
                0,
                0,
                1,
                dummy_signature(),
            ),
        ];

        let qc3 = QC3::new(votes);

        // Correct v_low
        assert!(verify_low_proof(&vec![hash1, hash2], &qc3));

        // Incorrect v_low
        assert!(!verify_low_proof(&vec![hash1], &qc3));
        assert!(!verify_low_proof(&vec![hash1, hash2, hash(3)], &qc3));
    }

    #[test]
    fn test_verify_high_proof_standalone() {
        let hash1 = hash(1);
        let hash2 = hash(2);
        let hash3 = hash(3);

        let qc2 = create_dummy_qc2();
        let votes = vec![
            Vote3::new(
                dummy_party_id(0),
                vec![hash1, hash2, hash3],
                qc2.clone(),
                0,
                0,
                1,
                dummy_signature(),
            ),
            Vote3::new(
                dummy_party_id(1),
                vec![hash1, hash2],
                qc2.clone(),
                0,
                0,
                1,
                dummy_signature(),
            ),
            Vote3::new(dummy_party_id(2), vec![hash1], qc2, 0, 0, 1, dummy_signature()),
        ];

        let qc3 = QC3::new(votes);

        // mce = longest vector = [hash1, hash2, hash3]
        assert!(verify_high_proof(&vec![hash1, hash2, hash3], &qc3));

        // Incorrect v_high
        assert!(!verify_high_proof(&vec![hash1, hash2], &qc3));
        assert!(!verify_high_proof(&vec![hash1, hash2, hash3, hash(4)], &qc3));
    }

    #[test]
    fn test_verify_output_proofs_standalone() {
        let hash1 = hash(1);
        let hash2 = hash(2);

        let qc2 = create_dummy_qc2();
        let votes = vec![
            Vote3::new(
                dummy_party_id(0),
                vec![hash1, hash2],
                qc2.clone(),
                0,
                0,
                1,
                dummy_signature(),
            ),
            Vote3::new(
                dummy_party_id(1),
                vec![hash1, hash2],
                qc2.clone(),
                0,
                0,
                1,
                dummy_signature(),
            ),
            Vote3::new(
                dummy_party_id(2),
                vec![hash1, hash2],
                qc2,
                0,
                0,
                1,
                dummy_signature(),
            ),
        ];

        let qc3 = QC3::new(votes);

        // Both correct
        assert!(verify_output_proofs(
            &vec![hash1, hash2],
            &vec![hash1, hash2],
            &qc3
        ));

        // One incorrect
        assert!(!verify_output_proofs(&vec![hash1], &vec![hash1, hash2], &qc3));
        assert!(!verify_output_proofs(
            &vec![hash1, hash2],
            &vec![hash1],
            &qc3
        ));

        // Both incorrect
        assert!(!verify_output_proofs(&vec![hash1], &vec![hash1], &qc3));
    }
}
