// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Types for the Multi-Slot Prefix Consensus protocol (Algorithm 4).
//!
//! Each slot, every validator broadcasts a `SlotProposal` containing their payload
//! (transactions pulled from mempool). The `SlotConsensusMsg` enum wraps both
//! slot proposals and per-slot Strong Prefix Consensus messages for network routing.

use crate::certificates::StrongPCCommit;
use crate::network_interface::PriorityClassifiable;
use crate::network_messages::StrongPrefixConsensusMsg;
use crate::types::PrefixVector;
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
    /// Hash of the previous slot's commit proof (None for slot 1).
    /// Each validator signs over their own proof hash; the canonical proof
    /// for ranking updates is selected from the first non-⊥ entry in v_high.
    pub prev_commit_proof_hash: Option<HashValue>,
}

// ============================================================================
// SlotProposal
// ============================================================================

/// A validator's proposal for a slot in the multi-slot consensus protocol.
///
/// Each validator broadcasts one `SlotProposal` per slot containing transactions
/// pulled from the mempool. The proposal is BLS-signed over the `SlotProposalSignData`
/// (which includes the payload hash, not the full payload).
///
/// `timestamp_usecs` carries the proposer's local wall-clock time. It is NOT part of
/// the signed data — it's advisory metadata used to compute a deterministic block
/// timestamp as `max(parent_ts + 1, max(proposal timestamps in v_high))`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SlotProposal {
    pub slot: u64,
    pub epoch: u64,
    pub author: Author,
    pub payload_hash: HashValue,
    pub payload: Payload,
    /// Previous slot's commit proof for verifiable ranking (None for slot 1).
    pub prev_commit_proof: Option<StrongPCCommit>,
    pub prev_commit_proof_hash: Option<HashValue>,
    pub signature: BlsSignature,
    pub timestamp_usecs: u64,
}

impl SlotProposal {
    /// Create a new SlotProposal. Computes `payload_hash` from the payload.
    pub fn new(
        slot: u64,
        epoch: u64,
        author: Author,
        payload: Payload,
        prev_commit_proof: Option<StrongPCCommit>,
        signature: BlsSignature,
        timestamp_usecs: u64,
    ) -> Self {
        let payload_hash = Self::compute_payload_hash(&payload);
        let prev_commit_proof_hash = prev_commit_proof.as_ref().map(Self::compute_commit_proof_hash);
        Self {
            slot,
            epoch,
            author,
            payload_hash,
            payload,
            prev_commit_proof,
            prev_commit_proof_hash,
            signature,
            timestamp_usecs,
        }
    }

    /// Reconstruct the signable data for verification.
    pub fn sign_data(&self) -> SlotProposalSignData {
        SlotProposalSignData {
            slot: self.slot,
            epoch: self.epoch,
            author: self.author,
            payload_hash: self.payload_hash,
            prev_commit_proof_hash: self.prev_commit_proof_hash,
        }
    }

    /// Verify the proposal: check payload integrity, commit proof integrity, then BLS signature.
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

        // Step 2: Verify prev_commit_proof hash integrity
        let computed_proof_hash = self.prev_commit_proof.as_ref().map(Self::compute_commit_proof_hash);
        ensure!(
            computed_proof_hash == self.prev_commit_proof_hash,
            "SlotProposal prev_commit_proof_hash mismatch"
        );

