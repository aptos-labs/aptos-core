// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Per-slot proposal buffer and state for Multi-Slot Consensus (Algorithm 4).
//!
//! Each slot collects [`SlotProposal`]s from all validators into a [`ProposalBuffer`].
//! When all proposals arrive (or the 2Δ timer expires), the buffer builds an input
//! vector ordered by the current ranking for the slot's SPC instance.
//!
//! [`SlotState`] wraps the buffer with a phase state machine:
//! `CollectingProposals → RunningSPC → Committed`
//!
//! **Caller responsibility**: Neither `ProposalBuffer` nor `SlotState` validates the
//! `slot` or `epoch` fields on incoming proposals. The caller (SlotManager in Phase 5)
//! is responsible for routing proposals to the correct slot's buffer and verifying
//! epoch before insertion.

use crate::{
    slot_types::SlotProposal,
    types::PrefixVector,
};
use aptos_consensus_types::common::{Author, Payload};
use aptos_crypto::HashValue;
use aptos_logger::prelude::*;
use std::collections::HashMap;

// ============================================================================
// ProposalBuffer
// ============================================================================

/// Collects [`SlotProposal`]s from validators for a single slot.
///
/// Stores at most one proposal per author. When all `n` expected proposals are
/// received, [`is_complete()`](ProposalBuffer::is_complete) returns true.
#[derive(Debug)]
pub struct ProposalBuffer {
    proposals: HashMap<Author, SlotProposal>,
    n: usize,
}

impl ProposalBuffer {
    /// Create an empty buffer expecting `n` proposals (one per validator).
    pub fn new(n: usize) -> Self {
        Self {
            proposals: HashMap::with_capacity(n),
            n,
        }
    }

    /// Insert a proposal into the buffer.
    ///
    /// Returns `true` if all `n` proposals are now received (buffer is complete).
    /// Returns `false` if the buffer is not yet complete, or if the proposal was
    /// a duplicate from the same author (duplicates are silently rejected without
    /// overwriting the existing proposal).
    pub fn insert(&mut self, proposal: SlotProposal) -> bool {
        if self.proposals.contains_key(&proposal.author) {
            return false;
        }
        self.proposals.insert(proposal.author, proposal);
        self.is_complete()
    }

    /// Whether all `n` proposals have been received.
    pub fn is_complete(&self) -> bool {
        self.proposals.len() == self.n
    }

    /// Number of proposals received so far.
    pub fn proposal_count(&self) -> usize {
        self.proposals.len()
    }

    /// Look up a proposal by author.
    pub fn get(&self, author: &Author) -> Option<&SlotProposal> {
        self.proposals.get(author)
    }

    /// Build the SPC input vector and payload lookup map, ordered by `ranking`.
    ///
    /// Returns:
    /// - `PrefixVector`: length-n hash vector where position i contains
    ///   `proposal.payload_hash` if ranking[i]'s proposal is present, or
    ///   `HashValue::zero()` (⊥) if missing.
    /// - `HashMap<HashValue, Payload>`: maps each present proposal's payload_hash
    ///   to its payload, for use during block construction after SPC completes.
    ///
    /// # Safety assumption
    ///
    /// `HashValue::zero()` is used as the ⊥ marker. A collision with a real
    /// `payload_hash` is cryptographically infeasible: `compute_payload_hash`
    /// produces SHA3-256 over BCS-serialized payload data, and the probability
    /// of hitting the all-zero hash is negligible (~2^{-256}).
    pub fn build_input_vector(
        &self,
        ranking: &[Author],
    ) -> (PrefixVector, HashMap<HashValue, Payload>) {
        let mut hash_vector = Vec::with_capacity(ranking.len());
        let mut payload_map = HashMap::new();

        for author in ranking {
            if let Some(proposal) = self.proposals.get(author) {
                hash_vector.push(proposal.payload_hash);
                payload_map
                    .entry(proposal.payload_hash)
                    .or_insert_with(|| proposal.payload.clone());
            } else {
                hash_vector.push(HashValue::zero());
            }
        }

        (hash_vector, payload_map)
    }
}

// ============================================================================
// SlotPhase
// ============================================================================

/// Lifecycle phase of a slot in the multi-slot consensus protocol.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotPhase {
    /// Collecting proposals from validators; 2Δ timer is running.
    CollectingProposals,
    /// SPC has been spawned and is running; messages are being forwarded.
    RunningSPC,
    /// v_high received and block committed (terminal state).
    Committed,
}

