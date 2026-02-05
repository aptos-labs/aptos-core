// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Core types for Prefix Consensus protocol

use aptos_crypto::{
    bls12381::Signature as BlsSignature,
    hash::{CryptoHash, CryptoHasher},
    HashValue,
};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Define custom hashers for Vote types using the same pattern as aptos-crypto
macro_rules! define_vote_hasher {
    ($hasher_type:ident, $salt:expr) => {
        #[derive(Clone)]
        pub struct $hasher_type(sha3::Sha3_256);

        impl $hasher_type {
            fn new() -> Self {
                let mut hasher = sha3::Sha3_256::default();
                if !$salt.is_empty() {
                    use sha3::Digest;
                    hasher.update($salt);
                }
                Self(hasher)
            }
        }

        impl Default for $hasher_type {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::io::Write for $hasher_type {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                use sha3::Digest;
                self.0.update(bytes);
                Ok(bytes.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        impl CryptoHasher for $hasher_type {
            fn seed() -> &'static [u8; 32] {
                use once_cell::sync::Lazy;
                static SEED: Lazy<[u8; 32]> = Lazy::new(|| {
                    let mut hasher = sha3::Sha3_256::default();
                    use sha3::Digest;
                    hasher.update($salt);
                    let hash = hasher.finalize();
                    let mut seed = [0u8; 32];
                    seed.copy_from_slice(hash.as_ref());
                    seed
                });
                &SEED
            }

            fn update(&mut self, bytes: &[u8]) {
                use sha3::Digest;
                self.0.update(bytes);
            }

            fn finish(self) -> HashValue {
                use sha3::Digest;
                HashValue::from_slice(&self.0.finalize()[..]).unwrap()
            }
        }
    };
}

define_vote_hasher!(Vote1Hasher, b"PrefixConsensus::Vote1");
define_vote_hasher!(Vote2Hasher, b"PrefixConsensus::Vote2");
define_vote_hasher!(Vote3Hasher, b"PrefixConsensus::Vote3");

/// Represents a party's unique identifier (validator account address)
pub type PartyId = aptos_types::account_address::AccountAddress;

/// Round number in the protocol (1, 2, or 3)
pub type Round = u8;

// ============================================================================
// Signable Data Types (for creating signatures, excludes signature field)
// ============================================================================

/// Signable data for Vote1 (excludes signature)
#[derive(Clone, Debug, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct Vote1SignData {
    pub author: PartyId,
    pub input_vector: PrefixVector,
    pub epoch: u64,
    pub slot: u64,
}

/// Signable data for Vote2 (excludes signature)
#[derive(Clone, Debug, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct Vote2SignData {
    pub author: PartyId,
    pub certified_prefix: PrefixVector,
    pub qc1: QC1,
    pub epoch: u64,
    pub slot: u64,
}

/// Signable data for Vote3 (excludes signature)
#[derive(Clone, Debug, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct Vote3SignData {
    pub author: PartyId,
    pub mcp_prefix: PrefixVector,
    pub qc2: QC2,
    pub epoch: u64,
    pub slot: u64,
}

/// Generic vector element type for prefix consensus
/// In practice, this could be transaction hashes, block hashes, or other values
pub type Element = HashValue;

/// Vector of elements used in prefix consensus
pub type PrefixVector = Vec<Element>;

// ============================================================================
// Vote Types
// ============================================================================

/// Vote in Round 1: Party votes on their input vector
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Vote1 {
    /// The party casting this vote
    pub author: PartyId,

    /// The input vector this party is voting for
    pub input_vector: PrefixVector,

    /// Epoch number (for future use)
    pub epoch: u64,

    /// Slot number (for future multi-slot, always 0 for single-shot)
    pub slot: u64,

    /// Signature over the vote data (not included in hash)
    pub signature: BlsSignature,
}

impl Vote1 {
    /// Create a new Round 1 vote
    pub fn new(
        author: PartyId,
        input_vector: PrefixVector,
        epoch: u64,
        slot: u64,
        signature: BlsSignature,
    ) -> Self {
        Self {
            author,
            input_vector,
            epoch,
            slot,
            signature,
        }
    }
}

/// Manual CryptoHash implementation that excludes the signature field
impl CryptoHash for Vote1 {
    type Hasher = Vote1Hasher;

