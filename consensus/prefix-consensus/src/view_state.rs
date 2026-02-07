// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! View State and Ranking Management for Strong Prefix Consensus
//!
//! This module provides infrastructure for multi-view Strong Prefix Consensus:
//! - `RankingManager`: Cyclic ranking that shifts each view for leaderless progress
//! - `ViewState`: Per-view state tracking (for views > 1 only)
//! - `ViewOutput`: Output from completing a view's Prefix Consensus
//!
//! Note: View 1 uses `PrefixConsensusProtocol` directly with raw input vectors.
//! `ViewState` is only for views 2+ where parties exchange certificates.

use crate::{
    certificates::Certificate,
    types::{PartyId, PrefixVector, QC3},
};
use aptos_crypto::HashValue;
use std::collections::HashMap;

// ============================================================================
// Helper functions for three-way decision in views > 1
// ============================================================================

/// Check if a vector has at least one non-⊥ entry (for views > 1)
///
/// Returns true if any entry is not `HashValue::zero()` (the ⊥ marker).
///
/// **Important**: In views > 1, a vector like `[⊥, ⊥, ⊥]` is NOT meaningful
/// (no certificate to trace back to View 1). This function distinguishes
/// between vectors with actual content and all-⊥ vectors.
///
/// **Note**: View 1 is special - even all-⊥ outputs are valid there
/// (raw inputs, not certificates). Do NOT use this for View 1 decision logic.
pub fn has_non_bot_entry(vector: &PrefixVector) -> bool {
    vector.iter().any(|h| *h != HashValue::zero())
}

/// Check if v_low has at least one non-⊥ entry (can commit)
///
/// Semantic alias for `has_non_bot_entry` used in commit decision logic.
/// If true, the view's v_low contains a certificate that can be traced
/// back to View 1 for commitment.
pub fn has_committable_low(v_low: &PrefixVector) -> bool {
    has_non_bot_entry(v_low)
}

/// Check if v_high has at least one non-⊥ entry (can create DirectCertificate)
///
/// Semantic alias for `has_non_bot_entry` used in certificate creation logic.
/// If true, the view's v_high contains a certificate that can be used to
/// create a DirectCertificate for the next view.
pub fn has_certifiable_high(v_high: &PrefixVector) -> bool {
    has_non_bot_entry(v_high)
}

// ============================================================================
// RankingManager
// ============================================================================

/// Manages validator rankings across views
///
/// Rankings shift cyclically each view to ensure leaderless progress.
/// View 1: [p1, p2, p3, p4]
/// View 2: [p2, p3, p4, p1]  (rotate left by 1)
/// View 3: [p3, p4, p1, p2]  (rotate left by 2)
/// etc.
///
/// This ensures that even if an adversary suspends one party per round,
/// eventually honest parties occupy the first positions.
#[derive(Clone, Debug)]
pub struct RankingManager {
    /// Initial ranking (view 1). Typically sorted by validator address or stake.
    initial_ranking: Vec<PartyId>,
}

impl RankingManager {
    /// Create with initial ranking (typically sorted by validator address or stake)
    ///
    /// The sorting order is the caller's responsibility.
    pub fn new(initial_ranking: Vec<PartyId>) -> Self {
        Self { initial_ranking }
    }

    /// Get ranking for a specific view (cyclic shift)
    ///
    /// For view v, rotate left by (v - 1) % n positions.
    /// View 1: no rotation (original order)
    /// View 2: rotate left by 1
    /// View 3: rotate left by 2
    /// etc.
    pub fn get_ranking_for_view(&self, view: u64) -> Vec<PartyId> {
        if self.initial_ranking.is_empty() {
            return Vec::new();
        }

        let n = self.initial_ranking.len();
        let rotation = ((view.saturating_sub(1)) % n as u64) as usize;

        // Rotate left by `rotation` positions
        let mut result = Vec::with_capacity(n);
        for i in 0..n {
            result.push(self.initial_ranking[(i + rotation) % n]);
        }
        result
    }

