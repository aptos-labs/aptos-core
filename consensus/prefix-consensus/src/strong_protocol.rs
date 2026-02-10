// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Strong Prefix Consensus Protocol — Pure State Machine
//!
//! This module implements the multi-view decision logic for Strong Prefix Consensus.
//! It is a pure state machine with no async, no I/O, and no networking. The manager
//! (`strong_manager.rs`) drives it by feeding `ViewOutput` values and acting on the
//! returned decisions.
//!
//! ## Responsibilities
//!
//! - **Certificate store**: Stores certificates by hash for trace-back
//! - **View 1 processing**: Sets Strong PC v_low, creates DirectCertificate
//! - **Three-way decision** (views > 1): Commit / DirectCert / EmptyView
//! - **Trace-back**: Builds certificate chain from committing view back to View 1
//! - **StrongPCCommit processing**: Verifies received commit messages
//!
//! ## Modularity
//!
//! The protocol receives `ViewOutput` and makes decisions. It does not know or care
//! how the inner Prefix Consensus was run. This enables swapping the inner algorithm
//! (e.g., optimized 2-round variant) without changing this module.

use crate::certificates::{
    cert_reaches_view1, Certificate, DirectCertificate, HighestKnownView, StrongPCCommit,
    StrongPCCommitError,
};
use crate::certify::qc3_certify;
use crate::types::{PrefixVector, QC3};
use crate::utils::first_non_bot;
use crate::view_state::{has_non_bot_entry, ViewOutput};
use aptos_crypto::HashValue;
use aptos_types::validator_verifier::ValidatorVerifier;
use std::collections::HashMap;
use std::fmt;

// ============================================================================
// Decision Types
// ============================================================================

/// Decision after View 1 completes.
///
/// View 1 always produces a DirectCertificate. Even an all-⊥ v_high is meaningful
/// in View 1 (raw inputs, not certificates). `DirectCertificate::validate()` allows
/// empty v_high only if view == 1.
#[derive(Clone, Debug)]
pub enum View1Decision {
    DirectCert(DirectCertificate),
}

/// Decision after View W > 1 completes (three-way decision).
///
/// Checked in priority order: Commit > DirectCert > EmptyView.
#[derive(Clone, Debug)]
pub enum ViewDecision {
    /// v_low has non-⊥ entry → commit! The manager should trace back to View 1.
    Commit {
        /// QC3 from the committing view — proves v_low had a non-⊥ entry
        committing_proof: QC3,
    },

    /// v_high has non-⊥ entry → create DirectCert, broadcast for next view.
    DirectCert(DirectCertificate),

    /// Both v_low and v_high are all-⊥ → empty view.
    /// Manager should broadcast EmptyViewMessage and collect >1/3 stake.
    EmptyView,
}

// ============================================================================
// Chain Build Error
// ============================================================================

/// Error during certificate chain building (trace-back).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChainBuildError {
    /// A certificate needed for the chain is not in the local store.
    /// The manager should fetch it by hash and retry.
    MissingCert { hash: HashValue },

    /// v_low from committing proof has no non-⊥ entry.
    /// This shouldn't happen if the manager only calls this after a Commit decision.
    NoCommitInVLow,

    /// A cert's v_high has no non-⊥ entry but cert doesn't reach View 1.
    /// This indicates a protocol error or corrupted certificate.
    BrokenChain { cert_view: u64 },
}

impl fmt::Display for ChainBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCert { hash } => {
                write!(f, "Missing certificate in local store: {:?}", hash)
            }
            Self::NoCommitInVLow => {
                write!(f, "v_low from committing proof has no non-⊥ entry")
            }
            Self::BrokenChain { cert_view } => {
                write!(
                    f,
                    "Broken chain: cert at view {} has no non-⊥ in v_high but doesn't reach View 1",
                    cert_view
                )
            }
        }
    }
}

impl std::error::Error for ChainBuildError {}

// ============================================================================
// Strong Prefix Consensus Protocol
// ============================================================================