// ============================================================================
// SlotState
// ============================================================================

/// Per-slot state combining proposal buffer, phase tracking, and SPC I/O.
///
/// Created by the SlotManager at the start of each slot. Tracks the slot through
/// its lifecycle: collecting proposals → running SPC → committed.
///
/// **Phase transitions**: `prepare_spc_input()` handles `CollectingProposals → RunningSPC`.
/// The caller (SlotManager) sets `Committed` via `set_phase()` after the block has
/// been sent to the execution pipeline.
#[derive(Debug)]
pub struct SlotState {
    slot: u64,
    phase: SlotPhase,
    proposal_buffer: ProposalBuffer,
    /// Set when transitioning to RunningSPC — the hash vector passed to SPC.
    input_vector: Option<PrefixVector>,
    /// Set when transitioning to RunningSPC — maps payload_hash → Payload for commit.
    /// Late proposals (arriving during RunningSPC) are inserted directly into this map.
    payload_map: Option<HashMap<HashValue, Payload>>,
}

impl SlotState {
    /// Create a new slot state in `CollectingProposals` phase.
    pub fn new(slot: u64, n: usize) -> Self {
        Self {
            slot,
            phase: SlotPhase::CollectingProposals,
            proposal_buffer: ProposalBuffer::new(n),
            input_vector: None,
            payload_map: None,
        }
    }

    /// Slot number.
    pub fn slot(&self) -> u64 {
        self.slot
    }

    /// Current phase.
    pub fn phase(&self) -> &SlotPhase {
        &self.phase
    }

    /// Set the phase. Used by SlotManager to transition to `Committed`.
    pub fn set_phase(&mut self, phase: SlotPhase) {
        self.phase = phase;
    }

    /// Insert a proposal.
    ///
    /// - `CollectingProposals`: inserts into the proposal buffer.
    ///   Returns `true` if the buffer is now complete (all n proposals received).
    /// - `RunningSPC`: inserts payload into `payload_map` for v_high resolution.
    ///   Returns `false`.
    /// - `Committed`: silently ignored (slot is done). Returns `false`.
    pub fn insert_proposal(&mut self, proposal: SlotProposal) -> bool {
        match self.phase {
            SlotPhase::CollectingProposals => self.proposal_buffer.insert(proposal),
            SlotPhase::RunningSPC => {
                // Insert late proposal's payload directly into payload_map
                // so it's available for v_high resolution without a network fetch.
                if let Some(ref mut map) = self.payload_map {
                    map.entry(proposal.payload_hash)
                        .or_insert(proposal.payload);
                }
                false
            },
            SlotPhase::Committed => false,
        }
    }

    /// Build the SPC input vector from collected proposals and transition to `RunningSPC`.
    ///
    /// Calls `ProposalBuffer::build_input_vector()` with the given ranking, stores
    /// the results, and sets the phase to `RunningSPC`.
    ///
    /// # Panics
    ///
    /// Panics if not in `CollectingProposals` phase.
    pub fn prepare_spc_input(&mut self, ranking: &[Author]) {
        assert_eq!(
            self.phase,
            SlotPhase::CollectingProposals,
            "prepare_spc_input called in {:?} phase (slot {})",
            self.phase,
            self.slot,
        );
        let (input_vector, payload_map) = self.proposal_buffer.build_input_vector(ranking);
        self.input_vector = Some(input_vector);
        self.payload_map = Some(payload_map);
        self.phase = SlotPhase::RunningSPC;
    }

    /// The hash vector passed to SPC (set after `prepare_spc_input`).
    pub fn input_vector(&self) -> Option<&PrefixVector> {
        self.input_vector.as_ref()
    }

    /// The payload lookup map (set after `prepare_spc_input`).
    pub fn payload_map(&self) -> Option<&HashMap<HashValue, Payload>> {
        self.payload_map.as_ref()
    }

    /// Take ownership of the payload map for block construction.
    ///
    /// Called by SlotManager in `on_spc_v_high()` before building the block.
    /// Returns `None` if already taken or if `prepare_spc_input` was never called.
    /// Does NOT transition phase — the caller sets `Committed` via `set_phase()`
    /// after the block has been sent to execution.
    pub fn take_payload_map(&mut self) -> Option<HashMap<HashValue, Payload>> {
        self.payload_map.take()
    }

