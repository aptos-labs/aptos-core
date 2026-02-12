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
use crate::utils::first_non_bot;
use crate::verification::{qc3_view, verify_qc3};
use anyhow::{ensure, Result};
use aptos_crypto::{bls12381::Signature as BlsSignature, HashValue};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::validator_verifier::ValidatorVerifier;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

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
    /// Epoch number (for cross-epoch replay filtering)
    pub epoch: u64,
    /// Slot number (for cross-slot replay filtering)
    pub slot: u64,
}

impl EmptyViewMessage {
    pub fn new(
        empty_view: u64,
        author: PartyId,
        highest_known_view: u64,
        highest_known_proof: QC3,
        signature: BlsSignature,
        epoch: u64,
        slot: u64,
    ) -> Self {
        Self {
            statement: EmptyViewStatement::new(empty_view, highest_known_view),
            author,
            highest_known_proof,
            signature,
            epoch,
            slot,
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
    ) -> Option<Self> {
        if messages.is_empty() {
            return None;
        }

        // Check that message authors have sufficient voting power (>1/3 stake)
        let authors: Vec<_> = messages.iter().map(|m| m.author).collect();
        if verifier
            .check_voting_power(authors.iter(), false)
            .is_err()
        {
            return None; // Not enough stake yet
        }

        // Find message with max highest_known_view
        let best_message = messages
            .iter()
            .max_by_key(|m| m.highest_known_view())
            .unwrap();

        Some(Self {
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
        epoch: u64,
        slot: u64,
    ) -> EmptyViewMessage {
        EmptyViewMessage::new(
            empty_view,
            author,
            self.view,
            self.proof.clone(),
            signature,
            epoch,
            slot,
        )
    }
}

// ============================================================================
// Shared Helper Functions (used by StrongPCCommit and trace-back in Phase 4)
// ============================================================================

/// Check if a certificate reaches View 1 (terminal condition for trace-back).
///
/// Uses `parent_view()` rather than `view()` because the last certificate in a
/// chain can be either:
///
/// - **`DirectCert(view=1)`**: Created by View 1's PC. `parent_view() == view() == 1`.
///   Its `v_high()` is View 1's raw transaction output.
///
/// - **`IndirectCert(empty_view=V, parent=1)`**: Created when some view V > 1 was
///   empty and the best known non-empty view was View 1. `parent_view() == 1` but
///   `view() == V ≠ 1`. Its `v_high()` is also View 1's output (derived from
///   `parent_proof`, which is View 1's QC3).
///
/// When the chain reaches such a terminal cert, we cannot follow `v_high` further
/// because it contains raw transaction hashes (not certificate hashes). This is
/// the stopping condition.
///
/// Note: View 1 itself always creates a DirectCertificate (never Indirect), but
/// the chain may trace to an IndirectCert that *points to* View 1 without passing
/// through DirectCert(V=1) — this happens when an intermediate view was empty.
pub fn cert_reaches_view1(cert: &Certificate) -> bool {
    cert.parent_view() == 1
}

// ============================================================================
// Strong Prefix Consensus Commit Message
// ============================================================================

/// Error types for StrongPCCommit verification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrongPCCommitError {
    /// Committing proof (QC3) is invalid
    InvalidCommittingProof(String),
    /// v_low derived from committing proof has no non-⊥ entry
    NoCommitInVLow,
    /// v_low's first non-⊥ doesn't match chain[0]'s hash
    VLowChainMismatch {
        expected: HashValue,
        got: HashValue,
    },
    /// Certificate chain is empty
    EmptyChain,
    /// Certificate validation failed (bad signatures)
    InvalidCertificate { view: u64, reason: String },
    /// Hash linkage broken: first non-⊥ in v_high doesn't match next cert's hash
    ChainLinkageMismatch {
        position: usize,
        expected: HashValue,
        got: HashValue,
    },
    /// Certificate's v_high has no non-⊥ entry but cert isn't terminal
    NoNextCertInVHigh { position: usize },
    /// Last certificate doesn't reach View 1
    DoesNotReachView1 { final_parent_view: u64 },
    /// Claimed v_high doesn't match derived v_high
    VHighMismatch,
}

impl fmt::Display for StrongPCCommitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCommittingProof(reason) => {
                write!(f, "Invalid committing proof (QC3): {}", reason)
            }
            Self::NoCommitInVLow => {
                write!(f, "v_low from committing proof has no non-⊥ entry")
            }
            Self::VLowChainMismatch { expected, got } => {
                write!(
                    f,
                    "v_low first non-⊥ hash {:?} does not match chain[0] hash {:?}",
                    expected, got
                )
            }
            Self::EmptyChain => {
                write!(f, "Certificate chain is empty")
            }
            Self::InvalidCertificate { view, reason } => {
                write!(f, "Invalid certificate at view {}: {}", view, reason)
            }
            Self::ChainLinkageMismatch {
                position,
                expected,
                got,
            } => {
                write!(
                    f,
                    "Chain linkage mismatch at position {}: v_high hash {:?} != next cert hash {:?}",
                    position, expected, got
                )
            }
            Self::NoNextCertInVHigh { position } => {
                write!(
                    f,
                    "Certificate at position {} has no non-⊥ entry in v_high but is not terminal",
                    position
                )
            }
            Self::DoesNotReachView1 { final_parent_view } => {
                write!(
                    f,
                    "Chain does not reach View 1: last cert's parent_view is {}",
                    final_parent_view
                )
            }
            Self::VHighMismatch => {
                write!(f, "Claimed v_high does not match derived v_high from chain")
            }
        }
    }
}

