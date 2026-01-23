// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Core types for Prefix Consensus protocol

use aptos_crypto::{ed25519::Ed25519Signature, HashValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a party's unique identifier (validator account address)
pub type PartyId = aptos_types::account_address::AccountAddress;

/// Round number in the protocol (1, 2, or 3)
pub type Round = u8;

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

    /// Signature over the vote data
    pub signature: Ed25519Signature,
}

impl Vote1 {
    /// Create a new Round 1 vote
    pub fn new(author: PartyId, input_vector: PrefixVector, signature: Ed25519Signature) -> Self {
        Self {
            author,
            input_vector,
            signature,
        }
    }

    /// Get the hash of this vote for signing/verification
    pub fn hash(&self) -> HashValue {
        let mut bytes = bcs::to_bytes(&self.author).unwrap();
        bytes.extend(bcs::to_bytes(&self.input_vector).unwrap());
        bytes.extend(b"VOTE1");
        HashValue::sha3_256_of(&bytes)
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

    /// Signature over the vote data
    pub signature: Ed25519Signature,
}

impl Vote2 {
    /// Create a new Round 2 vote
    pub fn new(
        author: PartyId,
        certified_prefix: PrefixVector,
        qc1: QC1,
        signature: Ed25519Signature,
    ) -> Self {
        Self {
            author,
            certified_prefix,
            qc1,
            signature,
        }
    }

    /// Get the hash of this vote for signing/verification
    pub fn hash(&self) -> HashValue {
        let mut bytes = bcs::to_bytes(&self.author).unwrap();
        bytes.extend(bcs::to_bytes(&self.certified_prefix).unwrap());
        bytes.extend(bcs::to_bytes(&self.qc1.hash()).unwrap());
        bytes.extend(b"VOTE2");
        HashValue::sha3_256_of(&bytes)
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

    /// Signature over the vote data
    pub signature: Ed25519Signature,
}

impl Vote3 {
    /// Create a new Round 3 vote
    pub fn new(
        author: PartyId,
        mcp_prefix: PrefixVector,
        qc2: QC2,
        signature: Ed25519Signature,
    ) -> Self {
        Self {
            author,
            mcp_prefix,
            qc2,
            signature,
        }
    }

    /// Get the hash of this vote for signing/verification
    pub fn hash(&self) -> HashValue {
        let mut bytes = bcs::to_bytes(&self.author).unwrap();
        bytes.extend(bcs::to_bytes(&self.mcp_prefix).unwrap());
        bytes.extend(bcs::to_bytes(&self.qc2.hash()).unwrap());
        bytes.extend(b"VOTE3");
        HashValue::sha3_256_of(&bytes)
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
}

impl PrefixConsensusInput {
    /// Create a new input for prefix consensus
    pub fn new(input_vector: PrefixVector, party_id: PartyId, n: usize, f: usize) -> Self {
        Self {
            input_vector,
            party_id,
            n,
            f,
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