    fn hash(&self) -> HashValue {
        let mut state = Self::Hasher::default();
        state.update(&bcs::to_bytes(&self.author).expect("Serialization should not fail"));
        state.update(&bcs::to_bytes(&self.input_vector).expect("Serialization should not fail"));
        state.update(&bcs::to_bytes(&self.epoch).expect("Serialization should not fail"));
        state.update(&bcs::to_bytes(&self.slot).expect("Serialization should not fail"));
        // Note: signature is intentionally NOT included in the hash
        state.finish()
    }
}

/// Vote in Round 2: Party votes on certified prefix from QC1
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Vote2 {
    /// The party casting this vote
    pub author: PartyId,

    /// The certified prefix extracted from QC1
    pub certified_prefix: PrefixVector,

    /// The QC1 that certifies this prefix
    pub qc1: QC1,

    /// Epoch number (for future use)
    pub epoch: u64,

    /// Slot number (for future multi-slot, always 0 for single-shot)
    pub slot: u64,

    /// Signature over the vote data (not included in hash)
    pub signature: BlsSignature,
}

impl Vote2 {
    /// Create a new Round 2 vote
    pub fn new(
        author: PartyId,
        certified_prefix: PrefixVector,
        qc1: QC1,
        epoch: u64,
        slot: u64,
        signature: BlsSignature,
    ) -> Self {
        Self {
            author,
            certified_prefix,
            qc1,
            epoch,
            slot,
            signature,
        }
    }
}

/// Manual CryptoHash implementation that excludes the signature field
impl CryptoHash for Vote2 {
    type Hasher = Vote2Hasher;

    fn hash(&self) -> HashValue {
        let mut state = Self::Hasher::default();
        state.update(&bcs::to_bytes(&self.author).expect("Serialization should not fail"));
        state.update(&bcs::to_bytes(&self.certified_prefix).expect("Serialization should not fail"));
        state.update(&bcs::to_bytes(&self.qc1).expect("Serialization should not fail"));
        state.update(&bcs::to_bytes(&self.epoch).expect("Serialization should not fail"));
        state.update(&bcs::to_bytes(&self.slot).expect("Serialization should not fail"));
        // Note: signature is intentionally NOT included in the hash
        state.finish()
    }
}

/// Vote in Round 3: Party votes on the maximum common prefix from QC2
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Vote3 {
    /// The party casting this vote
    pub author: PartyId,

    /// The maximum common prefix from QC2
    pub mcp_prefix: PrefixVector,

    /// The QC2 that certifies this prefix
    pub qc2: QC2,

    /// Epoch number (for future use)
    pub epoch: u64,

    /// Slot number (for future multi-slot, always 0 for single-shot)
    pub slot: u64,

    /// Signature over the vote data (not included in hash)
    pub signature: BlsSignature,
}

impl Vote3 {
    /// Create a new Round 3 vote
    pub fn new(
        author: PartyId,
        mcp_prefix: PrefixVector,
        qc2: QC2,
        epoch: u64,
        slot: u64,
        signature: BlsSignature,
    ) -> Self {
        Self {
            author,
            mcp_prefix,
            qc2,
            epoch,
            slot,
            signature,
        }
    }
}

/// Manual CryptoHash implementation that excludes the signature field
impl CryptoHash for Vote3 {
    type Hasher = Vote3Hasher;

    fn hash(&self) -> HashValue {
        let mut state = Self::Hasher::default();
        state.update(&bcs::to_bytes(&self.author).expect("Serialization should not fail"));
        state.update(&bcs::to_bytes(&self.mcp_prefix).expect("Serialization should not fail"));
        state.update(&bcs::to_bytes(&self.qc2).expect("Serialization should not fail"));
        state.update(&bcs::to_bytes(&self.epoch).expect("Serialization should not fail"));
        state.update(&bcs::to_bytes(&self.slot).expect("Serialization should not fail"));
        // Note: signature is intentionally NOT included in the hash
        state.finish()
    }
}

// ============================================================================
// Quorum Certificate Types
// ============================================================================

/// Quorum Certificate from Round 1: Collection of n-f Vote1 messages
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct QC1 {
    /// The votes included in this QC (at least n-f votes)
    pub votes: Vec<Vote1>,

    /// Set of authors who contributed votes (for quick lookup)
    pub authors: Vec<PartyId>,
}

impl QC1 {
    /// Create a new QC1 from a collection of votes
    pub fn new(votes: Vec<Vote1>) -> Self {
        let authors = votes.iter().map(|v| v.author).collect();
        Self { votes, authors }
    }