    /// Get position of a party in the ranking for a view
    ///
    /// Returns None if the party is not in the ranking.
    pub fn get_position(&self, view: u64, party: &PartyId) -> Option<usize> {
        let ranking = self.get_ranking_for_view(view);
        ranking.iter().position(|p| p == party)
    }

    /// Number of validators
    pub fn len(&self) -> usize {
        self.initial_ranking.len()
    }

    /// Check if ranking is empty
    pub fn is_empty(&self) -> bool {
        self.initial_ranking.is_empty()
    }

    /// Get the initial ranking (view 1)
    pub fn initial_ranking(&self) -> &[PartyId] {
        &self.initial_ranking
    }
}

/// Output from completing a view's Prefix Consensus
#[derive(Clone, Debug)]
pub struct ViewOutput {
    /// View number this output is from
    pub view: u64,
    /// Slot number (for future multi-slot consensus)
    pub slot: u64,
    /// Lower bound vector (mcp of inputs)
    pub v_low: PrefixVector,
    /// Upper bound vector (mce of inputs)
    pub v_high: PrefixVector,
    /// Proof of the output (QC3 from the Prefix Consensus)
    pub proof: QC3,
}

impl ViewOutput {
    /// Create a new ViewOutput
    pub fn new(
        view: u64,
        slot: u64,
        v_low: PrefixVector,
        v_high: PrefixVector,
        proof: QC3,
    ) -> Self {
        Self {
            view,
            slot,
            v_low,
            v_high,
            proof,
        }
    }

    /// Check if v_high vector has length > 0
    ///
    /// **Warning**: This only checks vector length, NOT content. A vector like
    /// `[⊥, ⊥, ⊥]` will return `true` but has no traceable certificate.
    ///
    /// For views > 1, use `has_certifiable_high(&output.v_high)` instead to
    /// check if the vector contains at least one actual certificate (non-⊥).
    ///
    /// For View 1, this check is appropriate since even all-⊥ outputs are
    /// meaningful (raw inputs conflict at position 0).
    pub fn has_non_empty_high(&self) -> bool {
        !self.v_high.is_empty()
    }
}

/// Per-view state for Strong Prefix Consensus (views > 1 only)
///
/// View 1 uses `PrefixConsensusProtocol` directly with raw input vectors.
/// `ViewState` is for views 2+ where parties exchange certificates.
///
/// **Certificate Validation**: Certificates must be validated by the caller
/// BEFORE calling `add_certificate()`. ViewState does not validate certificates
/// (it doesn't have access to ValidatorVerifier).
#[derive(Clone, Debug)]
pub struct ViewState {
    /// View number (must be > 1)
    view: u64,

    /// Slot number (for future multi-slot consensus)
    slot: u64,

    /// Ranking for this view (computed from RankingManager)
    ranking: Vec<PartyId>,

    /// Received certificates from other parties
    /// Key: party that sent the certificate
    /// Value: the certificate they sent
    received_certificates: HashMap<PartyId, Certificate>,

    /// Output once view completes (set by set_output)
    output: Option<ViewOutput>,
}

impl ViewState {
    /// Create new view state
    ///
    /// # Arguments
    /// * `view` - View number (must be > 1; view 1 uses raw input vectors)
    /// * `slot` - Slot number for multi-slot consensus
    /// * `ranking` - Ranking for this view (from RankingManager)
    ///
    /// # Panics
    /// Panics if view <= 1 (use PrefixConsensusProtocol directly for view 1)
    pub fn new(view: u64, slot: u64, ranking: Vec<PartyId>) -> Self {
        assert!(
            view > 1,
            "ViewState is for views > 1 only; view 1 uses PrefixConsensusProtocol directly"
        );
        Self {
            view,
            slot,
            ranking,
            received_certificates: HashMap::new(),
            output: None,
        }
    }

    /// View number
    pub fn view(&self) -> u64 {
        self.view
    }

    /// Slot number
    pub fn slot(&self) -> u64 {
        self.slot
    }

    /// Get the ranking for this view
    pub fn ranking(&self) -> &[PartyId] {
        &self.ranking
    }

