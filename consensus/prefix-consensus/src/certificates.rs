// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Certificate types for Strong Prefix Consensus
//!
//! Certificates link views together, enabling parties to trace back to View 1
//! and agree on a unique v_high output.
//!
//! ## Certificate Types
//!
//! - **DirectCertificate**: Created when a view produces non-empty v_high.
//!   Contains the view number and QC3 proof. v_high is derived from the proof.
//!
//! - **IndirectCertificate**: Created by aggregating EmptyViewMessages from validators
//!   with >1/3 of total stake when a view produces empty v_high. Points to the MAX
//!   highest_known_view among all messages, allowing skipping of empty views.
//!
//! ## Protocol Flow
//!
//! View 1: Parties input raw vectors to Prefix Consensus
//! Views 2+: Parties broadcast certificates, collect others, construct ranked
//!           input vector from certificate hashes, run Prefix Consensus

use crate::certify::qc3_certify;
use crate::types::{PartyId, PrefixVector, QC3};
use crate::verification::{qc3_view, verify_qc3};
use anyhow::{ensure, Result};
use aptos_crypto::{bls12381::Signature as BlsSignature, HashValue};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::validator_verifier::ValidatorVerifier;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// ============================================================================
// Direct Certificate
// ============================================================================

/// Direct certificate for view advancement
///
/// Created when a view produces a non-empty v_high output.
/// The v_high can be derived from the proof using qc3_certify().
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirectCertificate {
    /// The view this certificate is from
    pub view: u64,
    /// The QC3 proof from completing this view
    pub proof: QC3,
}

impl DirectCertificate {
    pub fn new(view: u64, proof: QC3) -> Self {
        Self { view, proof }
    }

    /// The view this certificate is for
    pub fn view(&self) -> u64 {
        self.view
    }

    /// Parent view for tracing (same as view for direct certs)
    pub fn parent_view(&self) -> u64 {
        self.view
    }

    /// Derive v_high from the proof
    pub fn v_high(&self) -> PrefixVector {
        let (_, v_high) = qc3_certify(&self.proof);
        v_high
    }

    /// Validate this certificate
    pub fn validate(&self, verifier: &ValidatorVerifier) -> Result<()> {
        // Verify QC3 structure and all signatures
        verify_qc3(&self.proof, verifier)?;

        // Verify view matches QC3 (replay protection)
        let proof_view = qc3_view(&self.proof);
        ensure!(
            proof_view == Some(self.view),
            "Direct certificate view {} doesn't match proof view {:?}",
            self.view,
            proof_view
        );

        // Direct cert requires non-empty v_high, unless view == 1
        // View 1 can have empty v_high (e.g., inputs conflict at first entry)
        let v_high = self.v_high();
        ensure!(
            !v_high.is_empty() || self.view == 1,
            "Direct certificate requires non-empty v_high (unless view 1)"
        );

        Ok(())
    }

    /// Canonical hash for this certificate
    pub fn hash(&self) -> HashValue {
        let bytes = bcs::to_bytes(self).expect("BCS serialization failed");
        HashValue::sha3_256_of(&bytes)
    }
}

// ============================================================================
// Empty-View Message
// ============================================================================

/// Data that is signed in an empty-view message
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, CryptoHasher, BCSCryptoHash)]
pub struct EmptyViewStatement {
    /// The view that produced empty v_high
    pub empty_view: u64,
    /// The sender's highest known view with non-empty v_high
    pub highest_known_view: u64,
}

impl EmptyViewStatement {
    pub fn new(empty_view: u64, highest_known_view: u64) -> Self {
        Self {
            empty_view,
            highest_known_view,
        }
    }
}

/// Message broadcast when a party gets empty v_high from a view
///
/// Contains the party's highest known non-empty view so that
/// indirect certificates can find the best parent.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmptyViewMessage {
    /// The signed statement
    pub statement: EmptyViewStatement,
    /// Who sent this message
    pub author: PartyId,
    /// Proof for the highest_known_view (QC3 from that view)
    pub highest_known_proof: QC3,
    /// Signature on the statement
    pub signature: BlsSignature,
}