impl std::error::Error for StrongPCCommitError {}

/// Commit announcement with full proof chain for termination
///
/// When a party commits (v_low in some view > 1 has a non-⊥ entry that traces
/// back to View 1), it broadcasts this message so other parties can verify the
/// chain and terminate with the same output.
///
/// ## Verification Flow
///
/// 1. Validate `committing_proof` (QC3 from the committing view)
/// 2. Derive `v_low` from committing_proof, find first non-⊥ → must match `hash(chain[0])`
/// 3. For each consecutive pair: `first_non_bot(chain[i].v_high()) == hash(chain[i+1])`
/// 4. Last cert must reach View 1 (`parent_view() == 1`)
/// 5. `v_high` must match last cert's `v_high()`
///
/// ## Example
///
/// Party commits at View 5, chain traces View 3 → View 2 → View 1:
/// ```text
/// committing_proof = QC3 from View 5
/// chain[0] = DirectCert(V=3)   // hash found in View 5's v_low
/// chain[1] = DirectCert(V=2)   // hash found in chain[0].v_high()
/// chain[2] = DirectCert(V=1)   // hash found in chain[1].v_high() — terminal
/// v_high = chain[2].v_high()   // View 1's raw transaction output
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StrongPCCommit {
    /// QC3 from the committing view — proves v_low had a non-⊥ entry
    pub committing_proof: QC3,

    /// Chain of certificates traced from v_low back to View 1
    ///
    /// - `chain[0]` = cert referenced by first non-⊥ in v_low
    /// - `chain[i+1]` = cert referenced by first non-⊥ in `chain[i].v_high()`
    /// - `chain[k]` = cert that reaches View 1 (terminal)
    pub certificate_chain: Vec<Certificate>,

    /// The final Strong PC v_high output (View 1's v_high)
    pub v_high: PrefixVector,

    /// Epoch for validation
    pub epoch: u64,

    /// Slot number for multi-slot consensus
    pub slot: u64,
}

impl StrongPCCommit {
    /// Create a new commit message
    pub fn new(
        committing_proof: QC3,
        certificate_chain: Vec<Certificate>,
        v_high: PrefixVector,
        epoch: u64,
        slot: u64,
    ) -> Self {
        Self {
            committing_proof,
            certificate_chain,
            v_high,
            epoch,
            slot,
        }
    }

    /// Get the committing view from the committing proof's votes.
    ///
    /// Returns `None` if the QC3 has no votes (which is invalid).
    pub fn committing_view(&self) -> Option<u64> {
        self.committing_proof.votes.first().map(|v| v.view)
    }

