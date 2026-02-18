// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Types for the Multi-Slot Prefix Consensus protocol (Algorithm 4).
//!
//! Each slot, every validator broadcasts a `SlotProposal` containing their payload
//! (transactions pulled from mempool). The `SlotConsensusMsg` enum wraps both
//! slot proposals and per-slot Strong Prefix Consensus messages for network routing.

use crate::network_messages::StrongPrefixConsensusMsg;
use anyhow::{ensure, Result};
use aptos_consensus_types::common::{Author, Payload};
use aptos_crypto::{bls12381::Signature as BlsSignature, HashValue};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::{validator_signer::ValidatorSigner, validator_verifier::ValidatorVerifier};
use serde::{Deserialize, Serialize};

// ============================================================================
// SlotProposal Signing
// ============================================================================

/// Signable data for a SlotProposal (excludes the signature and full payload).
///
/// We sign `payload_hash` (the SHA3-256 hash of the BCS-serialized payload) rather
/// than the full payload to avoid expensive serialization during signing. The full
/// payload is carried in `SlotProposal` and integrity is verified by checking
/// `H(payload) == payload_hash` before signature verification.
#[derive(Clone, Debug, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct SlotProposalSignData {
    pub slot: u64,
    pub epoch: u64,
    pub author: Author,
    pub payload_hash: HashValue,
}

// ============================================================================
// SlotProposal
// ============================================================================

/// A validator's proposal for a slot in the multi-slot consensus protocol.
///
/// Each validator broadcasts one `SlotProposal` per slot containing transactions
/// pulled from the mempool. The proposal is BLS-signed over the `SlotProposalSignData`
/// (which includes the payload hash, not the full payload).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SlotProposal {
    pub slot: u64,
    pub epoch: u64,
    pub author: Author,
    pub payload_hash: HashValue,
    pub payload: Payload,
    pub signature: BlsSignature,
}

impl SlotProposal {
    /// Create a new SlotProposal. Computes `payload_hash` from the payload.
    pub fn new(
        slot: u64,
        epoch: u64,
        author: Author,
        payload: Payload,
        signature: BlsSignature,
    ) -> Self {
        let payload_hash = Self::compute_payload_hash(&payload);
        Self {
            slot,
            epoch,
            author,
            payload_hash,
            payload,
            signature,
        }
    }

    /// Reconstruct the signable data for verification.
    pub fn sign_data(&self) -> SlotProposalSignData {
        SlotProposalSignData {
            slot: self.slot,
            epoch: self.epoch,
            author: self.author,
            payload_hash: self.payload_hash,
        }
    }

    /// Verify the proposal: check payload integrity, then BLS signature.
    ///
    /// The payload integrity check prevents payload substitution attacks where
    /// a Byzantine sender signs one payload hash but includes a different payload.
    pub fn verify(&self, verifier: &ValidatorVerifier) -> Result<()> {
        // Step 1: Verify payload hash matches the carried payload
        let computed_hash = Self::compute_payload_hash(&self.payload);
        ensure!(
            computed_hash == self.payload_hash,
            "SlotProposal payload hash mismatch: computed {:?} != claimed {:?}",
            computed_hash,
            self.payload_hash,
        );

        // Step 2: Verify BLS signature over the sign data
        let sign_data = self.sign_data();
        verifier.verify(self.author, &sign_data, &self.signature)?;

        Ok(())
    }

    /// Compute the hash of a payload via BCS serialization + SHA3-256.
    /// Payload does not implement CryptoHash, so we hash manually.
    fn compute_payload_hash(payload: &Payload) -> HashValue {
        let bytes = bcs::to_bytes(payload).expect("Payload BCS serialization should not fail");
        HashValue::sha3_256_of(&bytes)
    }
}

/// Create a signed SlotProposal.
///
/// Computes the payload hash, constructs the signable data, signs it with the
/// validator's BLS key, and returns the complete SlotProposal.
pub fn create_signed_slot_proposal(
    slot: u64,
    epoch: u64,
    author: Author,
    payload: Payload,
    signer: &ValidatorSigner,
) -> Result<SlotProposal> {
    let payload_hash = SlotProposal::compute_payload_hash(&payload);
    let sign_data = SlotProposalSignData {
        slot,
        epoch,
        author,
        payload_hash,
    };
    let signature = signer.sign(&sign_data)?;
    Ok(SlotProposal {
        slot,
        epoch,
        author,
        payload_hash,
        payload,
        signature,
    })
}

// ============================================================================
// SlotConsensusMsg
// ============================================================================

/// Network message type for the multi-slot consensus protocol.
///
/// Wraps both slot proposals (broadcast at the start of each slot) and
/// per-slot Strong Prefix Consensus messages (routed to the SPC task for that slot).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SlotConsensusMsg {
    /// Validator's proposal for a slot (payload + BLS signature)
    SlotProposal(Box<SlotProposal>),

    /// Wrapped Strong Prefix Consensus message for a specific slot.
    /// The slot and epoch fields enable routing and filtering in the SlotManager
    /// and EpochManager respectively.
    StrongPCMsg {
        slot: u64,
        epoch: u64,
        msg: StrongPrefixConsensusMsg,
    },
}

impl SlotConsensusMsg {
    /// Get the epoch of this message (for epoch filtering in EpochManager).
    pub fn epoch(&self) -> u64 {
        match self {
            SlotConsensusMsg::SlotProposal(p) => p.epoch,
            SlotConsensusMsg::StrongPCMsg { epoch, .. } => *epoch,
        }
    }