impl EmptyViewMessage {
    pub fn new(
        empty_view: u64,
        author: PartyId,
        highest_known_view: u64,
        highest_known_proof: QC3,
        signature: BlsSignature,
    ) -> Self {
        Self {
            statement: EmptyViewStatement::new(empty_view, highest_known_view),
            author,
            highest_known_proof,
            signature,
        }
    }

    pub fn empty_view(&self) -> u64 {
        self.statement.empty_view
    }

    pub fn highest_known_view(&self) -> u64 {
        self.statement.highest_known_view
    }

    /// Verify this message
    pub fn verify(&self, verifier: &ValidatorVerifier) -> Result<()> {
        // Verify signature on statement
        verifier.verify(self.author, &self.statement, &self.signature)?;

        // Verify the highest_known_proof is valid
        verify_qc3(&self.highest_known_proof, verifier)?;

        // Verify highest_known_view matches the proof's view (replay protection)
        let proof_view = qc3_view(&self.highest_known_proof);
        ensure!(
            proof_view == Some(self.highest_known_view()),
            "highest_known_view {} doesn't match proof view {:?}",
            self.highest_known_view(),
            proof_view
        );

        // Verify the proof produces valid v_high
        // Empty v_high is only allowed for view 1 (e.g., inputs conflict at first entry)
        let (_, v_high) = qc3_certify(&self.highest_known_proof);
        ensure!(
            !v_high.is_empty() || self.highest_known_view() == 1,
            "highest_known_proof must have non-empty v_high (unless view 1)"
        );

        // Verify highest_known_view < empty_view
        ensure!(
            self.highest_known_view() < self.empty_view(),
            "highest_known_view {} must be less than empty_view {}",
            self.highest_known_view(),
            self.empty_view()
        );

        Ok(())
    }
}

// ============================================================================
// Indirect Certificate
// ============================================================================

/// Indirect certificate for skipping empty views
///
/// Created by aggregating EmptyViewMessages from validators with >1/3 of total
/// stake (minority quorum). Points to the MAX highest_known_view among all
/// messages, allowing us to skip multiple empty views at once.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndirectCertificate {
    /// The view that was empty
    pub empty_view: u64,
    /// The parent view (MAX of all highest_known_view in messages)
    pub parent_view: u64,
    /// Proof for parent_view (from the message with max highest_known_view)
    pub parent_proof: QC3,
    /// The empty-view messages from validators with >1/3 stake
    pub messages: Vec<EmptyViewMessage>,
}

impl IndirectCertificate {
    /// Create an indirect certificate from empty-view messages
    ///
    /// Automatically selects the MAX highest_known_view as parent.
    /// Requires messages from validators with >1/3 stake (minority quorum).
    pub fn from_messages(
        empty_view: u64,
        messages: Vec<EmptyViewMessage>,
        verifier: &ValidatorVerifier,
    ) -> Result<Self> {
        ensure!(!messages.is_empty(), "Need at least one message");

        // Check that message authors have sufficient voting power (>1/3 stake)
        let authors: Vec<_> = messages.iter().map(|m| m.author).collect();
        verifier
            .check_voting_power(authors.iter(), false)
            .map_err(|e| anyhow::anyhow!("Insufficient voting power for indirect certificate: {}", e))?;

        // Find message with max highest_known_view
        let best_message = messages
            .iter()
            .max_by_key(|m| m.highest_known_view())
            .unwrap();

        Ok(Self {
            empty_view,
            parent_view: best_message.highest_known_view(),
            parent_proof: best_message.highest_known_proof.clone(),
            messages,
        })
    }

    /// The view this certificate is for (the empty view)
    pub fn view(&self) -> u64 {
        self.empty_view
    }

    /// Parent view for tracing (jump to this view)
    pub fn parent_view(&self) -> u64 {
        self.parent_view
    }

    /// Derive v_high from the parent proof
    pub fn v_high(&self) -> PrefixVector {
        let (_, v_high) = qc3_certify(&self.parent_proof);
        v_high
    }