    /// Verify the commit message
    ///
    /// Validates the full chain: committing_proof → v_low → chain[0] → v_high
    /// linkage → ... → View 1 → v_high output.
    pub fn verify(&self, verifier: &ValidatorVerifier) -> Result<(), StrongPCCommitError> {
        // 1. Validate committing proof (QC3 signatures)
        verify_qc3(&self.committing_proof, verifier).map_err(|e| {
            StrongPCCommitError::InvalidCommittingProof(e.to_string())
        })?;

        // 2. Derive v_low from committing proof
        let (v_low, _) = qc3_certify(&self.committing_proof);

        // 3. Find first non-⊥ in v_low (this is the commit condition)
        let start_hash = first_non_bot(&v_low).ok_or(StrongPCCommitError::NoCommitInVLow)?;

        // 4. Chain must be non-empty
        if self.certificate_chain.is_empty() {
            return Err(StrongPCCommitError::EmptyChain);
        }

        // 5. v_low's first non-⊥ must match chain[0]'s hash
        let chain_start_hash = self.certificate_chain[0].hash();
        if start_hash != chain_start_hash {
            return Err(StrongPCCommitError::VLowChainMismatch {
                expected: start_hash,
                got: chain_start_hash,
            });
        }

        // 6. Validate each certificate and check v_high hash linkage
        for (i, cert) in self.certificate_chain.iter().enumerate() {
            // Validate certificate signatures
            cert.validate(verifier).map_err(|e| {
                StrongPCCommitError::InvalidCertificate {
                    view: cert.view(),
                    reason: e.to_string(),
                }
            })?;

            // For non-last certs: v_high must link to next cert
            if i < self.certificate_chain.len() - 1 {
                let v_high = cert.v_high();
                let next_hash = first_non_bot(&v_high).ok_or(
                    StrongPCCommitError::NoNextCertInVHigh { position: i },
                )?;
                let actual_next = self.certificate_chain[i + 1].hash();
                if next_hash != actual_next {
                    return Err(StrongPCCommitError::ChainLinkageMismatch {
                        position: i,
                        expected: next_hash,
                        got: actual_next,
                    });
                }
            }
        }

        // 7. Last certificate must reach View 1
        let last_cert = self.certificate_chain.last().unwrap();
        if !cert_reaches_view1(last_cert) {
            return Err(StrongPCCommitError::DoesNotReachView1 {
                final_parent_view: last_cert.parent_view(),
            });
        }

        // 8. Claimed v_high must match last cert's v_high
        let derived_v_high = last_cert.v_high();
        if derived_v_high != self.v_high {
            return Err(StrongPCCommitError::VHighMismatch);
        }

        Ok(())
    }