    /// Add a certificate received from a party
    ///
    /// Returns false if we already have a certificate from this party.
    ///
    /// **IMPORTANT**: Caller must validate certificate before adding.
    /// ViewState does not have access to ValidatorVerifier for validation.
    pub fn add_certificate(&mut self, from: PartyId, cert: Certificate) -> bool {
        if self.has_certificate_from(&from) {
            return false;
        }
        self.received_certificates.insert(from, cert);
        true
    }

    /// Check if we have a certificate from a party
    pub fn has_certificate_from(&self, party: &PartyId) -> bool {
        self.received_certificates.contains_key(party)
    }

    /// Number of certificates received
    pub fn certificate_count(&self) -> usize {
        self.received_certificates.len()
    }

    /// Get all received certificates
    pub fn certificates(&self) -> &HashMap<PartyId, Certificate> {
        &self.received_certificates
    }

    /// Build truncated input vector for Prefix Consensus
    ///
    /// Algorithm (optimization not in paper):
    /// 1. Create vector ordered by ranking: [cert_rank0, cert_rank1, ...]
    /// 2. Find first non-⊥ position (first party we have a certificate from)
    /// 3. Truncate: return [⊥, ..., ⊥, cert_hash] (only up to first non-⊥)
    ///
    /// This optimization works because only the first non-⊥ entry is used
    /// when tracing back to View 1. Entries after first non-⊥ are provably
    /// unused in trace-back logic.
    ///
    /// Uses `HashValue::zero()` as ⊥ (bottom/empty) marker.
    pub fn build_truncated_input_vector(&self) -> PrefixVector {
        let mut result = Vec::new();

        for party in &self.ranking {
            if let Some(cert) = self.received_certificates.get(party) {
                // Found first non-⊥: add hash and stop
                result.push(cert.hash());
                return result;
            } else {
                // No certificate from this party: add ⊥ (empty hash)
                result.push(HashValue::zero());
            }
        }

        // All ⊥ case: return full vector of zeros
        result
    }

    /// Get the certificate at the first non-⊥ position (for trace-back)
    ///
    /// Used for trace-back: when v_low contains a certificate hash at position k,
    /// that corresponds to ranking[k]'s certificate. This method returns it directly.
    ///
    /// Returns None if no certificates have been received.
    pub fn get_first_certificate(&self) -> Option<&Certificate> {
        for party in &self.ranking {
            if let Some(cert) = self.received_certificates.get(party) {
                return Some(cert);
            }
        }
        None
    }

    /// Get the position of the first non-⊥ entry in the ranking
    ///
    /// Returns None if no certificates have been received.
    pub fn get_first_certificate_position(&self) -> Option<usize> {
        for (i, party) in self.ranking.iter().enumerate() {
            if self.received_certificates.contains_key(party) {
                return Some(i);
            }
        }
        None
    }

    /// Set the output for this view
    pub fn set_output(&mut self, output: ViewOutput) {
        self.output = Some(output);
    }

    /// Get output if view is complete
    pub fn output(&self) -> Option<&ViewOutput> {
        self.output.as_ref()
    }