    /// Get the number of votes in this QC
    pub fn vote_count(&self) -> usize {
        self.votes.len()
    }

    /// Get the hash of this QC
    pub fn hash(&self) -> HashValue {
        let bytes = bcs::to_bytes(&self.votes).unwrap();
        HashValue::sha3_256_of(&bytes)
    }

    /// Check if this QC contains a vote from the given author
    pub fn contains_author(&self, author: &PartyId) -> bool {
        self.authors.contains(author)
    }
}

/// Quorum Certificate from Round 2: Collection of n-f Vote2 messages
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct QC2 {
    /// The votes included in this QC (at least n-f votes)
    pub votes: Vec<Vote2>,

    /// Set of authors who contributed votes
    pub authors: Vec<PartyId>,
}

impl QC2 {
    /// Create a new QC2 from a collection of votes
    pub fn new(votes: Vec<Vote2>) -> Self {
        let authors = votes.iter().map(|v| v.author).collect();
        Self { votes, authors }
    }

    /// Get the number of votes in this QC
    pub fn vote_count(&self) -> usize {
        self.votes.len()
    }

    /// Get the hash of this QC
    pub fn hash(&self) -> HashValue {
        let bytes = bcs::to_bytes(&self.votes).unwrap();
        HashValue::sha3_256_of(&bytes)
    }

    /// Check if this QC contains a vote from the given author
    pub fn contains_author(&self, author: &PartyId) -> bool {
        self.authors.contains(author)
    }
}

/// Quorum Certificate from Round 3: Collection of n-f Vote3 messages
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct QC3 {
    /// The votes included in this QC (at least n-f votes)
    pub votes: Vec<Vote3>,

    /// Set of authors who contributed votes
    pub authors: Vec<PartyId>,
}

impl QC3 {
    /// Create a new QC3 from a collection of votes
    pub fn new(votes: Vec<Vote3>) -> Self {
        let authors = votes.iter().map(|v| v.author).collect();
        Self { votes, authors }
    }

    /// Get the number of votes in this QC
    pub fn vote_count(&self) -> usize {
        self.votes.len()
    }

    /// Get the hash of this QC
    pub fn hash(&self) -> HashValue {
        let bytes = bcs::to_bytes(&self.votes).unwrap();
        HashValue::sha3_256_of(&bytes)
    }

    /// Check if this QC contains a vote from the given author
    pub fn contains_author(&self, author: &PartyId) -> bool {
        self.authors.contains(author)
    }
}

// ============================================================================
// Input/Output Types
// ============================================================================

/// Input to the Prefix Consensus protocol
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrefixConsensusInput {
    /// The party's input vector
    pub input_vector: PrefixVector,

    /// The party's identity
    pub party_id: PartyId,

    /// Total number of parties
    pub n: usize,

    /// Maximum number of Byzantine parties
    pub f: usize,

    /// Epoch number (for future use)
    pub epoch: u64,
}

impl PrefixConsensusInput {
    /// Create a new input for prefix consensus
    pub fn new(
        input_vector: PrefixVector,
        party_id: PartyId,
        n: usize,
        f: usize,
        epoch: u64,
    ) -> Self {
        Self {
            input_vector,
            party_id,
            n,
            f,
            epoch,
        }
    }

    /// Get the quorum size (n-f)
    pub fn quorum_size(&self) -> usize {
        self.n - self.f
    }
}

/// Output from the Prefix Consensus protocol
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrefixConsensusOutput {
    /// v_low: The maximum common prefix (safe to commit)
    pub v_low: PrefixVector,

    /// v_high: The minimum common extension (safe to extend)
    pub v_high: PrefixVector,

    /// The final QC3 certificate
    pub qc3: QC3,
}

impl PrefixConsensusOutput {
    /// Create a new output
    pub fn new(v_low: PrefixVector, v_high: PrefixVector, qc3: QC3) -> Self {
        Self { v_low, v_high, qc3 }
    }

    /// Verify the upper bound property: v_low ⪯ v_high
    pub fn verify_upper_bound(&self) -> bool {
        // v_low should be a prefix of v_high
        if self.v_low.len() > self.v_high.len() {
            return false;
        }
        self.v_low
            .iter()
            .zip(self.v_high.iter())
            .all(|(a, b)| a == b)
    }