    /// Get the slot of this message (for routing to the correct SPC task).
    pub fn slot(&self) -> u64 {
        match self {
            SlotConsensusMsg::SlotProposal(p) => p.slot,
            SlotConsensusMsg::StrongPCMsg { slot, .. } => *slot,
        }
    }

    /// Get the author/sender if available.
    pub fn author(&self) -> Option<Author> {
        match self {
            SlotConsensusMsg::SlotProposal(p) => Some(p.author),
            SlotConsensusMsg::StrongPCMsg { msg, .. } => msg.author(),
        }
    }

    /// Message type name for logging and metrics.
    pub fn name(&self) -> &'static str {
        match self {
            SlotConsensusMsg::SlotProposal(_) => "SlotProposal",
            SlotConsensusMsg::StrongPCMsg { .. } => "StrongPCMsg",
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_consensus_types::common::Payload;
    use aptos_crypto::HashValue;
    use aptos_types::{
        validator_signer::ValidatorSigner,
        validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
    };

    /// Create a test validator signer and matching verifier.
    fn create_test_validator() -> (ValidatorSigner, ValidatorVerifier) {
        let signer = ValidatorSigner::random(None);
        let author = signer.author();
        let public_key = signer.public_key();
        let validator_info = ValidatorConsensusInfo::new(author, public_key, 1);
        let verifier = ValidatorVerifier::new(vec![validator_info]);
        (signer, verifier)
    }

    /// Create a simple DirectMempool payload for testing.
    fn create_test_payload() -> Payload {
        Payload::DirectMempool(vec![])
    }

    #[test]
    fn test_slot_proposal_sign_and_verify() {
        let (signer, verifier) = create_test_validator();
        let author = signer.author();
        let payload = create_test_payload();

        let proposal =
            create_signed_slot_proposal(1, 1, author, payload, &signer).expect("signing failed");

        assert_eq!(proposal.slot, 1);
        assert_eq!(proposal.epoch, 1);
        assert_eq!(proposal.author, author);
        assert!(proposal.verify(&verifier).is_ok());
    }

    #[test]
    fn test_slot_proposal_verify_wrong_signer() {
        let (signer, _verifier) = create_test_validator();
        let (_, wrong_verifier) = create_test_validator();
        let author = signer.author();
        let payload = create_test_payload();

        let proposal =
            create_signed_slot_proposal(1, 1, author, payload, &signer).expect("signing failed");

        // Verification with a different validator's verifier should fail
        assert!(proposal.verify(&wrong_verifier).is_err());
    }

    #[test]
    fn test_slot_proposal_serialization_roundtrip() {
        let (signer, _) = create_test_validator();
        let author = signer.author();
        let payload = create_test_payload();

        let proposal =
            create_signed_slot_proposal(1, 1, author, payload, &signer).expect("signing failed");

        let bytes = bcs::to_bytes(&proposal).expect("serialization failed");
        let deserialized: SlotProposal =
            bcs::from_bytes(&bytes).expect("deserialization failed");

        assert_eq!(proposal, deserialized);
    }

    #[test]
    fn test_slot_proposal_tampered_payload() {
        let (signer, verifier) = create_test_validator();
        let author = signer.author();
        let payload = create_test_payload();

        let mut proposal =
            create_signed_slot_proposal(1, 1, author, payload, &signer).expect("signing failed");

        // Tamper with the payload after signing (substitute different transactions)
        proposal.payload = Payload::DirectMempool(vec![]);
        // Manually set a wrong payload_hash to simulate a payload substitution attack
        proposal.payload_hash = HashValue::random();

        // verify() should catch the payload hash mismatch before even checking the signature
        let result = proposal.verify(&verifier);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("payload hash mismatch")
        );
    }

    #[test]
    fn test_slot_consensus_msg_helpers() {
        let (signer, _) = create_test_validator();
        let author = signer.author();
        let payload = create_test_payload();
        let proposal =
            create_signed_slot_proposal(5, 3, author, payload, &signer).expect("signing failed");

        // Test SlotProposal variant
        let msg = SlotConsensusMsg::SlotProposal(Box::new(proposal));
        assert_eq!(msg.epoch(), 3);
        assert_eq!(msg.slot(), 5);
        assert_eq!(msg.author(), Some(author));
        assert_eq!(msg.name(), "SlotProposal");

        // Test StrongPCMsg variant
        let inner_vote = crate::types::Vote1::new(
            author,
            vec![HashValue::random()],
            3, // epoch
            5, // slot
            1, // view
            BlsSignature::dummy_signature(),
        );
        let spc_msg = StrongPrefixConsensusMsg::InnerPC {
            view: 1,
            msg: crate::network_messages::PrefixConsensusMsg::Vote1Msg(Box::new(inner_vote)),
        };
        let msg = SlotConsensusMsg::StrongPCMsg {
            slot: 5,
            epoch: 3,
            msg: spc_msg,
        };
        assert_eq!(msg.epoch(), 3);
        assert_eq!(msg.slot(), 5);
        assert_eq!(msg.name(), "StrongPCMsg");
    }

    #[test]
    fn test_slot_consensus_msg_serialization() {
        let (signer, _) = create_test_validator();
        let author = signer.author();
        let payload = create_test_payload();
        let proposal =
            create_signed_slot_proposal(1, 1, author, payload, &signer).expect("signing failed");

        let msg = SlotConsensusMsg::SlotProposal(Box::new(proposal));
        let bytes = bcs::to_bytes(&msg).expect("serialization failed");
        let deserialized: SlotConsensusMsg =
            bcs::from_bytes(&bytes).expect("deserialization failed");

        assert_eq!(deserialized.epoch(), 1);
        assert_eq!(deserialized.slot(), 1);
        assert_eq!(deserialized.name(), "SlotProposal");
    }
}