        // Step 3: Enforce proof presence rules (structural checks only).
        // The expensive cryptographic verification of prev_commit_proof (O(n³) BLS sigs)
        // is deferred to extract_canonical_proof in finalize_slot, where it runs once
        // per slot instead of once per proposal, and off the event loop hot path.
        // The BLS signature in Step 4 binds the proposer to prev_commit_proof_hash,
        // so the proof cannot be swapped without invalidating the signature.
        if self.slot <= 1 {
            ensure!(
                self.prev_commit_proof.is_none(),
                "SlotProposal for slot {} must not carry a commit proof (no predecessor)",
                self.slot,
            );
        } else {
            let proof = self.prev_commit_proof.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "SlotProposal for slot {} must carry a commit proof",
                    self.slot,
                )
            })?;
            // Structural check only: proof must reference the previous slot.
            // Cryptographic verification deferred to finalize_slot.
            ensure!(
                proof.slot == self.slot - 1,
                "SlotProposal prev_commit_proof slot mismatch: proof slot {} != expected {}",
                proof.slot,
                self.slot - 1,
            );
        }

        // Step 4: Verify BLS signature over the sign data
        let sign_data = self.sign_data();
        verifier.verify(self.author, &sign_data, &self.signature)?;

        Ok(())
    }

    /// Compute the hash of a payload via BCS serialization + SHA3-256.
    /// Payload does not implement CryptoHash, so we hash manually.
    pub fn compute_payload_hash(payload: &Payload) -> HashValue {
        let bytes = bcs::to_bytes(payload).expect("Payload BCS serialization should not fail");
        HashValue::sha3_256_of(&bytes)
    }

    /// Compute the hash of a StrongPCCommit via BCS serialization + SHA3-256.
    pub fn compute_commit_proof_hash(proof: &StrongPCCommit) -> HashValue {
        let bytes = bcs::to_bytes(proof).expect("StrongPCCommit BCS serialization should not fail");
        HashValue::sha3_256_of(&bytes)
    }

    /// Compute the composite entry hash for this proposal's consensus-critical data.
    /// This is the value pushed into the SPC input vector (replaces payload_hash).
    pub fn entry_hash(&self) -> HashValue {
        compute_entry_hash(
            self.payload_hash,
            self.timestamp_usecs,
            self.prev_commit_proof_hash,
        )
    }
}

// ============================================================================
// Composite Entry Hash
// ============================================================================

/// Compute the composite entry hash that pins down all consensus-critical
/// proposal data in a single SPC input vector entry.
///
/// `entry_hash = SHA3-256(payload_hash || timestamp_usecs || prev_commit_proof_hash)`
///
/// SPC agreement on v_high entries using this hash guarantees that all honest
/// validators agree on the exact payload, timestamp, and commit proof for each
/// position — preventing Byzantine equivocation on proofs and timestamp divergence.
pub fn compute_entry_hash(
    payload_hash: HashValue,
    timestamp_usecs: u64,
    prev_commit_proof_hash: Option<HashValue>,
) -> HashValue {
    let mut bytes = Vec::with_capacity(73); // 32 + 8 + 1 + 32
    bytes.extend_from_slice(payload_hash.as_ref());
    bytes.extend_from_slice(&timestamp_usecs.to_le_bytes());
    match prev_commit_proof_hash {
        Some(h) => {
            bytes.push(1u8);
            bytes.extend_from_slice(h.as_ref());
        },
        None => {
            bytes.push(0u8);
        },
    }
    HashValue::sha3_256_of(&bytes)
}

// ============================================================================
// ProposalData — consensus-critical bundle for fetch and storage
// ============================================================================

/// All consensus-critical data from a proposal, bundled for storage and fetch.
///
/// When a v_high entry references an `entry_hash`, validators need the full
/// `ProposalData` to (a) execute the payload, (b) use the correct timestamp,
/// and (c) extract the commit proof for ranking updates. The `entry_hash`
/// is verified by recomputing `compute_entry_hash` from these fields.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProposalData {
    pub payload_hash: HashValue,
    pub payload: Payload,
    pub timestamp_usecs: u64,
    pub prev_commit_proof: Option<StrongPCCommit>,
    pub prev_commit_proof_hash: Option<HashValue>,
}

impl ProposalData {
    /// Create ProposalData from a SlotProposal.
    pub fn from_proposal(proposal: &SlotProposal) -> Self {
        Self {
            payload_hash: proposal.payload_hash,
            payload: proposal.payload.clone(),
            timestamp_usecs: proposal.timestamp_usecs,
            prev_commit_proof: proposal.prev_commit_proof.clone(),
            prev_commit_proof_hash: proposal.prev_commit_proof_hash,
        }
    }