    /// Validate this certificate
    pub fn validate(&self, verifier: &ValidatorVerifier) -> Result<()> {
        // Verify parent_view < empty_view
        ensure!(
            self.parent_view < self.empty_view,
            "parent_view {} must be less than empty_view {}",
            self.parent_view,
            self.empty_view
        );

        // Verify parent_proof
        verify_qc3(&self.parent_proof, verifier)?;

        // Verify parent_view matches parent_proof's view (replay protection)
        let proof_view = qc3_view(&self.parent_proof);
        ensure!(
            proof_view == Some(self.parent_view),
            "parent_view {} doesn't match parent_proof view {:?}",
            self.parent_view,
            proof_view
        );

        // Verify parent_proof has non-empty v_high
        ensure!(
            !self.v_high().is_empty(),
            "Parent proof must have non-empty v_high"
        );

        // Track seen authors and max view found
        let mut seen_authors = HashSet::new();
        let mut max_view_found = 0u64;

        for msg in &self.messages {
            // All messages must be for the same empty_view
            ensure!(
                msg.empty_view() == self.empty_view,
                "Message empty_view {} doesn't match certificate empty_view {}",
                msg.empty_view(),
                self.empty_view
            );

            // No duplicate authors
            ensure!(
                seen_authors.insert(msg.author),
                "Duplicate author: {}",
                msg.author
            );

            // Verify the message itself
            msg.verify(verifier)?;

            // Track max view
            if msg.highest_known_view() > max_view_found {
                max_view_found = msg.highest_known_view();
            }
        }

        // Verify messages have sufficient voting power (>1/3 stake = minority quorum)
        let authors: Vec<_> = self.messages.iter().map(|m| m.author).collect();
        verifier
            .check_voting_power(authors.iter(), false)
            .map_err(|e| anyhow::anyhow!("Indirect certificate insufficient voting power: {}", e))?;

        // Verify parent_view matches the max from messages
        ensure!(
            self.parent_view == max_view_found,
            "parent_view {} doesn't match max from messages {}",
            self.parent_view,
            max_view_found
        );

        Ok(())
    }

    /// Canonical hash for this certificate
    pub fn hash(&self) -> HashValue {
        let bytes = bcs::to_bytes(self).expect("BCS serialization failed");
        HashValue::sha3_256_of(&bytes)
    }
}

// ============================================================================
// Unified Certificate Type
// ============================================================================

/// A certificate that can be either direct or indirect
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Certificate {
    Direct(DirectCertificate),
    Indirect(IndirectCertificate),
}

impl Certificate {
    /// The view this certificate is for
    pub fn view(&self) -> u64 {
        match self {
            Certificate::Direct(c) => c.view(),
            Certificate::Indirect(c) => c.view(),
        }
    }

    /// Parent view for tracing back to View 1
    pub fn parent_view(&self) -> u64 {
        match self {
            Certificate::Direct(c) => c.parent_view(),
            Certificate::Indirect(c) => c.parent_view(),
        }
    }

    /// Derive v_high from the proof
    pub fn v_high(&self) -> PrefixVector {
        match self {
            Certificate::Direct(c) => c.v_high(),
            Certificate::Indirect(c) => c.v_high(),
        }
    }

    /// Validate this certificate
    pub fn validate(&self, verifier: &ValidatorVerifier) -> Result<()> {
        match self {
            Certificate::Direct(c) => c.validate(verifier),
            Certificate::Indirect(c) => c.validate(verifier),
        }
    }

    /// Canonical hash for this certificate (used in input vectors)
    pub fn hash(&self) -> HashValue {
        match self {
            Certificate::Direct(c) => c.hash(),
            Certificate::Indirect(c) => c.hash(),
        }
    }

    pub fn is_direct(&self) -> bool {
        matches!(self, Certificate::Direct(_))
    }

    pub fn is_indirect(&self) -> bool {
        matches!(self, Certificate::Indirect(_))
    }
}

// ============================================================================
// Helper: Highest Known View Tracker
// ============================================================================

/// Tracks the highest known view with a verifiable v_high
///
/// Used when a party needs to create an EmptyViewMessage.
/// Note: View 1 can have empty v_high (e.g., inputs conflict at first entry).
/// Views > 1 must have non-empty v_high to be tracked.
#[derive(Clone, Debug)]
pub struct HighestKnownView {
    pub view: u64,
    pub proof: QC3,
}