/// Pure state machine for Strong Prefix Consensus multi-view decisions.
///
/// No async, no I/O. The manager feeds it `ViewOutput` values and acts on the
/// returned `View1Decision` / `ViewDecision` enums.
#[derive(Clone, Debug)]
pub struct StrongPrefixConsensusProtocol {
    /// Epoch for validation
    epoch: u64,

    /// Slot for multi-slot consensus
    slot: u64,

    /// Certificate store: hash → cert (populated by manager as certs arrive)
    cert_store: HashMap<HashValue, Certificate>,

    /// Highest known non-empty view (for empty-view messages).
    /// None until view 1 completes, then always Some (view 1 always sets it).
    highest_known_view: Option<HighestKnownView>,

    /// Strong PC v_low output (set when View 1 completes)
    strong_v_low: Option<PrefixVector>,

    /// Strong PC v_high output (set when commit trace-back succeeds or
    /// a valid StrongPCCommit is received)
    strong_v_high: Option<PrefixVector>,

    /// Whether the protocol has committed (locally or via received StrongPCCommit)
    committed: bool,
}

impl StrongPrefixConsensusProtocol {
    /// Create a new protocol instance.
    pub fn new(epoch: u64, slot: u64) -> Self {
        Self {
            epoch,
            slot,
            cert_store: HashMap::new(),
            highest_known_view: None,
            strong_v_low: None,
            strong_v_high: None,
            committed: false,
        }
    }

    // --- Certificate Store ---

    /// Store a certificate indexed by its hash.
    ///
    /// Called by the manager as certificates arrive (from proposals, local creation,
    /// or fetch responses). The manager should validate the certificate before storing.
    pub fn store_certificate(&mut self, cert: Certificate) {
        let hash = cert.hash();
        self.cert_store.insert(hash, cert);
    }

    /// Look up a certificate by hash.
    pub fn get_certificate(&self, hash: &HashValue) -> Option<&Certificate> {
        self.cert_store.get(hash)
    }

    /// Number of certificates in the store.
    pub fn cert_store_len(&self) -> usize {
        self.cert_store.len()
    }

    // --- View Output Processing ---

    /// Process View 1 completion.
    ///
    /// - Sets `strong_v_low` from `output.v_low` (this is the Strong PC low output).
    /// - Always returns `View1Decision::DirectCert` (View 1 output is always meaningful).
    /// - Updates `highest_known_view` from the output's proof.
    pub fn process_view1_output(&mut self, output: ViewOutput) -> View1Decision {
        // Set Strong PC v_low from View 1
        self.strong_v_low = Some(output.v_low);

        // Update highest known view (View 1's proof)
        self.update_highest_known_view(output.view, output.proof.clone());

        // View 1 always creates a DirectCert
        let cert = DirectCertificate::new(output.view, output.proof);
        View1Decision::DirectCert(cert)
    }

    /// Process View W > 1 completion (three-way decision).
    ///
    /// Priority order:
    /// 1. `has_non_bot_entry(v_low)` → `Commit { committing_proof }`
    /// 2. `has_non_bot_entry(v_high)` → `DirectCert`
    /// 3. Otherwise → `EmptyView`
    ///
    /// For cases 2 and 3, also updates `highest_known_view` if applicable.
    pub fn process_view_output(&mut self, output: ViewOutput) -> ViewDecision {
        // Case (a): v_low has non-⊥ entry → commit
        if has_non_bot_entry(&output.v_low) {
            return ViewDecision::Commit {
                committing_proof: output.proof,
            };
        }

        // Case (b): v_high has non-⊥ entry → create DirectCert
        if has_non_bot_entry(&output.v_high) {
            self.update_highest_known_view(output.view, output.proof.clone());
            let cert = DirectCertificate::new(output.view, output.proof);
            return ViewDecision::DirectCert(cert);
        }

        // Case (c): both all-⊥ → empty view
        // Don't update highest_known_view (nothing meaningful to track)
        ViewDecision::EmptyView
    }