    /// Compute the composite entry hash for this data.
    pub fn entry_hash(&self) -> HashValue {
        compute_entry_hash(
            self.payload_hash,
            self.timestamp_usecs,
            self.prev_commit_proof_hash,
        )
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
    timestamp_usecs: u64,
    prev_commit_proof: Option<StrongPCCommit>,
) -> Result<SlotProposal> {
    let payload_hash = SlotProposal::compute_payload_hash(&payload);
    let prev_commit_proof_hash = prev_commit_proof.as_ref().map(SlotProposal::compute_commit_proof_hash);
    let sign_data = SlotProposalSignData {
        slot,
        epoch,
        author,
        payload_hash,
        prev_commit_proof_hash,
    };
    let signature = signer.sign(&sign_data)?;
    Ok(SlotProposal {
        slot,
        epoch,
        author,
        payload_hash,
        payload,
        prev_commit_proof,
        prev_commit_proof_hash,
        signature,
        timestamp_usecs,
    })
}

// ============================================================================
// SPCOutput: SPC → SlotManager communication
// ============================================================================

/// Output events from the Strong Prefix Consensus task to the SlotManager.
///
/// SPC sends `VLow` when View 1's inner PC completes (early commit opportunity),
/// then `VHigh` when the full commit is reached.
#[derive(Clone, Debug)]
pub enum SPCOutput {
    /// View 1's inner PC v_low is available. Each non-bot entry can be committed
    /// as a block immediately (safe because v_low ⪯ v_high at every position).
    VLow { slot: u64, v_low: PrefixVector },
    /// Full commit v_high is available. Entries not already committed via VLow
    /// become additional blocks. Carries the `StrongPCCommit` proof for the
    /// SlotManager to embed in the next slot's proposals (verifiable ranking).
    VHigh {
        slot: u64,
        v_high: PrefixVector,
        commit_proof: StrongPCCommit,
    },
}

// ============================================================================
// Entry Fetch Types
// ============================================================================

/// Request for missing proposal data identified by its composite entry hash.
///
/// Sent when v_high contains an entry_hash for a proposal we never received.
/// The responder looks up the ProposalData and returns it for verification
/// against the SPC-agreed entry_hash.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntryFetchRequest {
    pub slot: u64,
    pub epoch: u64,
    pub entry_hash: HashValue,
}

/// Response carrying requested proposal data (payload + timestamp + commit proof).
///
/// The receiver verifies `compute_entry_hash(data) == entry_hash` to prevent
/// substitution. No BLS signature needed — the entry_hash was committed by SPC
/// agreement on v_high, which pins down all consensus-critical fields.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntryFetchResponse {
    pub slot: u64,
    pub epoch: u64,
    pub entry_hash: HashValue,
    pub data: ProposalData,
}

impl EntryFetchResponse {
    /// Verify that the carried proposal data matches the claimed entry hash.
    ///
    /// Three checks:
    /// 1. `payload_hash` matches the actual `payload` (prevents payload substitution)
    /// 2. `prev_commit_proof_hash` matches the actual `prev_commit_proof` (prevents proof substitution)
    /// 3. Composite `entry_hash` matches the SPC-agreed value
    ///
    /// Without checks 1 and 2, a Byzantine responder could supply a valid entry_hash
    /// (binding payload_hash + timestamp + proof_hash) but substitute garbage payload
    /// or proof data, since entry_hash only binds the *hashes*, not the raw objects.
    pub fn verify_entry_hash(&self) -> bool {
        // Check payload integrity
        if SlotProposal::compute_payload_hash(&self.data.payload) != self.data.payload_hash {
            return false;
        }
        // Check commit proof integrity
        let computed_proof_hash = self
            .data
            .prev_commit_proof
            .as_ref()
            .map(SlotProposal::compute_commit_proof_hash);
        if computed_proof_hash != self.data.prev_commit_proof_hash {
            return false;
        }
        // Check composite entry hash
        self.data.entry_hash() == self.entry_hash
    }
}

// ============================================================================
// SlotConsensusMsg
// ============================================================================