    /// Verify that v_low and v_high are correctly derived from the QC3 proof
    ///
    /// This function checks that:
    /// - v_low equals the maximum common prefix (mcp) of all mcp_prefix values in QC3
    /// - v_high equals the minimum common extension (mce) of all mcp_prefix values in QC3
    ///
    /// **WARNING**: This method does NOT verify the QC3 structure itself (quorum size,
    /// signatures, etc.). For complete verification, use `verify()` instead.
    ///
    /// Returns true only if BOTH proofs are valid.
    pub fn verify_proofs(&self) -> bool {
        use crate::certify::qc3_certify;
        let (computed_v_low, computed_v_high) = qc3_certify(&self.qc3);
        self.v_low == computed_v_low && self.v_high == computed_v_high
    }

    /// Verify the complete output including QC3 validity
    ///
    /// This performs complete verification suitable for external parties:
    /// 1. Verifies QC3 structure (quorum size, no duplicate authors, all signatures)
    /// 2. Verifies v_low and v_high are correctly derived from QC3
    /// 3. Verifies upper bound property (v_low ⪯ v_high)
    ///
    /// # Arguments
    /// * `f` - Maximum number of Byzantine faults tolerated
    /// * `n` - Total number of validators
    /// * `verifier` - Validator verifier for signature checks
    pub fn verify(
        &self,
        f: usize,
        n: usize,
        verifier: &aptos_types::validator_verifier::ValidatorVerifier,
    ) -> anyhow::Result<()> {
        use crate::verification::verify_qc3;
        use anyhow::ensure;

        // 1. Verify QC3 structure is valid (including all signatures)
        verify_qc3(&self.qc3, f, n, verifier)?;

        // 2. Verify proofs (v_low/v_high match QC3)
        ensure!(
            self.verify_proofs(),
            "Output proofs invalid: v_low or v_high don't match QC3 derivation"
        );

        // 3. Verify upper bound property
        ensure!(
            self.verify_upper_bound(),
            "Upper bound property violated: v_low is not a prefix of v_high"
        );

        Ok(())
    }
}

// ============================================================================
// Pending Vote Collections
// ============================================================================

/// Collection of pending Vote1 messages
#[derive(Default)]
pub struct PendingVotes1 {
    votes: HashMap<PartyId, Vote1>,
}

impl PendingVotes1 {
    pub fn new() -> Self {
        Self {
            votes: HashMap::new(),
        }
    }

    /// Add a vote to the pending collection
    pub fn add_vote(&mut self, vote: Vote1) -> bool {
        if self.votes.contains_key(&vote.author) {
            return false; // Duplicate vote
        }
        self.votes.insert(vote.author, vote);
        true
    }

    /// Get the current vote count
    pub fn vote_count(&self) -> usize {
        self.votes.len()
    }

    /// Check if we have a quorum (n-f votes)
    pub fn has_quorum(&self, quorum_size: usize) -> bool {
        self.vote_count() >= quorum_size
    }

    /// Extract all votes as a QC1
    pub fn into_qc1(self) -> QC1 {
        let votes: Vec<Vote1> = self.votes.into_values().collect();
        QC1::new(votes)
    }
}

/// Collection of pending Vote2 messages
#[derive(Default)]
pub struct PendingVotes2 {
    votes: HashMap<PartyId, Vote2>,
}

impl PendingVotes2 {
    pub fn new() -> Self {
        Self {
            votes: HashMap::new(),
        }
    }

    pub fn add_vote(&mut self, vote: Vote2) -> bool {
        if self.votes.contains_key(&vote.author) {
            return false;
        }
        self.votes.insert(vote.author, vote);
        true
    }

    pub fn vote_count(&self) -> usize {
        self.votes.len()
    }

    pub fn has_quorum(&self, quorum_size: usize) -> bool {
        self.vote_count() >= quorum_size
    }

    pub fn into_qc2(self) -> QC2 {
        let votes: Vec<Vote2> = self.votes.into_values().collect();
        QC2::new(votes)
    }
}

/// Collection of pending Vote3 messages
#[derive(Default)]
pub struct PendingVotes3 {
    votes: HashMap<PartyId, Vote3>,
}

impl PendingVotes3 {
    pub fn new() -> Self {
        Self {
            votes: HashMap::new(),
        }
    }

    pub fn add_vote(&mut self, vote: Vote3) -> bool {
        if self.votes.contains_key(&vote.author) {
            return false;
        }
        self.votes.insert(vote.author, vote);
        true
    }

    pub fn vote_count(&self) -> usize {
        self.votes.len()
    }

