// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Network message types for Prefix Consensus
//!
//! This module defines the message envelope types that are transmitted over the network
//! between validators during Prefix Consensus execution.

use crate::types::{PartyId, Vote1, Vote2, Vote3};
use serde::{Deserialize, Serialize};

/// Network message type for Prefix Consensus
///
/// This enum wraps the three vote types that can be transmitted between validators.
/// Messages are serialized using BCS encoding and sent over the Aptos network layer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PrefixConsensusMsg {
    /// Round 1 vote message containing input vector
    Vote1Msg(Box<Vote1>),

    /// Round 2 vote message containing certified prefix and QC1
    Vote2Msg(Box<Vote2>),

    /// Round 3 vote message containing mcp prefix and QC2
    Vote3Msg(Box<Vote3>),
}

impl PrefixConsensusMsg {
    /// Returns the message type name for logging and debugging
    pub fn name(&self) -> &str {
        match self {
            PrefixConsensusMsg::Vote1Msg(_) => "Vote1Msg",
            PrefixConsensusMsg::Vote2Msg(_) => "Vote2Msg",
            PrefixConsensusMsg::Vote3Msg(_) => "Vote3Msg",
        }
    }

    /// Extracts the epoch number from the inner vote
    pub fn epoch(&self) -> u64 {
        match self {
            PrefixConsensusMsg::Vote1Msg(vote) => vote.epoch,
            PrefixConsensusMsg::Vote2Msg(vote) => vote.epoch,
            PrefixConsensusMsg::Vote3Msg(vote) => vote.epoch,
        }
    }

    /// Extracts the slot number from the inner vote
    pub fn slot(&self) -> u64 {
        match self {
            PrefixConsensusMsg::Vote1Msg(vote) => vote.slot,
            PrefixConsensusMsg::Vote2Msg(vote) => vote.slot,
            PrefixConsensusMsg::Vote3Msg(vote) => vote.slot,
        }
    }

    /// Extracts the author (sender) of the message
    pub fn author(&self) -> PartyId {
        match self {
            PrefixConsensusMsg::Vote1Msg(vote) => vote.author,
            PrefixConsensusMsg::Vote2Msg(vote) => vote.author,
            PrefixConsensusMsg::Vote3Msg(vote) => vote.author,
        }
    }

    /// Returns a reference to the inner Vote1, if this is a Vote1Msg
    pub fn as_vote1(&self) -> Option<&Vote1> {
        match self {
            PrefixConsensusMsg::Vote1Msg(vote) => Some(vote),
            _ => None,
        }
    }

    /// Returns a reference to the inner Vote2, if this is a Vote2Msg
    pub fn as_vote2(&self) -> Option<&Vote2> {
        match self {
            PrefixConsensusMsg::Vote2Msg(vote) => Some(vote),
            _ => None,
        }
    }

    /// Returns a reference to the inner Vote3, if this is a Vote3Msg
    pub fn as_vote3(&self) -> Option<&Vote3> {
        match self {
            PrefixConsensusMsg::Vote3Msg(vote) => Some(vote),
            _ => None,
        }
    }

    /// Consumes the message and returns the inner Vote1, if this is a Vote1Msg
    pub fn into_vote1(self) -> Option<Vote1> {
        match self {
            PrefixConsensusMsg::Vote1Msg(vote) => Some(*vote),
            _ => None,
        }
    }

    /// Consumes the message and returns the inner Vote2, if this is a Vote2Msg
    pub fn into_vote2(self) -> Option<Vote2> {
        match self {
            PrefixConsensusMsg::Vote2Msg(vote) => Some(*vote),
            _ => None,
        }
    }

    /// Consumes the message and returns the inner Vote3, if this is a Vote3Msg
    pub fn into_vote3(self) -> Option<Vote3> {
        match self {
            PrefixConsensusMsg::Vote3Msg(vote) => Some(*vote),
            _ => None,
        }
    }
}

/// Convenience conversion from Vote1 to PrefixConsensusMsg
impl From<Vote1> for PrefixConsensusMsg {
    fn from(vote: Vote1) -> Self {
        PrefixConsensusMsg::Vote1Msg(Box::new(vote))
    }
}

/// Convenience conversion from Vote2 to PrefixConsensusMsg
impl From<Vote2> for PrefixConsensusMsg {
    fn from(vote: Vote2) -> Self {
        PrefixConsensusMsg::Vote2Msg(Box::new(vote))
    }
}