    // --- Trace-Back (Commit) ---

    /// Build certificate chain from committing view's QC3 back to View 1.
    ///
    /// ## Algorithm
    ///
    /// 1. Derive v_low from `committing_proof` via `qc3_certify()`
    /// 2. `first_non_bot(v_low)` → starting cert hash H₀
    /// 3. Look up cert C₀ by H₀ in cert_store
    /// 4. If `cert_reaches_view1(C₀)`: done, return `(C₀.v_high(), [C₀])`
    /// 5. Else: `first_non_bot(C₀.v_high())` → next hash H₁, look up C₁
    /// 6. Repeat until reaching View 1 or encountering a missing cert
    ///
    /// ## Errors
    ///
    /// - `MissingCert { hash }`: A cert is not in the local store. The manager
    ///   should fetch it and retry.
    /// - `NoCommitInVLow`: v_low has no non-⊥ entry (shouldn't happen after Commit decision).
    /// - `BrokenChain`: A cert's v_high has no non-⊥ but cert doesn't reach View 1.
    pub fn build_certificate_chain(
        &self,
        committing_proof: &QC3,
    ) -> Result<(PrefixVector, Vec<Certificate>), ChainBuildError> {
        // 1. Derive v_low from committing proof
        let (v_low, _) = qc3_certify(committing_proof);

        // 2. Find first non-⊥ in v_low (the commit trigger)
        let mut current_hash =
            first_non_bot(&v_low).ok_or(ChainBuildError::NoCommitInVLow)?;

        // 3. Build chain by following hashes
        let mut chain = Vec::new();

        loop {
            // Look up cert by hash
            let cert = self
                .cert_store
                .get(&current_hash)
                .ok_or(ChainBuildError::MissingCert {
                    hash: current_hash,
                })?;

            chain.push(cert.clone());

            let v_high = cert.v_high();

            // Terminal: cert reaches View 1
            if cert_reaches_view1(cert) {
                return Ok((v_high, chain));
            }

            // Follow v_high to next cert
            current_hash = first_non_bot(&v_high).ok_or(ChainBuildError::BrokenChain {
                cert_view: cert.view(),
            })?;
        }
    }

    /// Build a complete `StrongPCCommit` message from a successful chain build.
    ///
    /// Calls `build_certificate_chain` internally. Returns the commit message
    /// ready for broadcast, or an error if the chain can't be built.
    pub fn build_commit_message(
        &self,
        committing_proof: &QC3,
    ) -> Result<StrongPCCommit, ChainBuildError> {
        let (v_high, chain) = self.build_certificate_chain(committing_proof)?;

        Ok(StrongPCCommit::new(
            committing_proof.clone(),
            chain,
            v_high,
            self.epoch,
            self.slot,
        ))
    }

    /// Process a received `StrongPCCommit` from another party.
    ///
    /// Verifies the commit message (delegates to `StrongPCCommit::verify`).
    /// If valid, sets `strong_v_high` and marks as committed.
    ///
    /// TODO(Chunk 3): Manager must filter by slot before calling this.
    /// `StrongPCCommit::verify` does not check slot numbers — the manager
    /// must reject cross-slot replays. Same applies to `HighestKnownView`
    /// proofs in EmptyViewMessages.
    pub fn process_received_commit(
        &mut self,
        commit: &StrongPCCommit,
        verifier: &ValidatorVerifier,
    ) -> Result<(), StrongPCCommitError> {
        commit.verify(verifier)?;
        self.strong_v_high = Some(commit.v_high.clone());
        self.committed = true;
        Ok(())
    }

    /// Mark as committed with the given v_high (after successful local trace-back).
    ///
    /// Called by the manager after `build_commit_message` succeeds and the
    /// `StrongPCCommit` has been broadcast.
    pub fn set_committed(&mut self, v_high: PrefixVector) {
        self.strong_v_high = Some(v_high);
        self.committed = true;
    }

    // --- Highest Known View ---