    pub fn has_quorum(&self, quorum_size: usize) -> bool {
        self.vote_count() >= quorum_size
    }

    pub fn into_qc3(self) -> QC3 {
        let votes: Vec<Vote3> = self.votes.into_values().collect();
        QC3::new(votes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(i: u64) -> HashValue {
        HashValue::sha3_256_of(&i.to_le_bytes())
    }

    fn dummy_party(i: u8) -> PartyId {
        PartyId::new([i; 32])
    }

    fn dummy_sig() -> BlsSignature {
        BlsSignature::dummy_signature()
    }

    fn create_dummy_qc1() -> QC1 {
        let votes = vec![
            Vote1::new(dummy_party(0), vec![hash(1)], 0, 0, dummy_sig()),
            Vote1::new(dummy_party(1), vec![hash(1)], 0, 0, dummy_sig()),
            Vote1::new(dummy_party(2), vec![hash(1)], 0, 0, dummy_sig()),
        ];
        QC1::new(votes)
    }

    fn create_dummy_qc2() -> QC2 {
        let qc1 = create_dummy_qc1();
        let votes = vec![
            Vote2::new(dummy_party(0), vec![hash(1)], qc1.clone(), 0, 0, dummy_sig()),
            Vote2::new(dummy_party(1), vec![hash(1)], qc1.clone(), 0, 0, dummy_sig()),
            Vote2::new(dummy_party(2), vec![hash(1)], qc1, 0, 0, dummy_sig()),
        ];
        QC2::new(votes)
    }

    #[test]
    fn test_verify_proofs_valid() {
        // Create QC3 with consistent mcp prefixes
        let hash1 = hash(1);
        let hash2 = hash(2);

        let qc2 = create_dummy_qc2();

        // All Vote3s have same mcp_prefix [hash1, hash2]
        let votes = vec![
            Vote3::new(
                dummy_party(0),
                vec![hash1, hash2],
                qc2.clone(),
                0,
                0,
                dummy_sig(),
            ),
            Vote3::new(
                dummy_party(1),
                vec![hash1, hash2],
                qc2.clone(),
                0,
                0,
                dummy_sig(),
            ),
            Vote3::new(dummy_party(2), vec![hash1, hash2], qc2, 0, 0, dummy_sig()),
        ];

        let qc3 = QC3::new(votes);
        let v_low = vec![hash1, hash2]; // mcp of prefixes
        let v_high = vec![hash1, hash2]; // mce of prefixes

        let output = PrefixConsensusOutput::new(v_low, v_high, qc3);

        // Proofs should be valid
        assert!(output.verify_proofs());
    }

    #[test]
    fn test_verify_proofs_invalid_v_low() {
        // Create QC3 with mcp = [hash1]
        let hash1 = hash(1);
        let hash2 = hash(2);

        let qc2 = create_dummy_qc2();
        let votes = vec![
            Vote3::new(dummy_party(0), vec![hash1], qc2.clone(), 0, 0, dummy_sig()),
            Vote3::new(dummy_party(1), vec![hash1], qc2.clone(), 0, 0, dummy_sig()),
            Vote3::new(dummy_party(2), vec![hash1], qc2, 0, 0, dummy_sig()),
        ];

        let qc3 = QC3::new(votes);

        // Claim v_low = [hash1, hash2] (WRONG - should be [hash1])
        let v_low = vec![hash1, hash2];
        let v_high = vec![hash1];

        let output = PrefixConsensusOutput::new(v_low, v_high, qc3);

        // Proof should be invalid
        assert!(!output.verify_proofs());
    }

    #[test]
    fn test_verify_proofs_invalid_v_high() {
        // Create QC3 with mce = [hash1, hash2, hash3]
        let hash1 = hash(1);
        let hash2 = hash(2);
        let hash3 = hash(3);
        let hash4 = hash(4);

        let qc2 = create_dummy_qc2();
        let votes = vec![
            Vote3::new(
                dummy_party(0),
                vec![hash1, hash2],
                qc2.clone(),
                0,
                0,
                dummy_sig(),
            ),
            Vote3::new(
                dummy_party(1),
                vec![hash1, hash2, hash3],
                qc2.clone(),
                0,
                0,
                dummy_sig(),
            ),
            Vote3::new(dummy_party(2), vec![hash1], qc2, 0, 0, dummy_sig()),
        ];

        let qc3 = QC3::new(votes);
        let v_low = vec![hash1]; // Correct mcp

        // Claim v_high = [hash1, hash2, hash3, hash4] (WRONG - should be [hash1, hash2, hash3])
        let v_high = vec![hash1, hash2, hash3, hash4];

        let output = PrefixConsensusOutput::new(v_low, v_high, qc3);

        // Proof should be invalid
        assert!(!output.verify_proofs());
    }

    #[test]
    fn test_verify_proofs_consistent_prefixes() {
        // Test with consistent prefixes (one is prefix of another)
        let hash1 = hash(1);
        let hash2 = hash(2);
        let hash3 = hash(3);

        let qc2 = create_dummy_qc2();

        // Prefixes: [hash1, hash2, hash3], [hash1, hash2], [hash1]
        // All are consistent (each is prefix of the longest)
        // mcp = [hash1] (shortest common prefix)
        // mce = [hash1, hash2, hash3] (longest vector)
        let votes = vec![
            Vote3::new(
                dummy_party(0),
                vec![hash1, hash2, hash3],
                qc2.clone(),
                0,
                0,
                dummy_sig(),
            ),
            Vote3::new(
                dummy_party(1),
                vec![hash1, hash2],
                qc2.clone(),
                0,
                0,
                dummy_sig(),
            ),
            Vote3::new(dummy_party(2), vec![hash1], qc2, 0, 0, dummy_sig()),
        ];

        let qc3 = QC3::new(votes);
        let v_low = vec![hash1];
        let v_high = vec![hash1, hash2, hash3];

        let output = PrefixConsensusOutput::new(v_low, v_high, qc3);

        // Proofs should be valid
        assert!(output.verify_proofs());
    }

    #[test]
    fn test_verify_complete() {
        // Test complete verification including QC3
        let hash1 = hash(1);
        let hash2 = hash(2);

        let qc2 = create_dummy_qc2();

        // All Vote3s have same mcp_prefix [hash1, hash2]
        let votes = vec![
            Vote3::new(
                dummy_party(0),
                vec![hash1, hash2],
                qc2.clone(),
                0,
                0,
                dummy_sig(),
            ),
            Vote3::new(
                dummy_party(1),
                vec![hash1, hash2],
                qc2.clone(),
                0,
                0,
                dummy_sig(),
            ),
            Vote3::new(dummy_party(2), vec![hash1, hash2], qc2, 0, 0, dummy_sig()),
        ];

        let qc3 = QC3::new(votes);
        let v_low = vec![hash1, hash2];
        let v_high = vec![hash1, hash2];

        let output = PrefixConsensusOutput::new(v_low, v_high, qc3);

        // Create validator verifier with dummy keys (won't match dummy signatures)
        use aptos_types::validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier};
        let validator_infos: Vec<_> = (0..4)
            .map(|i| {
                let signer = aptos_types::validator_signer::ValidatorSigner::random(None);
                ValidatorConsensusInfo::new(dummy_party(i), signer.public_key(), 1)
            })
            .collect();
        let verifier = ValidatorVerifier::new(validator_infos);

        // Complete verification will fail due to invalid dummy signatures
        assert!(output.verify(1, 4, &verifier).is_err());
    }

    #[test]
    fn test_verify_complete_insufficient_votes() {
        // Test that verify() catches insufficient quorum
        let hash1 = hash(1);

        let qc2 = create_dummy_qc2();

        // Only 2 votes (insufficient for n=4, f=1 which requires 3)
        let votes = vec![
            Vote3::new(dummy_party(0), vec![hash1], qc2.clone(), 0, 0, dummy_sig()),
            Vote3::new(dummy_party(1), vec![hash1], qc2, 0, 0, dummy_sig()),
        ];

        let qc3 = QC3::new(votes);
        let v_low = vec![hash1];
        let v_high = vec![hash1];

        let output = PrefixConsensusOutput::new(v_low, v_high, qc3);

        // Create validator verifier
        use aptos_types::validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier};
        let validator_infos: Vec<_> = (0..4)
            .map(|i| {
                let signer = aptos_types::validator_signer::ValidatorSigner::random(None);
                ValidatorConsensusInfo::new(dummy_party(i), signer.public_key(), 1)
            })
            .collect();
        let verifier = ValidatorVerifier::new(validator_infos);

        // Complete verification should fail due to insufficient votes
        assert!(output.verify(1, 4, &verifier).is_err());
    }
}