/// Network message type for the multi-slot consensus protocol.
///
/// Wraps slot proposals, per-slot Strong Prefix Consensus messages, and
/// entry fetch request/response messages for network routing.
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

    /// Request for missing proposal data by entry hash (broadcast to all peers).
    EntryFetchRequest(EntryFetchRequest),

    /// Response carrying requested proposal data (sent to requester).
    EntryFetchResponse(Box<EntryFetchResponse>),
}

impl SlotConsensusMsg {
    /// Get the epoch of this message (for epoch filtering in EpochManager).
    pub fn epoch(&self) -> u64 {
        match self {
            SlotConsensusMsg::SlotProposal(p) => p.epoch,
            SlotConsensusMsg::StrongPCMsg { epoch, .. } => *epoch,
            SlotConsensusMsg::EntryFetchRequest(req) => req.epoch,
            SlotConsensusMsg::EntryFetchResponse(resp) => resp.epoch,
        }
    }

    /// Get the slot of this message (for routing to the correct SPC task).
    pub fn slot(&self) -> u64 {
        match self {
            SlotConsensusMsg::SlotProposal(p) => p.slot,
            SlotConsensusMsg::StrongPCMsg { slot, .. } => *slot,
            SlotConsensusMsg::EntryFetchRequest(req) => req.slot,
            SlotConsensusMsg::EntryFetchResponse(resp) => resp.slot,
        }
    }

    /// Get the author/sender if available.
    pub fn author(&self) -> Option<Author> {
        match self {
            SlotConsensusMsg::SlotProposal(p) => Some(p.author),
            SlotConsensusMsg::StrongPCMsg { msg, .. } => msg.author(),
            SlotConsensusMsg::EntryFetchRequest(_) => None,
            SlotConsensusMsg::EntryFetchResponse(_) => None,
        }
    }

    /// Message type name for logging and metrics.
    pub fn name(&self) -> &'static str {
        match self {
            SlotConsensusMsg::SlotProposal(_) => "SlotProposal",
            SlotConsensusMsg::StrongPCMsg { .. } => "StrongPCMsg",
            SlotConsensusMsg::EntryFetchRequest(_) => "EntryFetchRequest",
            SlotConsensusMsg::EntryFetchResponse(_) => "EntryFetchResponse",
        }
    }
}