    /// Update highest known view if this view's output is higher.
    ///
    /// Called by the manager when receiving certs from other parties,
    /// and internally after processing view outputs.
    pub fn update_highest_known_view(&mut self, view: u64, proof: QC3) {
        match &mut self.highest_known_view {
            Some(hk) => {
                hk.update_if_higher(view, proof);
            }
            None => {
                self.highest_known_view = Some(HighestKnownView::new(view, proof));
            }
        }
    }

    /// Get current highest known view info (for building EmptyViewMessages).
    ///
    /// Returns `None` if no view with a valid v_high has completed yet.
    pub fn highest_known_view(&self) -> Option<&HighestKnownView> {
        self.highest_known_view.as_ref()
    }

    // --- Output Queries ---

    /// Strong PC is complete when v_high has been determined.
    pub fn is_complete(&self) -> bool {
        self.committed
    }

    /// Get Strong PC v_low output (from View 1).
    pub fn v_low(&self) -> Option<&PrefixVector> {
        self.strong_v_low.as_ref()
    }

    /// Get Strong PC v_high output (from commit trace-back).
    pub fn v_high(&self) -> Option<&PrefixVector> {
        self.strong_v_high.as_ref()
    }

    /// Get epoch.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Get slot.
    pub fn slot(&self) -> u64 {
        self.slot
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Vote3, QC2};
    use aptos_crypto::bls12381::Signature as BlsSignature;
    use aptos_types::account_address::AccountAddress;

    // ========================================================================
    // Test Helpers
    // ========================================================================

    fn hash(n: u64) -> HashValue {
        HashValue::sha3_256_of(&n.to_le_bytes())
    }

    /// Create a QC3 whose qc3_certify() produces the given (v_low, v_high).
    ///
    /// We do this by creating a single Vote3 with the desired prefix, then
    /// wrapping it in a QC3. Since qc3_certify with a single vote produces
    /// (mcp, mce) = (prefix, prefix), both v_low and v_high will be the same
    /// as the vote's prefix.
    fn qc3_with_prefix(prefix: Vec<HashValue>, view: u64) -> QC3 {
        let vote = Vote3::new(
            AccountAddress::random(),
            prefix,
            QC2 { votes: vec![], authors: vec![] },
            1,     // epoch
            0,     // slot
            view,
            BlsSignature::dummy_signature(),
        );
        QC3::new(vec![vote])
    }

    /// Create a DirectCertificate for the given view whose v_high is the given prefix.
    fn make_direct_cert(view: u64, v_high: Vec<HashValue>) -> DirectCertificate {
        let qc3 = qc3_with_prefix(v_high, view);
        DirectCertificate::new(view, qc3)
    }

    fn make_view_output(
        view: u64,
        v_low: Vec<HashValue>,
        v_high: Vec<HashValue>,
    ) -> ViewOutput {
        // Use v_high for the proof's prefix so cert.v_high() matches
        let proof = qc3_with_prefix(v_high.clone(), view);
        ViewOutput::new(view, 0, v_low, v_high, proof)
    }

    // ========================================================================
    // Constructor Tests
    // ========================================================================

    #[test]
    fn test_new_protocol() {
        let proto = StrongPrefixConsensusProtocol::new(1, 42);

        assert_eq!(proto.epoch(), 1);
        assert_eq!(proto.slot(), 42);
        assert!(!proto.is_complete());
        assert!(proto.v_low().is_none());
        assert!(proto.v_high().is_none());
        assert_eq!(proto.cert_store_len(), 0);
    }

    // ========================================================================
    // Certificate Store Tests
    // ========================================================================

    #[test]
    fn test_store_and_retrieve_certificate() {
        let mut proto = StrongPrefixConsensusProtocol::new(1, 0);

        let cert = Certificate::Direct(make_direct_cert(1, vec![hash(1), hash(2)]));
        let cert_hash = cert.hash();

        proto.store_certificate(cert.clone());

        assert_eq!(proto.cert_store_len(), 1);
        let retrieved = proto.get_certificate(&cert_hash).unwrap();
        assert_eq!(retrieved.hash(), cert_hash);
    }