    /// Resolve payloads for all non-⊥ entries in v_high.
    ///
    /// Checks the `payload_map` which contains payloads from both pre-2Δ proposals
    /// and late proposals (inserted directly during RunningSPC).
    ///
    /// Returns:
    /// - `HashMap<HashValue, Payload>`: resolved payloads (hash → payload)
    /// - `Vec<HashValue>`: hashes still missing (need network fetch)
    pub fn resolve_missing_payloads(
        &self,
        v_high: &PrefixVector,
    ) -> (HashMap<HashValue, Payload>, Vec<HashValue>) {
        let mut resolved = HashMap::new();
        let mut missing = Vec::new();
        let payload_map = match &self.payload_map {
            Some(map) => map,
            None => {
                error!(
                    slot = self.slot,
                    "resolve_missing_payloads called but payload_map is None \
                     (prepare_spc_input was never called)"
                );
                return (resolved, missing);
            },
        };

        for hash in v_high {
            if *hash == HashValue::zero() {
                continue;
            }
            if resolved.contains_key(hash) {
                continue;
            }
            if let Some(payload) = payload_map.get(hash) {
                resolved.insert(*hash, payload.clone());
            } else {
                missing.push(*hash);
            }
        }

        (resolved, missing)
    }

    /// Look up a payload by hash from the payload_map.
    ///
    /// Used for resolving v_high entries and for handling fetch requests from peers.
    pub fn lookup_payload(&self, hash: &HashValue) -> Option<Payload> {
        self.payload_map
            .as_ref()
            .and_then(|map| map.get(hash).cloned())
    }

    /// Reference to the proposal buffer.
    pub fn proposal_buffer(&self) -> &ProposalBuffer {
        &self.proposal_buffer
    }