/// Convenience conversion from Vote3 to PrefixConsensusMsg
impl From<Vote3> for PrefixConsensusMsg {
    fn from(vote: Vote3) -> Self {
        PrefixConsensusMsg::Vote3Msg(Box::new(vote))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::{bls12381::Signature as BlsSignature, HashValue};
    use aptos_types::account_address::AccountAddress;

    fn create_test_vote1() -> Vote1 {
        Vote1::new(
            AccountAddress::random(),
            vec![HashValue::random(), HashValue::random()],
            1, // epoch
            0, // slot
            1, // view (default for standalone)
            BlsSignature::dummy_signature(),
        )
    }

    fn create_test_vote2() -> Vote2 {
        use crate::types::QC1;
        Vote2::new(
            AccountAddress::random(),
            vec![HashValue::random()],
            QC1 {
                votes: vec![],
                authors: vec![],
            }, // Empty QC for testing
            1, // epoch
            0, // slot
            1, // view (default for standalone)
            BlsSignature::dummy_signature(),
        )
    }

    fn create_test_vote3() -> Vote3 {
        use crate::types::QC2;
        Vote3::new(
            AccountAddress::random(),
            vec![HashValue::random()],
            QC2 {
                votes: vec![],
                authors: vec![],
            }, // Empty QC for testing
            1, // epoch
            0, // slot
            1, // view (default for standalone)
            BlsSignature::dummy_signature(),
        )
    }

    #[test]
    fn test_message_name() {
        let vote1 = create_test_vote1();
        let vote2 = create_test_vote2();
        let vote3 = create_test_vote3();

        let msg1 = PrefixConsensusMsg::from(vote1);
        let msg2 = PrefixConsensusMsg::from(vote2);
        let msg3 = PrefixConsensusMsg::from(vote3);

        assert_eq!(msg1.name(), "Vote1Msg");
        assert_eq!(msg2.name(), "Vote2Msg");
        assert_eq!(msg3.name(), "Vote3Msg");
    }

    #[test]
    fn test_message_epoch_extraction() {
        let vote1 = create_test_vote1();
        let epoch = vote1.epoch;
        let msg = PrefixConsensusMsg::from(vote1);
        assert_eq!(msg.epoch(), epoch);
    }

    #[test]
    fn test_message_slot_extraction() {
        let vote1 = create_test_vote1();
        let slot = vote1.slot;
        let msg = PrefixConsensusMsg::from(vote1);
        assert_eq!(msg.slot(), slot);
    }

    #[test]
    fn test_message_author_extraction() {
        let vote1 = create_test_vote1();
        let author = vote1.author;
        let msg = PrefixConsensusMsg::from(vote1);
        assert_eq!(msg.author(), author);
    }

    #[test]
    fn test_as_vote1() {
        let vote1 = create_test_vote1();
        let author = vote1.author;
        let msg = PrefixConsensusMsg::from(vote1);

        let vote_ref = msg.as_vote1().expect("Should be Vote1");
        assert_eq!(vote_ref.author, author);

        assert!(msg.as_vote2().is_none());
        assert!(msg.as_vote3().is_none());
    }

    #[test]
    fn test_as_vote2() {
        let vote2 = create_test_vote2();
        let author = vote2.author;
        let msg = PrefixConsensusMsg::from(vote2);

        let vote_ref = msg.as_vote2().expect("Should be Vote2");
        assert_eq!(vote_ref.author, author);

        assert!(msg.as_vote1().is_none());
        assert!(msg.as_vote3().is_none());
    }

    #[test]
    fn test_as_vote3() {
        let vote3 = create_test_vote3();
        let author = vote3.author;
        let msg = PrefixConsensusMsg::from(vote3);

        let vote_ref = msg.as_vote3().expect("Should be Vote3");
        assert_eq!(vote_ref.author, author);

        assert!(msg.as_vote1().is_none());
        assert!(msg.as_vote2().is_none());
    }

    #[test]
    fn test_into_vote1() {
        let vote1 = create_test_vote1();
        let author = vote1.author;
        let msg = PrefixConsensusMsg::from(vote1);

        let vote = msg.into_vote1().expect("Should be Vote1");
        assert_eq!(vote.author, author);
    }

    #[test]
    fn test_into_vote2() {
        let vote2 = create_test_vote2();
        let author = vote2.author;
        let msg = PrefixConsensusMsg::from(vote2);

        let vote = msg.into_vote2().expect("Should be Vote2");
        assert_eq!(vote.author, author);
    }

    #[test]
    fn test_into_vote3() {
        let vote3 = create_test_vote3();
        let author = vote3.author;
        let msg = PrefixConsensusMsg::from(vote3);

        let vote = msg.into_vote3().expect("Should be Vote3");
        assert_eq!(vote.author, author);
    }

    #[test]
    fn test_vote1_serialization_roundtrip() {
        let vote1 = create_test_vote1();
        let msg = PrefixConsensusMsg::from(vote1);

        // Serialize
        let serialized = bcs::to_bytes(&msg).expect("Serialization should succeed");

        // Deserialize
        let deserialized: PrefixConsensusMsg =
            bcs::from_bytes(&serialized).expect("Deserialization should succeed");

        // Verify
        assert_eq!(msg.name(), deserialized.name());
        assert_eq!(msg.author(), deserialized.author());
        assert_eq!(msg.epoch(), deserialized.epoch());
        assert_eq!(msg.slot(), deserialized.slot());
    }

    #[test]
    fn test_vote2_serialization_roundtrip() {
        let vote2 = create_test_vote2();
        let msg = PrefixConsensusMsg::from(vote2);

        // Serialize
        let serialized = bcs::to_bytes(&msg).expect("Serialization should succeed");

        // Deserialize
        let deserialized: PrefixConsensusMsg =
            bcs::from_bytes(&serialized).expect("Deserialization should succeed");

        // Verify
        assert_eq!(msg.name(), deserialized.name());
        assert_eq!(msg.author(), deserialized.author());
        assert_eq!(msg.epoch(), deserialized.epoch());
    }

    #[test]
    fn test_vote3_serialization_roundtrip() {
        let vote3 = create_test_vote3();
        let msg = PrefixConsensusMsg::from(vote3);

        // Serialize
        let serialized = bcs::to_bytes(&msg).expect("Serialization should succeed");

        // Deserialize
        let deserialized: PrefixConsensusMsg =
            bcs::from_bytes(&serialized).expect("Deserialization should succeed");

        // Verify
        assert_eq!(msg.name(), deserialized.name());
        assert_eq!(msg.author(), deserialized.author());
        assert_eq!(msg.epoch(), deserialized.epoch());
    }

    #[test]
    fn test_vote2_with_full_qc_serialization() {
        // Create a Vote2 with actual QC1 containing votes
        let vote1_1 = create_test_vote1();
        let vote1_2 = create_test_vote1();

        use crate::types::QC1;
        let author1 = vote1_1.author;
        let author2 = vote1_2.author;
        let qc1 = QC1 {
            votes: vec![vote1_1, vote1_2],
            authors: vec![author1, author2],
        };

        let vote2 = Vote2::new(
            AccountAddress::random(),
            vec![HashValue::random()],
            qc1,
            1, // epoch
            0, // slot
            1, // view (default for standalone)
            BlsSignature::dummy_signature(),
        );

        let msg = PrefixConsensusMsg::from(vote2);

        // Serialize
        let serialized = bcs::to_bytes(&msg).expect("Serialization with full QC1 should succeed");

        // Deserialize
        let deserialized: PrefixConsensusMsg = bcs::from_bytes(&serialized)
            .expect("Deserialization with full QC1 should succeed");

        // Verify QC1 was preserved
        let vote2_back = deserialized.as_vote2().expect("Should be Vote2");
        assert_eq!(vote2_back.qc1.votes.len(), 2);
    }

    #[test]
    fn test_vote3_with_full_qc_serialization() {
        // Create a Vote3 with actual QC2 containing votes
        let vote2_1 = create_test_vote2();
        let vote2_2 = create_test_vote2();

        use crate::types::QC2;
        let author1 = vote2_1.author;
        let author2 = vote2_2.author;
        let qc2 = QC2 {
            votes: vec![vote2_1, vote2_2],
            authors: vec![author1, author2],
        };

        let vote3 = Vote3::new(
            AccountAddress::random(),
            vec![HashValue::random()],
            qc2,
            1, // epoch
            0, // slot
            1, // view (default for standalone)
            BlsSignature::dummy_signature(),
        );

        let msg = PrefixConsensusMsg::from(vote3);

        // Serialize
        let serialized = bcs::to_bytes(&msg).expect("Serialization with full QC2 should succeed");

        // Deserialize
        let deserialized: PrefixConsensusMsg = bcs::from_bytes(&serialized)
            .expect("Deserialization with full QC2 should succeed");

        // Verify QC2 was preserved
        let vote3_back = deserialized.as_vote3().expect("Should be Vote3");
        assert_eq!(vote3_back.qc2.votes.len(), 2);
    }

    #[test]
    fn test_message_size() {
        // Test to understand serialized message sizes
        let vote1 = create_test_vote1();
        let msg1 = PrefixConsensusMsg::from(vote1);
        let size1 = bcs::to_bytes(&msg1).unwrap().len();

        let vote2 = create_test_vote2();
        let msg2 = PrefixConsensusMsg::from(vote2);
        let size2 = bcs::to_bytes(&msg2).unwrap().len();

        let vote3 = create_test_vote3();
        let msg3 = PrefixConsensusMsg::from(vote3);
        let size3 = bcs::to_bytes(&msg3).unwrap().len();

        // Print sizes for information (will show in test output with --nocapture)
        println!("Vote1Msg serialized size: {} bytes", size1);
        println!("Vote2Msg serialized size (empty QC1): {} bytes", size2);
        println!("Vote3Msg serialized size (empty QC2): {} bytes", size3);

        // Basic sanity checks
        assert!(size1 > 0);
        assert!(size2 > 0);
        assert!(size3 > 0);
    }
}