    #[test]
    fn test_get_missing_certificate() {
        let proto = StrongPrefixConsensusProtocol::new(1, 0);
        assert!(proto.get_certificate(&HashValue::random()).is_none());
    }

    #[test]
    fn test_store_multiple_certificates() {
        let mut proto = StrongPrefixConsensusProtocol::new(1, 0);

        let cert1 = Certificate::Direct(make_direct_cert(1, vec![hash(1)]));
        let cert2 = Certificate::Direct(make_direct_cert(2, vec![hash(2)]));

        proto.store_certificate(cert1);
        proto.store_certificate(cert2);

        assert_eq!(proto.cert_store_len(), 2);
    }

    // ========================================================================
    // View 1 Decision Tests
    // ========================================================================

    #[test]
    fn test_view1_decision_sets_v_low() {
        let mut proto = StrongPrefixConsensusProtocol::new(1, 0);

        let v_low = vec![hash(10), hash(20)];
        let output = make_view_output(1, v_low.clone(), vec![hash(30)]);

        let _decision = proto.process_view1_output(output);

        assert_eq!(proto.v_low().unwrap(), &v_low);
    }

    #[test]
    fn test_view1_decision_always_returns_direct_cert() {
        let mut proto = StrongPrefixConsensusProtocol::new(1, 0);

        let output = make_view_output(1, vec![hash(10)], vec![hash(30)]);
        let decision = proto.process_view1_output(output);

        match decision {
            View1Decision::DirectCert(cert) => {
                assert_eq!(cert.view(), 1);
            }
        }
    }

    #[test]
    fn test_view1_decision_all_bot_v_high_still_creates_cert() {
        let mut proto = StrongPrefixConsensusProtocol::new(1, 0);

        // All-⊥ v_high is meaningful in View 1
        let output = make_view_output(
            1,
            vec![HashValue::zero()],
            vec![HashValue::zero(), HashValue::zero()],
        );
        let decision = proto.process_view1_output(output);

        match decision {
            View1Decision::DirectCert(cert) => {
                assert_eq!(cert.view(), 1);
            }
        }
    }

    #[test]
    fn test_view1_updates_highest_known_view() {
        let mut proto = StrongPrefixConsensusProtocol::new(1, 0);

        let output = make_view_output(1, vec![hash(10)], vec![hash(30)]);
        let _decision = proto.process_view1_output(output);

        assert_eq!(proto.highest_known_view().map(|hk| hk.view), Some(1));
    }

    // ========================================================================
    // Three-Way Decision Tests (Views > 1)
    // ========================================================================

    #[test]
    fn test_view_decision_commit() {
        let mut proto = StrongPrefixConsensusProtocol::new(1, 0);

        // v_low has non-⊥ entry → commit
        let output = make_view_output(
            3,
            vec![HashValue::zero(), hash(42)], // non-⊥ at position 1
            vec![hash(99)],
        );
        let decision = proto.process_view_output(output);

        match decision {
            ViewDecision::Commit { committing_proof } => {
                assert!(!committing_proof.votes.is_empty());
            }
            _ => panic!("Expected Commit decision"),
        }
    }

    #[test]
    fn test_view_decision_direct_cert() {
        let mut proto = StrongPrefixConsensusProtocol::new(1, 0);

        // v_low all-⊥, v_high has non-⊥ → DirectCert
        let output = make_view_output(
            3,
            vec![HashValue::zero(), HashValue::zero()],
            vec![hash(55)],
        );
        let decision = proto.process_view_output(output);

        match decision {
            ViewDecision::DirectCert(cert) => {
                assert_eq!(cert.view(), 3);
            }
            _ => panic!("Expected DirectCert decision"),
        }
    }

