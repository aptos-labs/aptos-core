// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Cross-slot ranking manager for the Multi-Slot Consensus protocol (Algorithm 4).
//!
//! Implements the `Update(rank^MC, v)` rule: if the committed v_high has length
//! ℓ < n, the party at ranking position ℓ+1 (the first excluded — responsible for
//! cutting v_high short) is moved to the end of the ranking. This is the
//! censorship-resistance mechanism.
//!
//! This is orthogonal to the per-view cyclic rotation in [`RankingManager`](crate::view_state::RankingManager),
//! which rotates within a single SPC instance for liveness.

use aptos_consensus_types::common::Author;

/// Manages the cross-slot ranking `rank^MC` for multi-slot consensus.
///
/// After each slot, the ranking is updated based on whether all validators'
/// proposals were included in v_high. If not, the first excluded validator
/// is demoted to the end of the ranking.
///
/// The current ranking is passed as `initial_ranking` to each slot's SPC instance.
#[derive(Clone, Debug)]
pub struct MultiSlotRankingManager {
    current_ranking: Vec<Author>,
}

impl MultiSlotRankingManager {
    /// Create a new ranking manager with the initial validator ordering.
    pub fn new(initial_ranking: Vec<Author>) -> Self {
        Self { current_ranking: initial_ranking }
    }

    /// Returns the current ranking (passed to SPC as initial_ranking for each slot).
    pub fn current_ranking(&self) -> &[Author] {
        &self.current_ranking
    }

    /// SPC-aware ranking update: demote parties from non-committing SPC views,
    /// then demote the first excluded party from v_high.
    ///
    /// - `committing_view`: the SPC view that produced the commit (from previous slot's proof).
    ///   Views 1..committing_view-1 each had a first-ranked party that failed to produce a
    ///   v_low commit — those parties are demoted in view order.
    /// - `spc_initial_ranking`: the ranking used for the SPC that produced the proof (one slot back).
    /// - `current_slot_ranking`: the ranking used for the current slot's proposal ordering
    ///   (needed to identify the first excluded party by name, not by index).
    /// - `committed_prefix_length`: number of non-⊥ entries in the current slot's v_high.
    pub fn update_with_proof(
        &mut self,
        committing_view: u64,
        spc_initial_ranking: &[Author],
        current_slot_ranking: &[Author],
        committed_prefix_length: usize,
    ) {
        debug_assert!(committing_view >= 1, "committing_view must be >= 1");
        debug_assert!(
            !self.current_ranking.is_empty(),
            "ranking must have at least one validator"
        );
        let n = spc_initial_ranking.len();

        // Step 1: Demote first-ranked party in each non-committing view.
        // Uses spc_initial_ranking (from the proof's slot).
        for view in 1..committing_view {
            let first_ranked_idx = ((view - 1) % (n as u64)) as usize;
            let party = spc_initial_ranking[first_ranked_idx];
            self.demote_party(&party);
        }

        // Step 2: v_high exclusion demotion.
        // Identify excluded party by name from current_slot_ranking, then
        // find and demote them in current_ranking (which may have been
        // modified by SPC-view demotions above).
        if committed_prefix_length < current_slot_ranking.len() {
            let excluded_party = current_slot_ranking[committed_prefix_length];
            self.demote_party(&excluded_party);
        }
    }

    /// Move a party to the end of the ranking.
    /// No-op if the party is already at the end or not found in the ranking.
    fn demote_party(&mut self, party: &Author) {
        if let Some(pos) = self.position_of(party) {
            if pos < self.current_ranking.len() - 1 {
                let demoted = self.current_ranking.remove(pos);
                self.current_ranking.push(demoted);
            }
        }
    }

    /// Look up a validator's position in the current ranking.
    pub fn position_of(&self, author: &Author) -> Option<usize> {
        self.current_ranking.iter().position(|a| a == author)
    }

    /// Override the current ranking (used during catch-up to adopt a BFT-agreed ranking).
    pub fn set_ranking(&mut self, ranking: Vec<Author>) {
        self.current_ranking = ranking;
    }