    /// Whether all expected proposals have been received.
    pub fn has_all_proposals(&self) -> bool {
        self.proposal_buffer.is_complete()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_consensus_types::common::Payload;
    use aptos_crypto::{bls12381::Signature as BlsSignature, HashValue};
    use aptos_types::validator_signer::ValidatorSigner;

    /// Create n validator signers with deterministic addresses.
    fn create_signers(n: usize) -> Vec<ValidatorSigner> {
        (0..n).map(|_| ValidatorSigner::random(None)).collect()
    }

    /// Create a signed SlotProposal with a given payload.
    fn make_proposal(
        slot: u64,
        epoch: u64,
        signer: &ValidatorSigner,
        payload: Payload,
    ) -> SlotProposal {
        crate::slot_types::create_signed_slot_proposal(slot, epoch, signer.author(), payload, signer)
            .expect("signing should not fail")
    }

    /// Create a test SlotProposal with a distinct payload hash (for tests needing unique hashes).
    ///
    /// Uses a random payload_hash and dummy signature. Not suitable for signature
    /// verification tests — only for buffer/ordering logic.
    fn make_proposal_with_distinct_hash(
        slot: u64,
        epoch: u64,
        author: Author,
    ) -> SlotProposal {
        SlotProposal {
            slot,
            epoch,
            author,
            payload_hash: HashValue::random(),
            payload: Payload::DirectMempool(vec![]),
            signature: BlsSignature::dummy_signature(),
        }
    }

    fn empty_payload() -> Payload {
        Payload::DirectMempool(vec![])
    }

    // ==================== ProposalBuffer Tests ====================

    #[test]
    fn test_proposal_buffer_insert_and_complete() {
        let signers = create_signers(4);
        let mut buffer = ProposalBuffer::new(4);

        // Insert 3 proposals — not yet complete
        for signer in &signers[..3] {
            let proposal = make_proposal(1, 1, signer, empty_payload());
            assert!(!buffer.insert(proposal));
        }
        assert_eq!(buffer.proposal_count(), 3);
        assert!(!buffer.is_complete());

        // Insert 4th — now complete
        let proposal = make_proposal(1, 1, &signers[3], empty_payload());
        assert!(buffer.insert(proposal));
        assert_eq!(buffer.proposal_count(), 4);
        assert!(buffer.is_complete());
    }

    #[test]
    fn test_proposal_buffer_reject_duplicate() {
        let signers = create_signers(4);
        let mut buffer = ProposalBuffer::new(4);

        let proposal1 = make_proposal(1, 1, &signers[0], empty_payload());
        let proposal2 = make_proposal(1, 1, &signers[0], empty_payload());

        // First insert succeeds (not complete yet)
        assert!(!buffer.insert(proposal1));
        assert_eq!(buffer.proposal_count(), 1);

        // Duplicate from same author is rejected
        assert!(!buffer.insert(proposal2));
        assert_eq!(buffer.proposal_count(), 1);
    }

    #[test]
    fn test_proposal_buffer_not_complete_partial() {
        let signers = create_signers(4);
        let mut buffer = ProposalBuffer::new(4);

        for signer in &signers[..3] {
            buffer.insert(make_proposal(1, 1, signer, empty_payload()));
        }

        assert_eq!(buffer.proposal_count(), 3);
        assert!(!buffer.is_complete());
    }

    #[test]
    fn test_proposal_buffer_get() {
        let signers = create_signers(2);
        let mut buffer = ProposalBuffer::new(2);

        let proposal = make_proposal(1, 1, &signers[0], empty_payload());
        buffer.insert(proposal);

        assert!(buffer.get(&signers[0].author()).is_some());
        assert!(buffer.get(&signers[1].author()).is_none());
    }

    // ==================== build_input_vector Tests ====================

    #[test]
    fn test_build_input_vector_all_present() {
        let signers = create_signers(4);
        let mut buffer = ProposalBuffer::new(4);

        // Use distinct hashes so all 4 entries are unique
        for signer in &signers {
            buffer.insert(make_proposal_with_distinct_hash(1, 1, signer.author()));
        }

        let ranking: Vec<Author> = signers.iter().map(|s| s.author()).collect();
        let (hash_vector, payload_map) = buffer.build_input_vector(&ranking);

        // All positions filled
        assert_eq!(hash_vector.len(), 4);
        for h in &hash_vector {
            assert_ne!(*h, HashValue::zero(), "no ⊥ entries expected");
        }
        assert_eq!(payload_map.len(), 4);
    }

    #[test]
    fn test_build_input_vector_partial() {
        let signers = create_signers(4);
        let mut buffer = ProposalBuffer::new(4);

        // Only insert proposals from signers[0] and signers[2]
        buffer.insert(make_proposal_with_distinct_hash(1, 1, signers[0].author()));
        buffer.insert(make_proposal_with_distinct_hash(1, 1, signers[2].author()));

        let ranking: Vec<Author> = signers.iter().map(|s| s.author()).collect();
        let (hash_vector, payload_map) = buffer.build_input_vector(&ranking);

        assert_eq!(hash_vector.len(), 4);
        assert_ne!(hash_vector[0], HashValue::zero()); // signers[0] present
        assert_eq!(hash_vector[1], HashValue::zero()); // signers[1] missing → ⊥
        assert_ne!(hash_vector[2], HashValue::zero()); // signers[2] present
        assert_eq!(hash_vector[3], HashValue::zero()); // signers[3] missing → ⊥
        assert_eq!(payload_map.len(), 2);
    }

    #[test]
    fn test_build_input_vector_ordering() {
        // Insert proposals in order [D, A, C] but ranking is [A, B, C, D]
        let signers = create_signers(4);
        let mut buffer = ProposalBuffer::new(4);

        let prop_d = make_proposal_with_distinct_hash(1, 1, signers[3].author());
        let prop_a = make_proposal_with_distinct_hash(1, 1, signers[0].author());
        let prop_c = make_proposal_with_distinct_hash(1, 1, signers[2].author());

        let hash_d = prop_d.payload_hash;
        let hash_a = prop_a.payload_hash;
        let hash_c = prop_c.payload_hash;

        // Insert out of ranking order
        buffer.insert(prop_d);
        buffer.insert(prop_a);
        buffer.insert(prop_c);

        let ranking: Vec<Author> = signers.iter().map(|s| s.author()).collect();
        let (hash_vector, _) = buffer.build_input_vector(&ranking);

        // Vector follows ranking order [A, B, C, D], not insertion order [D, A, C]
        assert_eq!(hash_vector[0], hash_a);
        assert_eq!(hash_vector[1], HashValue::zero()); // B missing
        assert_eq!(hash_vector[2], hash_c);
        assert_eq!(hash_vector[3], hash_d);
    }

    #[test]
    fn test_build_input_vector_empty() {
        let signers = create_signers(4);
        let buffer = ProposalBuffer::new(4);

        let ranking: Vec<Author> = signers.iter().map(|s| s.author()).collect();
        let (hash_vector, payload_map) = buffer.build_input_vector(&ranking);

        assert_eq!(hash_vector.len(), 4);
        for h in &hash_vector {
            assert_eq!(*h, HashValue::zero());
        }
        assert!(payload_map.is_empty());
    }

    #[test]
    fn test_build_input_vector_payload_map_correctness() {
        let signers = create_signers(3);
        let mut buffer = ProposalBuffer::new(3);

        // Each signer gets a proposal with a distinct random hash
        for signer in &signers {
            buffer.insert(make_proposal_with_distinct_hash(1, 1, signer.author()));
        }

        let ranking: Vec<Author> = signers.iter().map(|s| s.author()).collect();
        let (hash_vector, payload_map) = buffer.build_input_vector(&ranking);

        // Each hash in the vector maps to a payload in the map
        for hash in &hash_vector {
            assert!(payload_map.contains_key(hash), "payload_map should contain hash");
        }
        // Each proposal's payload_hash should appear in the vector at its ranking position
        for (i, signer) in signers.iter().enumerate() {
            let proposal = buffer.get(&signer.author()).unwrap();
            assert_eq!(hash_vector[i], proposal.payload_hash);
        }
    }

    // ==================== SlotState Tests ====================

    #[test]
    fn test_slot_state_phase_transitions() {
        let signers = create_signers(2);
        let mut state = SlotState::new(1, 2);

        assert_eq!(*state.phase(), SlotPhase::CollectingProposals);

        // Insert proposals
        for signer in &signers {
            state
                .insert_proposal(make_proposal(1, 1, signer, empty_payload()));
        }

        // Transition to RunningSPC
        let ranking: Vec<Author> = signers.iter().map(|s| s.author()).collect();
        state.prepare_spc_input(&ranking);
        assert_eq!(*state.phase(), SlotPhase::RunningSPC);

        // Transition to Committed
        state.set_phase(SlotPhase::Committed);
        assert_eq!(*state.phase(), SlotPhase::Committed);
    }

    #[test]
    fn test_slot_state_insert_buffers_late_during_spc() {
        let signers = create_signers(2);
        let mut state = SlotState::new(1, 2);

        // Prepare SPC (skip to RunningSPC with no proposals)
        let ranking: Vec<Author> = signers.iter().map(|s| s.author()).collect();
        state.prepare_spc_input(&ranking);

        // Insertion during RunningSPC succeeds (payload goes into payload_map)
        let proposal = make_proposal(1, 1, &signers[0], empty_payload());
        let hash = proposal.payload_hash;
        assert!(!state.insert_proposal(proposal)); // late proposals don't affect buffer completion

        // Payload should be findable via lookup_payload
        assert!(state.lookup_payload(&hash).is_some());
    }

    #[test]
    fn test_slot_state_insert_ignored_after_committed() {
        let signers = create_signers(2);
        let mut state = SlotState::new(1, 2);

        let ranking: Vec<Author> = signers.iter().map(|s| s.author()).collect();
        state.prepare_spc_input(&ranking);
        state.set_phase(SlotPhase::Committed);

        // Insertion in Committed phase is silently ignored
        let proposal = make_proposal(1, 1, &signers[0], empty_payload());
        assert!(!state.insert_proposal(proposal));
    }

    #[test]
    fn test_slot_state_prepare_spc_input() {
        let signers = create_signers(3);
        let mut state = SlotState::new(5, 3);

        // Insert 2 out of 3 proposals with distinct hashes
        state
            .insert_proposal(make_proposal_with_distinct_hash(5, 1, signers[0].author()));
        state
            .insert_proposal(make_proposal_with_distinct_hash(5, 1, signers[2].author()));

        assert!(state.input_vector().is_none());
        assert!(state.payload_map().is_none());

        let ranking: Vec<Author> = signers.iter().map(|s| s.author()).collect();
        state.prepare_spc_input(&ranking);

        // input_vector and payload_map should be set
        let iv = state.input_vector().expect("input_vector should be set");
        assert_eq!(iv.len(), 3);
        assert_ne!(iv[0], HashValue::zero());
        assert_eq!(iv[1], HashValue::zero()); // missing
        assert_ne!(iv[2], HashValue::zero());

        let pm = state.payload_map().expect("payload_map should be set");
        assert_eq!(pm.len(), 2);

        assert_eq!(*state.phase(), SlotPhase::RunningSPC);
    }

    #[test]
    #[should_panic(expected = "prepare_spc_input called in RunningSPC")]
    fn test_slot_state_prepare_spc_input_wrong_phase_running() {
        let signers = create_signers(2);
        let mut state = SlotState::new(1, 2);

        let ranking: Vec<Author> = signers.iter().map(|s| s.author()).collect();
        state.prepare_spc_input(&ranking); // → RunningSPC
        state.prepare_spc_input(&ranking); // should panic
    }

    #[test]
    #[should_panic(expected = "prepare_spc_input called in Committed")]
    fn test_slot_state_prepare_spc_input_wrong_phase_committed() {
        let signers = create_signers(2);
        let mut state = SlotState::new(1, 2);

        let ranking: Vec<Author> = signers.iter().map(|s| s.author()).collect();
        state.prepare_spc_input(&ranking);
        state.set_phase(SlotPhase::Committed);
        state.prepare_spc_input(&ranking); // should panic
    }

    #[test]
    fn test_slot_state_take_payload_map() {
        let signers = create_signers(2);
        let mut state = SlotState::new(1, 2);

        for signer in &signers {
            state
                .insert_proposal(make_proposal(1, 1, signer, empty_payload()));
        }

        let ranking: Vec<Author> = signers.iter().map(|s| s.author()).collect();
        state.prepare_spc_input(&ranking);

        // First take returns Some
        let map = state.take_payload_map();
        assert!(map.is_some());
        assert!(!map.unwrap().is_empty());

        // Second take returns None (already taken)
        assert!(state.take_payload_map().is_none());

        // payload_map() also returns None
        assert!(state.payload_map().is_none());
    }

    #[test]
    fn test_slot_state_accessors() {
        let state = SlotState::new(42, 4);

        assert_eq!(state.slot(), 42);
        assert_eq!(*state.phase(), SlotPhase::CollectingProposals);
        assert!(!state.has_all_proposals());
        assert_eq!(state.proposal_buffer().proposal_count(), 0);
    }

    #[test]
    fn test_slot_state_has_all_proposals_delegates() {
        let signers = create_signers(2);
        let mut state = SlotState::new(1, 2);

        assert!(!state.has_all_proposals());

        state.insert_proposal(make_proposal(1, 1, &signers[0], empty_payload()));
        assert!(!state.has_all_proposals());

        state.insert_proposal(make_proposal(1, 1, &signers[1], empty_payload()));
        assert!(state.has_all_proposals());
    }

    // ==================== Late Proposal + Payload Resolution Tests ====================

    #[test]
    fn test_late_proposal_insert_during_committed_ignored() {
        let signers = create_signers(2);
        let mut state = SlotState::new(1, 2);

        let ranking: Vec<Author> = signers.iter().map(|s| s.author()).collect();
        state.prepare_spc_input(&ranking);
        state.set_phase(SlotPhase::Committed);

        let proposal = make_proposal(1, 1, &signers[0], empty_payload());
        assert!(!state.insert_proposal(proposal));
    }

    #[test]
    fn test_late_proposal_duplicate_hash_not_overwritten() {
        let signers = create_signers(2);
        let mut state = SlotState::new(1, 2);

        let ranking: Vec<Author> = signers.iter().map(|s| s.author()).collect();
        state.prepare_spc_input(&ranking);

        // Insert a late proposal, then insert another with the same payload_hash.
        // The second should not overwrite the first (HashMap::entry().or_insert).
        let proposal = make_proposal_with_distinct_hash(1, 1, signers[0].author());
        let hash = proposal.payload_hash;
        state.insert_proposal(proposal);

        // Payload map size before second insert
        let size_before = state.payload_map().unwrap().len();

        // Insert with same hash — or_insert means no overwrite
        let mut dup = make_proposal_with_distinct_hash(1, 1, signers[1].author());
        dup.payload_hash = hash; // force same hash
        state.insert_proposal(dup);

        assert_eq!(state.payload_map().unwrap().len(), size_before);
        assert!(state.lookup_payload(&hash).is_some());
    }

    #[test]
    fn test_resolve_all_from_payload_map() {
        let signers = create_signers(3);
        let mut state = SlotState::new(1, 3);

        // Insert all 3 proposals before SPC
        for signer in &signers {
            state
                .insert_proposal(make_proposal_with_distinct_hash(1, 1, signer.author()));
        }

        let ranking: Vec<Author> = signers.iter().map(|s| s.author()).collect();
        state.prepare_spc_input(&ranking);

        let v_high = state.input_vector().unwrap().clone();
        let (resolved, missing) = state.resolve_missing_payloads(&v_high);

        assert_eq!(resolved.len(), 3);
        assert!(missing.is_empty());
    }

    #[test]
    fn test_resolve_from_late_proposal() {
        let signers = create_signers(3);
        let mut state = SlotState::new(1, 3);

        // Insert only signers[0] before SPC
        state
            .insert_proposal(make_proposal_with_distinct_hash(1, 1, signers[0].author()));

        let ranking: Vec<Author> = signers.iter().map(|s| s.author()).collect();
        state.prepare_spc_input(&ranking);

        // signer[1] arrives late
        let late_proposal = make_proposal_with_distinct_hash(1, 1, signers[1].author());
        let late_hash = late_proposal.payload_hash;
        state.insert_proposal(late_proposal);

        // v_high contains signers[0]'s hash and the late hash
        let v_high = vec![
            state.input_vector().unwrap()[0], // signer[0] — in payload_map (pre-2Δ)
            late_hash,                        // signer[1] — in payload_map (late insert)
            HashValue::zero(),                // signer[2] — ⊥
        ];

        let (resolved, missing) = state.resolve_missing_payloads(&v_high);
        assert_eq!(resolved.len(), 2);
        assert!(resolved.contains_key(&late_hash));
        assert!(missing.is_empty());
    }

    #[test]
    fn test_resolve_some_missing() {
        let signers = create_signers(3);
        let mut state = SlotState::new(1, 3);

        // Insert only signers[0] before SPC
        state
            .insert_proposal(make_proposal_with_distinct_hash(1, 1, signers[0].author()));

        let ranking: Vec<Author> = signers.iter().map(|s| s.author()).collect();
        state.prepare_spc_input(&ranking);

        // v_high contains a hash we don't have
        let unknown_hash = HashValue::random();
        let v_high = vec![
            state.input_vector().unwrap()[0], // signer[0] — in payload_map
            unknown_hash,                     // unknown — not in any buffer
            HashValue::zero(),                // ⊥
        ];

        let (resolved, missing) = state.resolve_missing_payloads(&v_high);
        assert_eq!(resolved.len(), 1);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0], unknown_hash);
    }

    #[test]
    fn test_resolve_ignores_bot_entries() {
        let signers = create_signers(3);
        let mut state = SlotState::new(1, 3);

        let ranking: Vec<Author> = signers.iter().map(|s| s.author()).collect();
        state.prepare_spc_input(&ranking);

        // v_high is all ⊥
        let v_high = vec![HashValue::zero(); 3];
        let (resolved, missing) = state.resolve_missing_payloads(&v_high);
        assert!(resolved.is_empty());
        assert!(missing.is_empty());
    }

    #[test]
    fn test_lookup_payload_from_early_and_late() {
        let signers = create_signers(3);
        let mut state = SlotState::new(1, 3);

        // Insert signer[0] before SPC
        let early_proposal = make_proposal_with_distinct_hash(1, 1, signers[0].author());
        let early_hash = early_proposal.payload_hash;
        state.insert_proposal(early_proposal);

        let ranking: Vec<Author> = signers.iter().map(|s| s.author()).collect();
        state.prepare_spc_input(&ranking);

        // Insert signer[1] as late proposal
        let late_proposal = make_proposal_with_distinct_hash(1, 1, signers[1].author());
        let late_hash = late_proposal.payload_hash;
        state.insert_proposal(late_proposal);

        // Both should be found via lookup_payload
        assert!(state.lookup_payload(&early_hash).is_some());
        assert!(state.lookup_payload(&late_hash).is_some());
    }

    #[test]
    fn test_lookup_payload_not_found() {
        let signers = create_signers(2);
        let mut state = SlotState::new(1, 2);

        let ranking: Vec<Author> = signers.iter().map(|s| s.author()).collect();
        state.prepare_spc_input(&ranking);

        assert!(state.lookup_payload(&HashValue::random()).is_none());
    }
}