    #[test]
    fn test_view_decision_empty_view() {
        let mut proto = StrongPrefixConsensusProtocol::new(1, 0);

        // Both v_low and v_high are all-⊥ → EmptyView
        let output = make_view_output(
            3,
            vec![HashValue::zero(), HashValue::zero()],
            vec![HashValue::zero(), HashValue::zero()],
        );
        let decision = proto.process_view_output(output);

        match decision {
            ViewDecision::EmptyView => {}
            _ => panic!("Expected EmptyView decision"),
        }
    }

    #[test]
    fn test_view_decision_commit_takes_priority_over_cert() {
        let mut proto = StrongPrefixConsensusProtocol::new(1, 0);

        // Both v_low and v_high have non-⊥ → Commit (priority)
        let output = make_view_output(3, vec![hash(1)], vec![hash(2)]);
        let decision = proto.process_view_output(output);

        match decision {
            ViewDecision::Commit { .. } => {}
            _ => panic!("Expected Commit decision (priority over DirectCert)"),
        }
    }

    #[test]
    fn test_view_decision_direct_cert_updates_highest_known_view() {
        let mut proto = StrongPrefixConsensusProtocol::new(1, 0);

        // First: view 1
        let output1 = make_view_output(1, vec![hash(10)], vec![hash(30)]);
        let _d = proto.process_view1_output(output1);
        assert_eq!(proto.highest_known_view().map(|hk| hk.view), Some(1));

        // View 3 with non-⊥ v_high → DirectCert → updates highest_known_view
        let output3 = make_view_output(
            3,
            vec![HashValue::zero()],
            vec![hash(55)],
        );
        let _d = proto.process_view_output(output3);
        assert_eq!(proto.highest_known_view().map(|hk| hk.view), Some(3));
    }

    #[test]
    fn test_view_decision_empty_view_does_not_update_highest_known_view() {
        let mut proto = StrongPrefixConsensusProtocol::new(1, 0);

        // View 1
        let output1 = make_view_output(1, vec![hash(10)], vec![hash(30)]);
        let _d = proto.process_view1_output(output1);
        assert_eq!(proto.highest_known_view().map(|hk| hk.view), Some(1));

        // View 3 all-⊥ → EmptyView → highest_known_view stays at 1
        let output3 = make_view_output(
            3,
            vec![HashValue::zero()],
            vec![HashValue::zero()],
        );
        let _d = proto.process_view_output(output3);
        assert_eq!(proto.highest_known_view().map(|hk| hk.view), Some(1));
    }

    // ========================================================================
    // Trace-Back (Chain Build) Tests
    // ========================================================================