    /// Number of validators in the ranking.
    pub fn validator_count(&self) -> usize {
        self.current_ranking.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_types::account_address::AccountAddress;

    fn authors(n: usize) -> Vec<Author> {
        (0..n)
            .map(|i| AccountAddress::from_hex_literal(&format!("0x{}", i + 1)).unwrap())
            .collect()
    }

    // ========================================================================
    // v_high exclusion demotion tests (committing_view=1, no SPC-view demotions)
    // ========================================================================

    #[test]
    fn test_ranking_no_change_full_prefix() {
        let v = authors(4);
        let mut mgr = MultiSlotRankingManager::new(v.clone());

        mgr.update_with_proof(1, &v, &v, 4); // ℓ = n, full prefix
        assert_eq!(mgr.current_ranking(), &v);
    }

    #[test]
    fn test_ranking_demotion_first_excluded() {
        // [A, B, C, D], ℓ=1 → party at position 2 (B) demoted → [A, C, D, B]
        let v = authors(4);
        let mut mgr = MultiSlotRankingManager::new(v.clone());

        mgr.update_with_proof(1, &v, &v, 1);
        assert_eq!(mgr.current_ranking(), &[v[0], v[2], v[3], v[1]]);
    }

    #[test]
    fn test_ranking_demotion_last_excluded() {
        // [A, B, C, D], ℓ=3 → party at position 4 (D) demoted → [A, B, C, D] (no visible change)
        let v = authors(4);
        let mut mgr = MultiSlotRankingManager::new(v.clone());

        mgr.update_with_proof(1, &v, &v, 3);
        // D moves to end, but was already at end — ranking unchanged
        assert_eq!(mgr.current_ranking(), &v);
    }

    #[test]
    fn test_ranking_demotion_multiple_slots() {
        // Simulate 3 slots of v_high exclusion demotion
        let v = authors(4); // [A, B, C, D]
        let mut mgr = MultiSlotRankingManager::new(v.clone());

        // Slot 1: ℓ=2 → C demoted → [A, B, D, C]
        let r = mgr.current_ranking().to_vec();
        mgr.update_with_proof(1, &r, &r, 2);
        assert_eq!(mgr.current_ranking(), &[v[0], v[1], v[3], v[2]]);

        // Slot 2: ℓ=1 → B demoted → [A, D, C, B]
        let r = mgr.current_ranking().to_vec();
        mgr.update_with_proof(1, &r, &r, 1);
        assert_eq!(mgr.current_ranking(), &[v[0], v[3], v[2], v[1]]);

        // Slot 3: ℓ=4 = n → no change
        let r = mgr.current_ranking().to_vec();
        mgr.update_with_proof(1, &r, &r, 4);
        assert_eq!(mgr.current_ranking(), &[v[0], v[3], v[2], v[1]]);
    }

    #[test]
    fn test_ranking_demotion_zero_prefix_length() {
        // ℓ=0 (empty v_high) → party at position 1 (A) demoted
        let v = authors(4); // [A, B, C, D]
        let mut mgr = MultiSlotRankingManager::new(v.clone());

        mgr.update_with_proof(1, &v, &v, 0);
        // A (index 0) moves to end → [B, C, D, A]
        assert_eq!(mgr.current_ranking(), &[v[1], v[2], v[3], v[0]]);
    }

    #[test]
    fn test_ranking_single_validator() {
        let v = authors(1); // [A]
        let mut mgr = MultiSlotRankingManager::new(v.clone());

        // ℓ=1 = n → no change
        mgr.update_with_proof(1, &v, &v, 1);
        assert_eq!(mgr.current_ranking(), &v);

        // ℓ=0 → A moves to end → still [A]
        mgr.update_with_proof(1, &v, &v, 0);
        assert_eq!(mgr.current_ranking(), &v);
    }

    #[test]
    fn test_ranking_position_of() {
        let v = authors(4);
        let mut mgr = MultiSlotRankingManager::new(v.clone());

        assert_eq!(mgr.position_of(&v[0]), Some(0));
        assert_eq!(mgr.position_of(&v[3]), Some(3));

        mgr.update_with_proof(1, &v, &v, 2); // C demoted → [A, B, D, C]
        assert_eq!(mgr.position_of(&v[2]), Some(3)); // C now at end
        assert_eq!(mgr.position_of(&v[3]), Some(2)); // D moved up

        let unknown = AccountAddress::from_hex_literal("0xFF").unwrap();
        assert_eq!(mgr.position_of(&unknown), None);
    }

    // ========================================================================
    // New SPC-aware demotion tests
    // ========================================================================

    #[test]
    fn test_update_with_proof_view1_commit() {
        // committing_view=1: no SPC-view demotions, only v_high exclusion
        let v = authors(4); // [A, B, C, D]
        let mut mgr = MultiSlotRankingManager::new(v.clone());

        mgr.update_with_proof(1, &v, &v, 3); // exclude D
        // Only v_high exclusion: D demoted → [A, B, C, D] (D already at end, no visible change)
        assert_eq!(mgr.current_ranking(), &v);

        mgr.update_with_proof(1, &v, mgr.current_ranking().to_vec().as_slice(), 2);
        // Exclude C → [A, B, D, C]
        assert_eq!(mgr.current_ranking(), &[v[0], v[1], v[3], v[2]]);
    }

    #[test]
    fn test_update_with_proof_view3_commit() {
        // committing_view=3: demote first-ranked in views 1 and 2, then v_high exclusion
        let v = authors(4); // [A, B, C, D]
        let spc_ranking = v.clone(); // SPC used this ranking
        let current_ranking = v.clone(); // Current slot used same ranking
        let mut mgr = MultiSlotRankingManager::new(v.clone());

        // Views 1,2 failed: demote spc_ranking[(1-1)%4]=A, spc_ranking[(2-1)%4]=B
        // Then v_high exclusion: current_ranking[2]=C demoted
        mgr.update_with_proof(3, &spc_ranking, &current_ranking, 2);

        // After demoting A: [B, C, D, A]
        // After demoting B: [C, D, A, B]
        // After demoting C (from current_ranking[2]=C): [D, A, B, C]
        assert_eq!(mgr.current_ranking(), &[v[3], v[0], v[1], v[2]]);
    }

    #[test]
    fn test_update_with_proof_wraparound() {
        // committing_view > n: cyclic demotion wraps around
        let v = authors(4); // [A, B, C, D]
        let spc_ranking = v.clone();
        let mut mgr = MultiSlotRankingManager::new(v.clone());

        // committing_view=6: demote views 1-5, full v_high (no exclusion demotion)
        // View 1: A, View 2: B, View 3: C, View 4: D, View 5: A (wraps)
        mgr.update_with_proof(6, &spc_ranking, &v, 4);

        // Start:   [A, B, C, D]
        // Demote A → [B, C, D, A]
        // Demote B → [C, D, A, B]
        // Demote C → [D, A, B, C]
        // Demote D → [A, B, C, D]
        // Demote A → [B, C, D, A]
        assert_eq!(mgr.current_ranking(), &[v[1], v[2], v[3], v[0]]);
    }

    #[test]
    fn test_update_with_proof_repeated_party() {
        // Same party is first-ranked in multiple views; second demotion is a no-op
        let v = authors(4); // [A, B, C, D]
        let spc_ranking = v.clone();
        let mut mgr = MultiSlotRankingManager::new(v.clone());

        // committing_view=5: views 1-4 failed
        // View 1: A, View 2: B, View 3: C, View 4: D
        // All 4 demoted once each, full rotation back to [A, B, C, D]
        // Then no v_high exclusion (full prefix)
        mgr.update_with_proof(5, &spc_ranking, &v, 4);
        assert_eq!(mgr.current_ranking(), &v);
    }

    #[test]
    fn test_update_with_proof_vhigh_exclusion_after_spc_demotions() {
        // Verify that v_high exclusion finds the correct party by name
        // even after SPC-view demotions have reshuffled the ranking.
        let v = authors(4); // [A, B, C, D]
        let spc_ranking = v.clone();
        // Current slot ranking is [A, B, C, D] (same as initial)
        let current_slot_ranking = v.clone();
        let mut mgr = MultiSlotRankingManager::new(v.clone());

        // committing_view=2: demote view 1's first-ranked (A), then exclude current_slot_ranking[1]=B
        mgr.update_with_proof(2, &spc_ranking, &current_slot_ranking, 1);

        // After demoting A: [B, C, D, A]
        // v_high exclusion: current_slot_ranking[1] = B → find B in [B, C, D, A] at pos 0, demote → [C, D, A, B]
        assert_eq!(mgr.current_ranking(), &[v[2], v[3], v[0], v[1]]);
    }

    #[test]
    fn test_set_ranking_overrides_current() {
        let v = authors(4); // [A, B, C, D]
        let mut mgr = MultiSlotRankingManager::new(v.clone());
        assert_eq!(mgr.current_ranking(), &v);

        // Override with reversed ranking
        let reversed: Vec<Author> = v.iter().rev().cloned().collect();
        mgr.set_ranking(reversed.clone());
        assert_eq!(mgr.current_ranking(), &reversed);

        // Further updates work on the adopted ranking
        mgr.update_with_proof(1, &reversed, &reversed, 2); // exclude reversed[2]=B
        assert_eq!(mgr.current_ranking(), &[v[3], v[2], v[0], v[1]]);
    }

    #[test]
    fn test_update_with_proof_full_vhigh_no_exclusion() {
        // When committed_prefix_length == n, no v_high exclusion demotion
        let v = authors(4);
        let spc_ranking = v.clone();
        let mut mgr = MultiSlotRankingManager::new(v.clone());

        // committing_view=2, but full v_high → only SPC demotion for view 1
        mgr.update_with_proof(2, &spc_ranking, &v, 4);

        // Demote A (view 1 first-ranked) → [B, C, D, A]
        // No v_high exclusion (committed_prefix_length == 4 == n)
        assert_eq!(mgr.current_ranking(), &[v[1], v[2], v[3], v[0]]);
    }
}