impl HighestKnownView {
    pub fn new(view: u64, proof: QC3) -> Self {
        Self { view, proof }
    }

    /// Derive v_high from the proof
    pub fn v_high(&self) -> PrefixVector {
        let (_, v_high) = qc3_certify(&self.proof);
        v_high
    }

    /// Update if the new view is higher and has valid v_high
    ///
    /// A valid v_high is:
    /// - Non-empty for any view, OR
    /// - Empty only if view == 1 (e.g., inputs conflict at first entry)
    pub fn update_if_higher(&mut self, view: u64, proof: QC3) {
        let (_, v_high) = qc3_certify(&proof);
        let v_high_is_valid = !v_high.is_empty() || view == 1;
        if view > self.view && v_high_is_valid {
            self.view = view;
            self.proof = proof;
        }
    }

    /// Create an EmptyViewMessage for a given empty view
    pub fn create_empty_view_message(
        &self,
        empty_view: u64,
        author: PartyId,
        signature: BlsSignature,
    ) -> EmptyViewMessage {
        EmptyViewMessage::new(
            empty_view,
            author,
            self.view,
            self.proof.clone(),
            signature,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Vote1, Vote2, Vote3, QC1, QC2};
    use aptos_types::validator_verifier::ValidatorConsensusInfo;

    fn hash(i: u64) -> HashValue {
        HashValue::sha3_256_of(&i.to_le_bytes())
    }

    fn dummy_party_id(i: u8) -> PartyId {
        PartyId::new([i; 32])
    }

    fn dummy_signature() -> BlsSignature {
        BlsSignature::dummy_signature()
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

    // Helper to create a QC3 with the given mcp_prefixes and view number
    fn create_qc3_with_prefixes_and_view(prefixes: Vec<PrefixVector>, view: u64) -> QC3 {
        let qc1 = QC1::new(vec![
            Vote1::new(dummy_party_id(0), vec![hash(1)], 0, 0, view, dummy_signature()),
            Vote1::new(dummy_party_id(1), vec![hash(1)], 0, 0, view, dummy_signature()),
            Vote1::new(dummy_party_id(2), vec![hash(1)], 0, 0, view, dummy_signature()),
        ]);

        let qc2 = QC2::new(vec![
            Vote2::new(dummy_party_id(0), vec![hash(1)], qc1.clone(), 0, 0, view, dummy_signature()),
            Vote2::new(dummy_party_id(1), vec![hash(1)], qc1.clone(), 0, 0, view, dummy_signature()),
            Vote2::new(dummy_party_id(2), vec![hash(1)], qc1, 0, 0, view, dummy_signature()),
        ]);

        let votes: Vec<Vote3> = prefixes
            .into_iter()
            .enumerate()
            .map(|(i, prefix)| {
                Vote3::new(
                    dummy_party_id(i as u8),
                    prefix,
                    qc2.clone(),
                    0,
                    0,
                    view,
                    dummy_signature(),
                )
            })
            .collect();

        QC3::new(votes)
    }

    // Helper with default view = 1
    fn create_qc3_with_prefixes(prefixes: Vec<PrefixVector>) -> QC3 {
        create_qc3_with_prefixes_and_view(prefixes, 1)
    }

    // ========================================================================
    // DirectCertificate Tests
    // ========================================================================

    #[test]
    fn test_direct_certificate_new() {
        let qc3 = create_qc3_with_prefixes(vec![
            vec![hash(1), hash(2)],
            vec![hash(1), hash(2)],
            vec![hash(1), hash(2)],
        ]);

        let cert = DirectCertificate::new(1, qc3);

        assert_eq!(cert.view(), 1);
        assert_eq!(cert.parent_view(), 1);
        assert_eq!(cert.v_high(), vec![hash(1), hash(2)]);
    }

    #[test]
    fn test_direct_certificate_v_high_derivation() {
        // QC3 with consistent prefixes: mce = longest
        let qc3 = create_qc3_with_prefixes(vec![
            vec![hash(1)],
            vec![hash(1), hash(2)],
            vec![hash(1), hash(2), hash(3)],
        ]);

        let cert = DirectCertificate::new(1, qc3);

        // v_high should be mce = [hash(1), hash(2), hash(3)]
        assert_eq!(cert.v_high(), vec![hash(1), hash(2), hash(3)]);
    }

    #[test]
    fn test_direct_certificate_hash_determinism() {
        let qc3 = create_qc3_with_prefixes(vec![
            vec![hash(1), hash(2)],
            vec![hash(1), hash(2)],
            vec![hash(1), hash(2)],
        ]);

        let cert1 = DirectCertificate::new(1, qc3.clone());
        let cert2 = DirectCertificate::new(1, qc3);

        assert_eq!(cert1.hash(), cert2.hash());
    }

    #[test]
    fn test_direct_certificate_different_views_different_hash() {
        let qc3 = create_qc3_with_prefixes(vec![
            vec![hash(1), hash(2)],
            vec![hash(1), hash(2)],
            vec![hash(1), hash(2)],
        ]);

        let cert1 = DirectCertificate::new(1, qc3.clone());
        let cert2 = DirectCertificate::new(2, qc3);

        assert_ne!(cert1.hash(), cert2.hash());
    }

    // ========================================================================
    // EmptyViewStatement Tests
    // ========================================================================

    #[test]
    fn test_empty_view_statement() {
        let stmt = EmptyViewStatement::new(5, 3);
        assert_eq!(stmt.empty_view, 5);
        assert_eq!(stmt.highest_known_view, 3);
    }

    // ========================================================================
    // EmptyViewMessage Tests
    // ========================================================================

    #[test]
    fn test_empty_view_message_new() {
        let qc3 = create_qc3_with_prefixes(vec![
            vec![hash(1)],
            vec![hash(1)],
            vec![hash(1)],
        ]);

        let msg = EmptyViewMessage::new(
            5,
            dummy_party_id(0),
            3,
            qc3,
            dummy_signature(),
        );

        assert_eq!(msg.empty_view(), 5);
        assert_eq!(msg.highest_known_view(), 3);
        assert_eq!(msg.author, dummy_party_id(0));
    }

    // ========================================================================
    // IndirectCertificate Tests
    // ========================================================================

    #[test]
    fn test_indirect_certificate_from_messages() {
        // 4 validators, each with weight 1, quorum = 3, minority (>1/3) = 2
        let verifier = create_test_verifier(4);

        let qc3_view3 = create_qc3_with_prefixes(vec![
            vec![hash(1)],
            vec![hash(1)],
            vec![hash(1)],
        ]);

        let qc3_view2 = create_qc3_with_prefixes(vec![
            vec![hash(1)],
            vec![hash(1)],
            vec![hash(1)],
        ]);

        // Messages with different highest_known_views (3 messages = >1/3 of 4)
        let messages = vec![
            EmptyViewMessage::new(5, dummy_party_id(0), 2, qc3_view2.clone(), dummy_signature()),
            EmptyViewMessage::new(5, dummy_party_id(1), 3, qc3_view3.clone(), dummy_signature()),
            EmptyViewMessage::new(5, dummy_party_id(2), 2, qc3_view2, dummy_signature()),
        ];

        let cert = IndirectCertificate::from_messages(5, messages, &verifier).unwrap();

        assert_eq!(cert.view(), 5);
        // Should select MAX = 3
        assert_eq!(cert.parent_view(), 3);
    }

    #[test]
    fn test_indirect_certificate_empty_messages_error() {
        let verifier = create_test_verifier(4);
        let result = IndirectCertificate::from_messages(5, vec![], &verifier);
        assert!(result.is_err());
    }

    #[test]
    fn test_indirect_certificate_hash_determinism() {
        // 3 validators, each with weight 1, minority (>1/3) = 2
        let verifier = create_test_verifier(3);

        let qc3 = create_qc3_with_prefixes(vec![
            vec![hash(1)],
            vec![hash(1)],
            vec![hash(1)],
        ]);

        let messages = vec![
            EmptyViewMessage::new(5, dummy_party_id(0), 3, qc3.clone(), dummy_signature()),
            EmptyViewMessage::new(5, dummy_party_id(1), 3, qc3.clone(), dummy_signature()),
        ];

        let cert1 = IndirectCertificate::from_messages(5, messages.clone(), &verifier).unwrap();
        let cert2 = IndirectCertificate::from_messages(5, messages, &verifier).unwrap();

        assert_eq!(cert1.hash(), cert2.hash());
    }

    // ========================================================================
    // Certificate Enum Tests
    // ========================================================================

    #[test]
    fn test_certificate_direct_variant() {
        let qc3 = create_qc3_with_prefixes(vec![
            vec![hash(1), hash(2)],
            vec![hash(1), hash(2)],
            vec![hash(1), hash(2)],
        ]);

        let cert = Certificate::Direct(DirectCertificate::new(1, qc3));

        assert!(cert.is_direct());
        assert!(!cert.is_indirect());
        assert_eq!(cert.view(), 1);
        assert_eq!(cert.parent_view(), 1);
    }

    #[test]
    fn test_certificate_indirect_variant() {
        // 3 validators, minority (>1/3) = 2
        let verifier = create_test_verifier(3);

        let qc3 = create_qc3_with_prefixes(vec![
            vec![hash(1)],
            vec![hash(1)],
            vec![hash(1)],
        ]);

        let messages = vec![
            EmptyViewMessage::new(5, dummy_party_id(0), 3, qc3.clone(), dummy_signature()),
            EmptyViewMessage::new(5, dummy_party_id(1), 3, qc3, dummy_signature()),
        ];

        let indirect = IndirectCertificate::from_messages(5, messages, &verifier).unwrap();
        let cert = Certificate::Indirect(indirect);

        assert!(!cert.is_direct());
        assert!(cert.is_indirect());
        assert_eq!(cert.view(), 5);
        assert_eq!(cert.parent_view(), 3);
    }

    #[test]
    fn test_certificate_unified_interface() {
        // 3 validators, minority (>1/3) = 2
        let verifier = create_test_verifier(3);

        // Create direct and indirect certificates
        let qc3 = create_qc3_with_prefixes(vec![
            vec![hash(1), hash(2)],
            vec![hash(1), hash(2)],
            vec![hash(1), hash(2)],
        ]);

        let direct = Certificate::Direct(DirectCertificate::new(1, qc3.clone()));

        let messages = vec![
            EmptyViewMessage::new(5, dummy_party_id(0), 3, qc3.clone(), dummy_signature()),
            EmptyViewMessage::new(5, dummy_party_id(1), 3, qc3, dummy_signature()),
        ];
        let indirect = Certificate::Indirect(
            IndirectCertificate::from_messages(5, messages, &verifier).unwrap()
        );

        // Both should provide same interface
        let _ = direct.view();
        let _ = direct.parent_view();
        let _ = direct.v_high();
        let _ = direct.hash();

        let _ = indirect.view();
        let _ = indirect.parent_view();
        let _ = indirect.v_high();
        let _ = indirect.hash();
    }

    // ========================================================================
    // HighestKnownView Tests
    // ========================================================================

    #[test]
    fn test_highest_known_view_new() {
        let qc3 = create_qc3_with_prefixes(vec![
            vec![hash(1), hash(2)],
            vec![hash(1), hash(2)],
            vec![hash(1), hash(2)],
        ]);

        let tracker = HighestKnownView::new(1, qc3);

        assert_eq!(tracker.view, 1);
        assert_eq!(tracker.v_high(), vec![hash(1), hash(2)]);
    }

    #[test]
    fn test_highest_known_view_update_if_higher() {
        let qc3_v1 = create_qc3_with_prefixes(vec![
            vec![hash(1)],
            vec![hash(1)],
            vec![hash(1)],
        ]);

        let qc3_v2 = create_qc3_with_prefixes(vec![
            vec![hash(1), hash(2)],
            vec![hash(1), hash(2)],
            vec![hash(1), hash(2)],
        ]);

        let mut tracker = HighestKnownView::new(1, qc3_v1);
        assert_eq!(tracker.view, 1);

        // Update with higher view
        tracker.update_if_higher(2, qc3_v2);
        assert_eq!(tracker.view, 2);
        assert_eq!(tracker.v_high(), vec![hash(1), hash(2)]);
    }

    #[test]
    fn test_highest_known_view_no_update_if_lower() {
        let qc3_v2 = create_qc3_with_prefixes(vec![
            vec![hash(1), hash(2)],
            vec![hash(1), hash(2)],
            vec![hash(1), hash(2)],
        ]);

        let qc3_v1 = create_qc3_with_prefixes(vec![
            vec![hash(1)],
            vec![hash(1)],
            vec![hash(1)],
        ]);

        let mut tracker = HighestKnownView::new(2, qc3_v2.clone());

        // Try to update with lower view - should not change
        tracker.update_if_higher(1, qc3_v1);
        assert_eq!(tracker.view, 2);
    }

    #[test]
    fn test_highest_known_view_no_update_if_empty_v_high() {
        let qc3_v1 = create_qc3_with_prefixes(vec![
            vec![hash(1)],
            vec![hash(1)],
            vec![hash(1)],
        ]);

        // QC3 with empty prefixes -> empty v_high
        let qc3_empty = create_qc3_with_prefixes(vec![
            vec![],
            vec![],
            vec![],
        ]);

        let mut tracker = HighestKnownView::new(1, qc3_v1);

        // Try to update with empty v_high - should not change
        tracker.update_if_higher(2, qc3_empty);
        assert_eq!(tracker.view, 1);
    }

    #[test]
    fn test_highest_known_view_create_empty_view_message() {
        let qc3 = create_qc3_with_prefixes(vec![
            vec![hash(1)],
            vec![hash(1)],
            vec![hash(1)],
        ]);

        let tracker = HighestKnownView::new(3, qc3);

        let msg = tracker.create_empty_view_message(
            5,
            dummy_party_id(0),
            dummy_signature(),
        );

        assert_eq!(msg.empty_view(), 5);
        assert_eq!(msg.highest_known_view(), 3);
        assert_eq!(msg.author, dummy_party_id(0));
    }

    // ========================================================================
    // Serialization Tests
    // ========================================================================

    #[test]
    fn test_direct_certificate_serialization_roundtrip() {
        let qc3 = create_qc3_with_prefixes(vec![
            vec![hash(1), hash(2)],
            vec![hash(1), hash(2)],
            vec![hash(1), hash(2)],
        ]);

        let cert = DirectCertificate::new(1, qc3);
        let bytes = bcs::to_bytes(&cert).unwrap();
        let recovered: DirectCertificate = bcs::from_bytes(&bytes).unwrap();

        assert_eq!(cert, recovered);
    }

    #[test]
    fn test_indirect_certificate_serialization_roundtrip() {
        // 3 validators, minority (>1/3) = 2
        let verifier = create_test_verifier(3);

        let qc3 = create_qc3_with_prefixes(vec![
            vec![hash(1)],
            vec![hash(1)],
            vec![hash(1)],
        ]);

        let messages = vec![
            EmptyViewMessage::new(5, dummy_party_id(0), 3, qc3.clone(), dummy_signature()),
            EmptyViewMessage::new(5, dummy_party_id(1), 3, qc3, dummy_signature()),
        ];

        let cert = IndirectCertificate::from_messages(5, messages, &verifier).unwrap();
        let bytes = bcs::to_bytes(&cert).unwrap();
        let recovered: IndirectCertificate = bcs::from_bytes(&bytes).unwrap();

        assert_eq!(cert, recovered);
    }

    #[test]
    fn test_certificate_enum_serialization_roundtrip() {
        let qc3 = create_qc3_with_prefixes(vec![
            vec![hash(1), hash(2)],
            vec![hash(1), hash(2)],
            vec![hash(1), hash(2)],
        ]);

        let cert = Certificate::Direct(DirectCertificate::new(1, qc3));
        let bytes = bcs::to_bytes(&cert).unwrap();
        let recovered: Certificate = bcs::from_bytes(&bytes).unwrap();

        assert_eq!(cert, recovered);
    }
}