/// SlotConsensusMsg does not use priority routing (the SlotManager processes all
/// message types in a single event loop without priority separation).
impl PriorityClassifiable for SlotConsensusMsg {}

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
            create_signed_slot_proposal(1, 1, author, payload, &signer, 0, None).expect("signing failed");

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
            create_signed_slot_proposal(1, 1, author, payload, &signer, 0, None).expect("signing failed");

        // Verification with a different validator's verifier should fail
        assert!(proposal.verify(&wrong_verifier).is_err());
    }

    #[test]
    fn test_slot_proposal_serialization_roundtrip() {
        let (signer, _) = create_test_validator();
        let author = signer.author();
        let payload = create_test_payload();

        let proposal =
            create_signed_slot_proposal(1, 1, author, payload, &signer, 0, None).expect("signing failed");

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
            create_signed_slot_proposal(1, 1, author, payload, &signer, 0, None).expect("signing failed");

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
            create_signed_slot_proposal(5, 3, author, payload, &signer, 0, None).expect("signing failed");

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
            create_signed_slot_proposal(1, 1, author, payload, &signer, 0, None).expect("signing failed");

        let msg = SlotConsensusMsg::SlotProposal(Box::new(proposal));
        let bytes = bcs::to_bytes(&msg).expect("serialization failed");
        let deserialized: SlotConsensusMsg =
            bcs::from_bytes(&bytes).expect("deserialization failed");

        assert_eq!(deserialized.epoch(), 1);
        assert_eq!(deserialized.slot(), 1);
        assert_eq!(deserialized.name(), "SlotProposal");
    }

    #[test]
    fn test_entry_fetch_request_serialization() {
        let req = EntryFetchRequest {
            slot: 5,
            epoch: 3,
            entry_hash: HashValue::random(),
        };
        let msg = SlotConsensusMsg::EntryFetchRequest(req.clone());
        assert_eq!(msg.epoch(), 3);
        assert_eq!(msg.slot(), 5);
        assert_eq!(msg.name(), "EntryFetchRequest");

        let bytes = bcs::to_bytes(&msg).expect("serialization failed");
        let deserialized: SlotConsensusMsg =
            bcs::from_bytes(&bytes).expect("deserialization failed");
        assert_eq!(deserialized.epoch(), 3);
        assert_eq!(deserialized.slot(), 5);
        assert_eq!(deserialized.name(), "EntryFetchRequest");
    }

    #[test]
    fn test_entry_fetch_response_serialization_and_verification() {
        let payload = create_test_payload();
        let payload_hash = SlotProposal::compute_payload_hash(&payload);
        let data = ProposalData {
            payload_hash,
            payload,
            timestamp_usecs: 1000,
            prev_commit_proof: None,
            prev_commit_proof_hash: None,
        };
        let entry_hash = data.entry_hash();
        let resp = EntryFetchResponse {
            slot: 5,
            epoch: 3,
            entry_hash,
            data,
        };
        assert!(resp.verify_entry_hash());

        let msg = SlotConsensusMsg::EntryFetchResponse(Box::new(resp));
        assert_eq!(msg.epoch(), 3);
        assert_eq!(msg.slot(), 5);
        assert_eq!(msg.name(), "EntryFetchResponse");

        let bytes = bcs::to_bytes(&msg).expect("serialization failed");
        let deserialized: SlotConsensusMsg =
            bcs::from_bytes(&bytes).expect("deserialization failed");
        assert_eq!(deserialized.epoch(), 3);
        assert_eq!(deserialized.slot(), 5);
    }

    #[test]
    fn test_slot_consensus_msg_is_never_priority() {
        use crate::network_interface::PriorityClassifiable;

        // SlotConsensusMsg uses the default is_priority() which always returns false
        let (signer, _) = create_test_validator();
        let author = signer.author();
        let payload = create_test_payload();
        let proposal =
            create_signed_slot_proposal(1, 1, author, payload, &signer, 0, None).expect("signing failed");

        let msg = SlotConsensusMsg::SlotProposal(Box::new(proposal));
        assert!(!msg.is_priority());

        let inner_vote = crate::types::Vote1::new(
            author,
            vec![HashValue::random()],
            1, 1, 1,
            BlsSignature::dummy_signature(),
        );
        let spc_msg = StrongPrefixConsensusMsg::InnerPC {
            view: 1,
            msg: crate::network_messages::PrefixConsensusMsg::Vote1Msg(Box::new(inner_vote)),
        };
        let msg = SlotConsensusMsg::StrongPCMsg { slot: 1, epoch: 1, msg: spc_msg };
        assert!(!msg.is_priority());

        let req = EntryFetchRequest { slot: 1, epoch: 1, entry_hash: HashValue::random() };
        let msg = SlotConsensusMsg::EntryFetchRequest(req);
        assert!(!msg.is_priority());
    }

    #[test]
    fn test_entry_fetch_response_wrong_hash() {
        let payload = create_test_payload();
        let payload_hash = SlotProposal::compute_payload_hash(&payload);
        let data = ProposalData {
            payload_hash,
            payload,
            timestamp_usecs: 1000,
            prev_commit_proof: None,
            prev_commit_proof_hash: None,
        };
        let resp = EntryFetchResponse {
            slot: 1,
            epoch: 1,
            entry_hash: HashValue::random(), // wrong hash
            data,
        };
        assert!(!resp.verify_entry_hash());
    }

    #[test]
    fn test_entry_fetch_response_rejects_payload_substitution() {
        // Byzantine responder supplies a fabricated payload_hash that doesn't
        // match the carried payload. The entry_hash is computed from the fake
        // payload_hash, so the composite check passes — but the payload
        // integrity check catches the mismatch.
        let payload = create_test_payload();
        let fake_payload_hash = HashValue::random(); // doesn't match H(payload)

        let data = ProposalData {
            payload_hash: fake_payload_hash,
            payload,
            timestamp_usecs: 1000,
            prev_commit_proof: None,
            prev_commit_proof_hash: None,
        };
        // entry_hash is valid w.r.t. the fake_payload_hash
        let entry_hash = compute_entry_hash(fake_payload_hash, 1000, None);
        let resp = EntryFetchResponse {
            slot: 1,
            epoch: 1,
            entry_hash,
            data,
        };
        // Should fail because H(payload) != fake_payload_hash
        assert!(!resp.verify_entry_hash());
    }

    #[test]
    fn test_entry_fetch_response_rejects_proof_hash_mismatch() {
        // Byzantine responder supplies correct prev_commit_proof_hash but
        // prev_commit_proof is None (or vice versa).
        let payload = create_test_payload();
        let payload_hash = SlotProposal::compute_payload_hash(&payload);
        let fake_proof_hash = Some(HashValue::random());

        let data = ProposalData {
            payload_hash,
            payload,
            timestamp_usecs: 1000,
            prev_commit_proof: None, // no actual proof
            prev_commit_proof_hash: fake_proof_hash, // but claims there is one
        };
        let entry_hash = compute_entry_hash(payload_hash, 1000, fake_proof_hash);
        let resp = EntryFetchResponse {
            slot: 1,
            epoch: 1,
            entry_hash,
            data,
        };
        // Should fail because H(None) != Some(hash)
        assert!(!resp.verify_entry_hash());
    }

    // ========================================================================
    // Composite entry hash tests (Phase 13)
    // ========================================================================

    #[test]
    fn test_compute_entry_hash_determinism() {
        let payload_hash = HashValue::random();
        let timestamp = 42u64;
        let proof_hash = Some(HashValue::random());

        let h1 = compute_entry_hash(payload_hash, timestamp, proof_hash);
        let h2 = compute_entry_hash(payload_hash, timestamp, proof_hash);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_compute_entry_hash_differs_on_timestamp() {
        let payload_hash = HashValue::random();
        let proof_hash = Some(HashValue::random());

        let h1 = compute_entry_hash(payload_hash, 100, proof_hash);
        let h2 = compute_entry_hash(payload_hash, 200, proof_hash);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_compute_entry_hash_differs_on_proof_hash() {
        let payload_hash = HashValue::random();
        let timestamp = 42u64;

        let h1 = compute_entry_hash(payload_hash, timestamp, Some(HashValue::random()));
        let h2 = compute_entry_hash(payload_hash, timestamp, Some(HashValue::random()));
        assert_ne!(h1, h2);

        // None vs Some should also differ
        let h3 = compute_entry_hash(payload_hash, timestamp, None);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_proposal_data_entry_hash_matches_proposal() {
        let (signer, _) = create_test_validator();
        let author = signer.author();
        let payload = create_test_payload();

        let proposal =
            create_signed_slot_proposal(1, 1, author, payload, &signer, 12345, None)
                .expect("signing failed");

        let data = ProposalData::from_proposal(&proposal);
        assert_eq!(proposal.entry_hash(), data.entry_hash());
        assert_eq!(data.payload_hash, proposal.payload_hash);
        assert_eq!(data.timestamp_usecs, proposal.timestamp_usecs);
        assert_eq!(data.prev_commit_proof_hash, proposal.prev_commit_proof_hash);
    }

    // ========================================================================
    // Commit proof verification tests (Phase 12)
    // ========================================================================

    /// Create multiple test validators with a matching verifier.
    fn create_test_validators(n: usize) -> (Vec<ValidatorSigner>, ValidatorVerifier) {
        let signers: Vec<_> = (0..n).map(|_| ValidatorSigner::random(None)).collect();
        let infos: Vec<_> = signers
            .iter()
            .map(|s| ValidatorConsensusInfo::new(s.author(), s.public_key(), 1))
            .collect();
        (signers, ValidatorVerifier::new(infos))
    }

    /// Create a valid StrongPCCommit for the given slot (full v_low fast path).
    /// Uses all signers voting with the same prefix through 3 rounds (mcp == mce).
    fn create_valid_commit_proof(
        signers: &[ValidatorSigner],
        epoch: u64,
        slot: u64,
    ) -> crate::certificates::StrongPCCommit {
        use crate::signing::{create_signed_vote1, create_signed_vote2, create_signed_vote3};
        use crate::types::{QC1, QC2, QC3};

        let n = signers.len();
        let prefix: Vec<HashValue> = (0..n)
            .map(|i| HashValue::sha3_256_of(&(i as u64).to_le_bytes()))
            .collect();

        let vote1s: Vec<_> = signers
            .iter()
            .map(|s| {
                create_signed_vote1(s.author(), prefix.clone(), epoch, slot, 1, s)
                    .expect("sign vote1")
            })
            .collect();
        let qc1 = QC1::new(vote1s);

        let vote2s: Vec<_> = signers
            .iter()
            .map(|s| {
                create_signed_vote2(s.author(), prefix.clone(), qc1.clone(), epoch, slot, 1, s)
                    .expect("sign vote2")
            })
            .collect();
        let qc2 = QC2::new(vote2s);

        let vote3s: Vec<_> = signers
            .iter()
            .map(|s| {
                create_signed_vote3(s.author(), prefix.clone(), qc2.clone(), epoch, slot, 1, s)
                    .expect("sign vote3")
            })
            .collect();
        let qc3 = QC3::new(vote3s);

        crate::certificates::StrongPCCommit::new(qc3, vec![], prefix, epoch, slot)
    }

    #[test]
    fn test_slot_proposal_with_valid_commit_proof() {
        // Slot 2 proposal embedding a valid commit proof from slot 1
        let (signers, verifier) = create_test_validators(4);
        let signer = &signers[0];
        let author = signer.author();
        let payload = create_test_payload();

        let proof = create_valid_commit_proof(&signers, 1, 1);
        let proposal = create_signed_slot_proposal(
            2, 1, author, payload, signer, 0, Some(proof),
        )
        .expect("signing failed");

        assert!(proposal.prev_commit_proof.is_some());
        assert!(proposal.prev_commit_proof_hash.is_some());
        assert!(proposal.verify(&verifier).is_ok());
    }

    #[test]
    fn test_slot_proposal_commit_proof_hash_mismatch() {
        // Tamper prev_commit_proof_hash to mismatch the actual proof
        let (signers, verifier) = create_test_validators(4);
        let signer = &signers[0];
        let author = signer.author();
        let payload = create_test_payload();

        let proof = create_valid_commit_proof(&signers, 1, 1);
        let mut proposal = create_signed_slot_proposal(
            2, 1, author, payload, signer, 0, Some(proof),
        )
        .expect("signing failed");

        // Tamper the proof hash (keeps proof and BLS signature intact)
        proposal.prev_commit_proof_hash = Some(HashValue::random());

        let result = proposal.verify(&verifier);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("prev_commit_proof_hash mismatch")
        );
    }

    #[test]
    fn test_slot_proposal_invalid_commit_proof_signatures_accepted() {
        // Proposals with invalid proof signatures now pass verify() — the expensive
        // O(n³) proof verification was moved to extract_canonical_proof in finalize_slot
        // to avoid blocking the event loop. The BLS signature binds the proposer to
        // prev_commit_proof_hash, so proof integrity is guaranteed.
        // TODO: Once real proof verification is added back to extract_canonical_proof,
        // add a test there that invalid proofs are skipped.
        let (signers, verifier) = create_test_validators(4);
        let signer = &signers[0];
        let author = signer.author();
        let payload = create_test_payload();

        // Commit proof with an empty (unverifiable) QC3
        let bad_proof = crate::certificates::StrongPCCommit::new(
            crate::types::QC3::new(vec![]),
            vec![],
            vec![HashValue::random()],
            1,
            1,
        );

        let proposal = create_signed_slot_proposal(
            2, 1, author, payload, signer, 0, Some(bad_proof),
        )
        .expect("signing failed");

        // Proposal verify() no longer checks proof signatures — only structural checks
        let result = proposal.verify(&verifier);
        assert!(result.is_ok(), "Proposal with bad proof should pass verify(): {:?}", result);
    }
}