    /// Check if view is complete
    pub fn is_complete(&self) -> bool {
        self.output.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::certificates::DirectCertificate;

    fn party(id: u8) -> PartyId {
        PartyId::new([id; 32])
    }

    // ==================== Helper Function Tests ====================

    #[test]
    fn test_has_non_bot_entry_with_content() {
        let v = vec![HashValue::zero(), HashValue::random(), HashValue::zero()];
        assert!(has_non_bot_entry(&v));
    }

    #[test]
    fn test_has_non_bot_entry_all_bots() {
        let v = vec![HashValue::zero(), HashValue::zero(), HashValue::zero()];
        assert!(!has_non_bot_entry(&v));
    }

    #[test]
    fn test_has_non_bot_entry_empty_vector() {
        let v: PrefixVector = vec![];
        assert!(!has_non_bot_entry(&v));
    }

    #[test]
    fn test_has_non_bot_entry_single_non_bot() {
        let v = vec![HashValue::random()];
        assert!(has_non_bot_entry(&v));
    }

    #[test]
    fn test_has_non_bot_entry_single_bot() {
        let v = vec![HashValue::zero()];
        assert!(!has_non_bot_entry(&v));
    }

    #[test]
    fn test_has_committable_low_alias() {
        let v_with_content = vec![HashValue::zero(), HashValue::random()];
        let v_all_bots = vec![HashValue::zero(), HashValue::zero()];

        assert!(has_committable_low(&v_with_content));
        assert!(!has_committable_low(&v_all_bots));
        // Verify it's the same as has_non_bot_entry
        assert_eq!(has_committable_low(&v_with_content), has_non_bot_entry(&v_with_content));
        assert_eq!(has_committable_low(&v_all_bots), has_non_bot_entry(&v_all_bots));
    }

    #[test]
    fn test_has_certifiable_high_alias() {
        let v_with_content = vec![HashValue::random(), HashValue::zero()];
        let v_all_bots = vec![HashValue::zero(), HashValue::zero()];

        assert!(has_certifiable_high(&v_with_content));
        assert!(!has_certifiable_high(&v_all_bots));
        // Verify it's the same as has_non_bot_entry
        assert_eq!(has_certifiable_high(&v_with_content), has_non_bot_entry(&v_with_content));
        assert_eq!(has_certifiable_high(&v_all_bots), has_non_bot_entry(&v_all_bots));
    }

    // ==================== RankingManager Tests ====================

    #[test]
    fn test_ranking_view_1_is_initial() {
        let ranking = vec![party(1), party(2), party(3), party(4)];
        let manager = RankingManager::new(ranking.clone());

        assert_eq!(manager.get_ranking_for_view(1), ranking);
    }

    #[test]
    fn test_ranking_cyclic_shift() {
        let ranking = vec![party(1), party(2), party(3), party(4)];
        let manager = RankingManager::new(ranking);

        // View 2: rotate left by 1
        assert_eq!(
            manager.get_ranking_for_view(2),
            vec![party(2), party(3), party(4), party(1)]
        );

        // View 3: rotate left by 2
        assert_eq!(
            manager.get_ranking_for_view(3),
            vec![party(3), party(4), party(1), party(2)]
        );

        // View 4: rotate left by 3
        assert_eq!(
            manager.get_ranking_for_view(4),
            vec![party(4), party(1), party(2), party(3)]
        );
    }

    #[test]
    fn test_ranking_wraps_around() {
        let ranking = vec![party(1), party(2), party(3), party(4)];
        let manager = RankingManager::new(ranking.clone());

        // View 5: rotate left by 4 (same as view 1)
        assert_eq!(manager.get_ranking_for_view(5), ranking);

        // View 9: rotate left by 8 (same as view 1)
        assert_eq!(manager.get_ranking_for_view(9), ranking);
    }

    #[test]
    fn test_get_position() {
        let ranking = vec![party(1), party(2), party(3), party(4)];
        let manager = RankingManager::new(ranking);

        // View 1: party(1) is at position 0
        assert_eq!(manager.get_position(1, &party(1)), Some(0));
        assert_eq!(manager.get_position(1, &party(2)), Some(1));
        assert_eq!(manager.get_position(1, &party(4)), Some(3));

        // View 2: party(1) is at position 3 (rotated)
        assert_eq!(manager.get_position(2, &party(1)), Some(3));
        assert_eq!(manager.get_position(2, &party(2)), Some(0));

        // Non-existent party
        assert_eq!(manager.get_position(1, &party(99)), None);
    }

    #[test]
    fn test_ranking_with_different_sizes() {
        // Size 1
        let manager1 = RankingManager::new(vec![party(1)]);
        assert_eq!(manager1.get_ranking_for_view(1), vec![party(1)]);
        assert_eq!(manager1.get_ranking_for_view(2), vec![party(1)]); // Still the same

        // Size 2
        let manager2 = RankingManager::new(vec![party(1), party(2)]);
        assert_eq!(manager2.get_ranking_for_view(1), vec![party(1), party(2)]);
        assert_eq!(manager2.get_ranking_for_view(2), vec![party(2), party(1)]);
        assert_eq!(manager2.get_ranking_for_view(3), vec![party(1), party(2)]); // Wraps

        // Empty
        let manager_empty = RankingManager::new(vec![]);
        assert_eq!(manager_empty.get_ranking_for_view(1), Vec::<PartyId>::new());
        assert!(manager_empty.is_empty());
    }

    #[test]
    fn test_ranking_len() {
        let manager = RankingManager::new(vec![party(1), party(2), party(3)]);
        assert_eq!(manager.len(), 3);
        assert!(!manager.is_empty());
    }

    // ==================== ViewState Tests ====================

    fn create_test_direct_certificate(view: u64) -> Certificate {
        use crate::types::{Vote3, QC2};
        use aptos_crypto::bls12381::Signature;

        // Create minimal certificate for testing
        // In real usage, certificates would be properly constructed with valid signatures
        let qc2 = QC2::new(vec![]);
        let vote3 = Vote3::new(
            party(1),
            vec![HashValue::random()],
            qc2,
            1,    // epoch
            0,    // slot
            view, // view
            Signature::dummy_signature(),
        );
        let qc3 = QC3::new(vec![vote3]);
        Certificate::Direct(DirectCertificate::new(view, qc3))
    }

    #[test]
    fn test_view_state_new() {
        let ranking = vec![party(1), party(2), party(3), party(4)];
        let state = ViewState::new(2, 0, ranking.clone());

        assert_eq!(state.view(), 2);
        assert_eq!(state.slot(), 0);
        assert_eq!(state.ranking(), ranking.as_slice());
        assert_eq!(state.certificate_count(), 0);
        assert!(!state.is_complete());
    }

    #[test]
    #[should_panic(expected = "ViewState is for views > 1 only")]
    fn test_view_state_rejects_view_1() {
        let ranking = vec![party(1), party(2)];
        let _ = ViewState::new(1, 0, ranking);
    }

    #[test]
    fn test_add_certificate() {
        let ranking = vec![party(1), party(2), party(3), party(4)];
        let mut state = ViewState::new(2, 0, ranking);

        let cert = create_test_direct_certificate(1);

        // First add should succeed
        assert!(state.add_certificate(party(1), cert.clone()));
        assert_eq!(state.certificate_count(), 1);
        assert!(state.has_certificate_from(&party(1)));

        // Second add from same party should fail
        assert!(!state.add_certificate(party(1), cert.clone()));
        assert_eq!(state.certificate_count(), 1);

        // Add from different party should succeed
        assert!(state.add_certificate(party(2), cert));
        assert_eq!(state.certificate_count(), 2);
    }

    #[test]
    fn test_add_certificate_duplicate_rejected() {
        let ranking = vec![party(1), party(2)];
        let mut state = ViewState::new(2, 0, ranking);

        let cert = create_test_direct_certificate(1);

        assert!(state.add_certificate(party(1), cert.clone()));
        assert!(!state.add_certificate(party(1), cert)); // Duplicate rejected
        assert_eq!(state.certificate_count(), 1);
    }

    #[test]
    fn test_build_truncated_vector_first_has_cert() {
        let ranking = vec![party(1), party(2), party(3), party(4)];
        let mut state = ViewState::new(2, 0, ranking);

        let cert = create_test_direct_certificate(1);
        let cert_hash = cert.hash();
        state.add_certificate(party(1), cert);

        let vector = state.build_truncated_input_vector();

        // First party has cert, so vector is [cert_hash] (length 1)
        assert_eq!(vector.len(), 1);
        assert_eq!(vector[0], cert_hash);
    }

    #[test]
    fn test_build_truncated_vector_middle_has_cert() {
        let ranking = vec![party(1), party(2), party(3), party(4)];
        let mut state = ViewState::new(2, 0, ranking);

        let cert = create_test_direct_certificate(1);
        let cert_hash = cert.hash();
        // party(3) is at position 2 in ranking
        state.add_certificate(party(3), cert);

        let vector = state.build_truncated_input_vector();

        // [⊥, ⊥, cert_hash] (truncated after first non-⊥)
        assert_eq!(vector.len(), 3);
        assert_eq!(vector[0], HashValue::zero());
        assert_eq!(vector[1], HashValue::zero());
        assert_eq!(vector[2], cert_hash);
    }

    #[test]
    fn test_build_truncated_vector_all_empty() {
        let ranking = vec![party(1), party(2), party(3), party(4)];
        let state = ViewState::new(2, 0, ranking);

        let vector = state.build_truncated_input_vector();

        // All ⊥ case: [⊥, ⊥, ⊥, ⊥]
        assert_eq!(vector.len(), 4);
        for hash in &vector {
            assert_eq!(*hash, HashValue::zero());
        }
    }

    #[test]
    fn test_build_truncated_vector_last_has_cert() {
        let ranking = vec![party(1), party(2), party(3), party(4)];
        let mut state = ViewState::new(2, 0, ranking);

        let cert = create_test_direct_certificate(1);
        let cert_hash = cert.hash();
        // party(4) is at position 3 (last) in ranking
        state.add_certificate(party(4), cert);

        let vector = state.build_truncated_input_vector();

        // [⊥, ⊥, ⊥, cert_hash]
        assert_eq!(vector.len(), 4);
        assert_eq!(vector[0], HashValue::zero());
        assert_eq!(vector[1], HashValue::zero());
        assert_eq!(vector[2], HashValue::zero());
        assert_eq!(vector[3], cert_hash);
    }

    #[test]
    fn test_get_first_certificate() {
        let ranking = vec![party(1), party(2), party(3), party(4)];
        let mut state = ViewState::new(2, 0, ranking);

        // No certificates yet
        assert!(state.get_first_certificate().is_none());

        let cert1 = create_test_direct_certificate(1);
        let cert2 = create_test_direct_certificate(1);

        // Add cert for party(3) (position 2)
        state.add_certificate(party(3), cert1.clone());

        // First certificate should be from party(3)
        let first = state.get_first_certificate().unwrap();
        assert_eq!(first.view(), cert1.view());

        // Add cert for party(1) (position 0)
        state.add_certificate(party(1), cert2.clone());

        // Now first certificate should be from party(1)
        let first = state.get_first_certificate().unwrap();
        assert_eq!(first.view(), cert2.view());
    }

    #[test]
    fn test_get_first_certificate_none_when_empty() {
        let ranking = vec![party(1), party(2), party(3), party(4)];
        let state = ViewState::new(2, 0, ranking);

        assert!(state.get_first_certificate().is_none());
        assert!(state.get_first_certificate_position().is_none());
    }

    #[test]
    fn test_get_first_certificate_position() {
        let ranking = vec![party(1), party(2), party(3), party(4)];
        let mut state = ViewState::new(2, 0, ranking);

        let cert = create_test_direct_certificate(1);

        // Add cert for party(3) (position 2)
        state.add_certificate(party(3), cert);

        assert_eq!(state.get_first_certificate_position(), Some(2));
    }

    #[test]
    fn test_view_output() {
        let ranking = vec![party(1), party(2), party(3), party(4)];
        let mut state = ViewState::new(2, 0, ranking);

        assert!(!state.is_complete());
        assert!(state.output().is_none());

        // Create a minimal QC3 for testing
        let qc3 = QC3::new(vec![]);
        let output = ViewOutput::new(
            2,
            0,
            vec![HashValue::random()],
            vec![HashValue::random(), HashValue::random()],
            qc3,
        );

        state.set_output(output.clone());

        assert!(state.is_complete());
        assert!(state.output().is_some());
        assert_eq!(state.output().unwrap().view, 2);
        assert!(state.output().unwrap().has_non_empty_high());
    }

    #[test]
    fn test_view_output_has_non_empty_high() {
        let qc3 = QC3::new(vec![]);

        // Non-empty v_high
        let output1 = ViewOutput::new(2, 0, vec![], vec![HashValue::random()], qc3.clone());
        assert!(output1.has_non_empty_high());

        // Empty v_high
        let output2 = ViewOutput::new(2, 0, vec![], vec![], qc3);
        assert!(!output2.has_non_empty_high());
    }
}