    #[test]
    fn test_trace_back_single_hop() {
        // View 2 commits → chain[0] = DirectCert(V=1) → terminal
        let mut proto = StrongPrefixConsensusProtocol::new(1, 0);

        // Create DirectCert(V=1) with v_high = [hash(100), hash(200)]
        let v1_output = vec![hash(100), hash(200)];
        let cert_v1 = Certificate::Direct(make_direct_cert(1, v1_output.clone()));
        let cert_v1_hash = cert_v1.hash();
        proto.store_certificate(cert_v1);

        // Committing proof: v_low contains hash of cert_v1
        let committing_proof = qc3_with_prefix(vec![cert_v1_hash], 2);

        let (v_high, chain) = proto.build_certificate_chain(&committing_proof).unwrap();

        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].view(), 1);
        assert_eq!(v_high, v1_output);
    }

    #[test]
    fn test_trace_back_two_hops() {
        // View 3 commits → chain[0] = DirectCert(V=2) → chain[1] = DirectCert(V=1)
        let mut proto = StrongPrefixConsensusProtocol::new(1, 0);

        // Create DirectCert(V=1)
        let v1_output = vec![hash(100)];
        let cert_v1 = Certificate::Direct(make_direct_cert(1, v1_output.clone()));
        let cert_v1_hash = cert_v1.hash();
        proto.store_certificate(cert_v1);

        // Create DirectCert(V=2) whose v_high points to cert_v1
        let cert_v2 = Certificate::Direct(make_direct_cert(2, vec![cert_v1_hash]));
        let cert_v2_hash = cert_v2.hash();
        proto.store_certificate(cert_v2);

        // Committing proof: v_low contains hash of cert_v2
        let committing_proof = qc3_with_prefix(vec![cert_v2_hash], 3);

        let (v_high, chain) = proto.build_certificate_chain(&committing_proof).unwrap();

        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].view(), 2);
        assert_eq!(chain[1].view(), 1);
        assert_eq!(v_high, v1_output);
    }

    #[test]
    fn test_trace_back_three_hops() {
        // View 4 → cert_v3 → cert_v2 → cert_v1
        let mut proto = StrongPrefixConsensusProtocol::new(1, 0);

        let v1_output = vec![hash(7), hash(8)];
        let cert_v1 = Certificate::Direct(make_direct_cert(1, v1_output.clone()));
        let cert_v1_hash = cert_v1.hash();
        proto.store_certificate(cert_v1);

        let cert_v2 = Certificate::Direct(make_direct_cert(2, vec![cert_v1_hash]));
        let cert_v2_hash = cert_v2.hash();
        proto.store_certificate(cert_v2);

        let cert_v3 = Certificate::Direct(make_direct_cert(3, vec![cert_v2_hash]));
        let cert_v3_hash = cert_v3.hash();
        proto.store_certificate(cert_v3);

        let committing_proof = qc3_with_prefix(vec![cert_v3_hash], 4);

        let (v_high, chain) = proto.build_certificate_chain(&committing_proof).unwrap();

        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0].view(), 3);
        assert_eq!(chain[1].view(), 2);
        assert_eq!(chain[2].view(), 1);
        assert_eq!(v_high, v1_output);
    }

    #[test]
    fn test_trace_back_missing_cert() {
        let proto = StrongPrefixConsensusProtocol::new(1, 0);

        // Committing proof points to a cert we don't have
        let missing_hash = hash(999);
        let committing_proof = qc3_with_prefix(vec![missing_hash], 2);

        let err = proto.build_certificate_chain(&committing_proof).unwrap_err();
        assert_eq!(err, ChainBuildError::MissingCert { hash: missing_hash });
    }

    #[test]
    fn test_trace_back_missing_intermediate_cert() {
        // Have cert_v2 but not cert_v1 that it points to
        let mut proto = StrongPrefixConsensusProtocol::new(1, 0);

        let missing_hash = hash(888);
        // cert_v2's v_high points to a cert we don't have
        let cert_v2 = Certificate::Direct(make_direct_cert(2, vec![missing_hash]));
        let cert_v2_hash = cert_v2.hash();
        proto.store_certificate(cert_v2);

        let committing_proof = qc3_with_prefix(vec![cert_v2_hash], 3);

        let err = proto.build_certificate_chain(&committing_proof).unwrap_err();
        assert_eq!(err, ChainBuildError::MissingCert { hash: missing_hash });
    }

    #[test]
    fn test_trace_back_no_commit_in_v_low() {
        let proto = StrongPrefixConsensusProtocol::new(1, 0);

        // v_low is all-⊥
        let committing_proof =
            qc3_with_prefix(vec![HashValue::zero(), HashValue::zero()], 2);

        let err = proto.build_certificate_chain(&committing_proof).unwrap_err();
        assert_eq!(err, ChainBuildError::NoCommitInVLow);
    }

    #[test]
    fn test_trace_back_v_low_with_leading_bots() {
        // v_low = [⊥, ⊥, cert_hash] — first non-⊥ is at position 2
        let mut proto = StrongPrefixConsensusProtocol::new(1, 0);

        let v1_output = vec![hash(42)];
        let cert_v1 = Certificate::Direct(make_direct_cert(1, v1_output.clone()));
        let cert_v1_hash = cert_v1.hash();
        proto.store_certificate(cert_v1);

        let committing_proof = qc3_with_prefix(
            vec![HashValue::zero(), HashValue::zero(), cert_v1_hash],
            2,
        );

        let (v_high, chain) = proto.build_certificate_chain(&committing_proof).unwrap();
        assert_eq!(chain.len(), 1);
        assert_eq!(v_high, v1_output);
    }

    // ========================================================================
    // Build Commit Message Tests
    // ========================================================================

    #[test]
    fn test_build_commit_message_success() {
        let mut proto = StrongPrefixConsensusProtocol::new(5, 3);

        let v1_output = vec![hash(100)];
        let cert_v1 = Certificate::Direct(make_direct_cert(1, v1_output.clone()));
        let cert_v1_hash = cert_v1.hash();
        proto.store_certificate(cert_v1);

        let committing_proof = qc3_with_prefix(vec![cert_v1_hash], 2);

        let commit = proto.build_commit_message(&committing_proof).unwrap();

        assert_eq!(commit.v_high, v1_output);
        assert_eq!(commit.certificate_chain.len(), 1);
        assert_eq!(commit.epoch, 5);
        assert_eq!(commit.slot, 3);
    }

    #[test]
    fn test_build_commit_message_propagates_error() {
        let proto = StrongPrefixConsensusProtocol::new(1, 0);

        let committing_proof = qc3_with_prefix(vec![hash(999)], 2);

        let err = proto.build_commit_message(&committing_proof).unwrap_err();
        match err {
            ChainBuildError::MissingCert { .. } => {}
            _ => panic!("Expected MissingCert error"),
        }
    }

    // ========================================================================
    // Commit State Tests
    // ========================================================================

    #[test]
    fn test_set_committed() {
        let mut proto = StrongPrefixConsensusProtocol::new(1, 0);

        assert!(!proto.is_complete());
        assert!(proto.v_high().is_none());

        let v_high = vec![hash(1), hash(2)];
        proto.set_committed(v_high.clone());

        assert!(proto.is_complete());
        assert_eq!(proto.v_high().unwrap(), &v_high);
    }

    #[test]
    fn test_not_complete_without_commit() {
        let mut proto = StrongPrefixConsensusProtocol::new(1, 0);

        // Process View 1 — sets v_low but NOT committed
        let output = make_view_output(1, vec![hash(10)], vec![hash(20)]);
        let _d = proto.process_view1_output(output);

        assert!(!proto.is_complete());
        assert!(proto.v_low().is_some());
        assert!(proto.v_high().is_none());
    }

    // ========================================================================
    // Highest Known View Tests
    // ========================================================================

    #[test]
    fn test_update_highest_known_view_external() {
        let mut proto = StrongPrefixConsensusProtocol::new(1, 0);

        assert!(proto.highest_known_view().is_none());

        // Use QC3 with non-empty prefix (update_if_higher requires valid v_high for views > 1)
        proto.update_highest_known_view(3, qc3_with_prefix(vec![hash(1)], 3));
        assert_eq!(proto.highest_known_view().map(|hk| hk.view), Some(3));

        // Lower view doesn't update
        proto.update_highest_known_view(2, qc3_with_prefix(vec![hash(2)], 2));
        assert_eq!(proto.highest_known_view().map(|hk| hk.view), Some(3));

        // Higher view does update
        proto.update_highest_known_view(5, qc3_with_prefix(vec![hash(3)], 5));
        assert_eq!(proto.highest_known_view().map(|hk| hk.view), Some(5));
    }

    // ========================================================================
    // Error Display Tests
    // ========================================================================

    #[test]
    fn test_chain_build_error_display() {
        let err = ChainBuildError::MissingCert {
            hash: HashValue::zero(),
        };
        let s = format!("{}", err);
        assert!(s.contains("Missing certificate"));

        let err = ChainBuildError::NoCommitInVLow;
        let s = format!("{}", err);
        assert!(s.contains("non-⊥"));

        let err = ChainBuildError::BrokenChain { cert_view: 4 };
        let s = format!("{}", err);
        assert!(s.contains("view 4"));
    }
}