    /// Returns the message type name for logging
    pub fn name(&self) -> &str {
        "StrongPCCommit"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Vote1, Vote2, Vote3, QC1, QC2};
    use aptos_types::account_address::AccountAddress;
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
            1,
            0,
        );

        assert_eq!(msg.empty_view(), 5);
        assert_eq!(msg.highest_known_view(), 3);
        assert_eq!(msg.author, dummy_party_id(0));
        assert_eq!(msg.epoch, 1);
        assert_eq!(msg.slot, 0);
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
            EmptyViewMessage::new(5, dummy_party_id(0), 2, qc3_view2.clone(), dummy_signature(), 1, 0),
            EmptyViewMessage::new(5, dummy_party_id(1), 3, qc3_view3.clone(), dummy_signature(), 1, 0),
            EmptyViewMessage::new(5, dummy_party_id(2), 2, qc3_view2, dummy_signature(), 1, 0),
        ];

        let cert = IndirectCertificate::from_messages(5, messages, &verifier).unwrap();

        assert_eq!(cert.view(), 5);
        // Should select MAX = 3
        assert_eq!(cert.parent_view(), 3);
    }

    #[test]
    fn test_indirect_certificate_empty_messages_returns_none() {
        let verifier = create_test_verifier(4);
        let result = IndirectCertificate::from_messages(5, vec![], &verifier);
        assert!(result.is_none());
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
            EmptyViewMessage::new(5, dummy_party_id(0), 3, qc3.clone(), dummy_signature(), 1, 0),
            EmptyViewMessage::new(5, dummy_party_id(1), 3, qc3.clone(), dummy_signature(), 1, 0),
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
            EmptyViewMessage::new(5, dummy_party_id(0), 3, qc3.clone(), dummy_signature(), 1, 0),
            EmptyViewMessage::new(5, dummy_party_id(1), 3, qc3, dummy_signature(), 1, 0),
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
            EmptyViewMessage::new(5, dummy_party_id(0), 3, qc3.clone(), dummy_signature(), 1, 0),
            EmptyViewMessage::new(5, dummy_party_id(1), 3, qc3, dummy_signature(), 1, 0),
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
            1,
            0,
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
            EmptyViewMessage::new(5, dummy_party_id(0), 3, qc3.clone(), dummy_signature(), 1, 0),
            EmptyViewMessage::new(5, dummy_party_id(1), 3, qc3, dummy_signature(), 1, 0),
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

    // ========================================================================
    // StrongPCCommit Tests
    // ========================================================================

    /// Helper: create a dummy QC3 with empty votes (for constructor/serialization tests only)
    fn dummy_qc3() -> QC3 {
        QC3::new(vec![])
    }

    /// Helper: create a dummy QC3 with a vote at a given view
    fn dummy_qc3_with_view(view: u64) -> QC3 {
        let vote = Vote3::new(
            AccountAddress::random(),
            vec![HashValue::random()],
            QC2 { votes: vec![], authors: vec![] },
            1,
            0,
            view,
            dummy_signature(),
        );
        QC3::new(vec![vote])
    }

    #[test]
    fn test_strong_pc_commit_new() {
        let qc3 = dummy_qc3();
        let v_high = vec![HashValue::random()];
        let commit = StrongPCCommit::new(qc3, vec![], v_high.clone(), 5, 0);

        assert_eq!(commit.v_high, v_high);
        assert!(commit.certificate_chain.is_empty());
        assert_eq!(commit.epoch, 5);
        assert_eq!(commit.slot, 0);
        assert_eq!(commit.name(), "StrongPCCommit");
    }

    #[test]
    fn test_strong_pc_commit_committing_view() {
        let qc3 = dummy_qc3_with_view(5);
        let v_high = vec![HashValue::random()];
        let commit = StrongPCCommit::new(qc3, vec![], v_high, 1, 0);

        assert_eq!(commit.committing_view(), Some(5));
    }

    #[test]
    fn test_strong_pc_commit_committing_view_empty_qc3() {
        let qc3 = dummy_qc3();
        let v_high = vec![HashValue::random()];
        let commit = StrongPCCommit::new(qc3, vec![], v_high, 1, 0);

        // Empty QC3 has no votes → None
        assert_eq!(commit.committing_view(), None);
    }

    #[test]
    fn test_strong_pc_commit_serialization_roundtrip() {
        let qc3 = dummy_qc3_with_view(3);
        let v_high = vec![HashValue::random(), HashValue::random()];
        let commit = StrongPCCommit::new(qc3, vec![], v_high, 1, 42);

        let serialized = bcs::to_bytes(&commit).expect("Serialization should succeed");
        let recovered: StrongPCCommit =
            bcs::from_bytes(&serialized).expect("Deserialization should succeed");

        assert_eq!(commit, recovered);
    }

    #[test]
    fn test_strong_pc_commit_error_display() {
        let err = StrongPCCommitError::EmptyChain;
        let display = format!("{}", err);
        assert!(display.contains("empty"));

        let err = StrongPCCommitError::InvalidCommittingProof("bad sig".into());
        let display = format!("{}", err);
        assert!(display.contains("bad sig"));

        let err = StrongPCCommitError::NoCommitInVLow;
        let display = format!("{}", err);
        assert!(display.contains("non-⊥"));

        let err = StrongPCCommitError::VLowChainMismatch {
            expected: HashValue::zero(),
            got: HashValue::zero(),
        };
        let display = format!("{}", err);
        assert!(display.contains("chain[0]"));

        let err = StrongPCCommitError::ChainLinkageMismatch {
            position: 1,
            expected: HashValue::zero(),
            got: HashValue::zero(),
        };
        let display = format!("{}", err);
        assert!(display.contains("position 1"));

        let err = StrongPCCommitError::NoNextCertInVHigh { position: 2 };
        let display = format!("{}", err);
        assert!(display.contains("position 2"));

        let err = StrongPCCommitError::DoesNotReachView1 { final_parent_view: 7 };
        let display = format!("{}", err);
        assert!(display.contains("7"));
        assert!(display.contains("View 1"));

        let err = StrongPCCommitError::VHighMismatch;
        let display = format!("{}", err);
        assert!(display.contains("v_high"));
    }
}
