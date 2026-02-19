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

    /// Apply the demotion rule from Algorithm 4: `Update(rank^MC, v)`.
    ///
    /// `committed_prefix_length` is ℓ = |v_high|, the length of the committed high output vector.
    ///
    /// - If `ℓ = n`: ranking unchanged (all proposals included, no censorship)
    /// - If `ℓ < n`: the party at ranking position ℓ+1 (the first excluded — responsible
    ///   for cutting v_high) is moved to the end of the ranking
    pub fn update(&mut self, committed_prefix_length: usize) {
        let n = self.current_ranking.len();
        debug_assert!(
            committed_prefix_length <= n,
            "committed_prefix_length ({}) exceeds validator count ({})",
            committed_prefix_length,
            n,
        );
        if committed_prefix_length < n {
            let demoted = self.current_ranking.remove(committed_prefix_length);
            self.current_ranking.push(demoted);
        }
    }

    /// Look up a validator's position in the current ranking.
    pub fn position_of(&self, author: &Author) -> Option<usize> {
        self.current_ranking.iter().position(|a| a == author)
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

    #[test]
    fn test_ranking_no_change_full_prefix() {
        let validators = authors(4);
        let mut mgr = MultiSlotRankingManager::new(validators.clone());

        mgr.update(4); // ℓ = n, full prefix
        assert_eq!(mgr.current_ranking(), &validators);
    }

    #[test]
    fn test_ranking_demotion_first_excluded() {
        // [A, B, C, D], ℓ=1 → party at position 2 (B) demoted → [A, C, D, B]
        let v = authors(4);
        let mut mgr = MultiSlotRankingManager::new(v.clone());

        mgr.update(1);
        assert_eq!(mgr.current_ranking(), &[v[0], v[2], v[3], v[1]]);
    }

    #[test]
    fn test_ranking_demotion_last_excluded() {
        // [A, B, C, D], ℓ=3 → party at position 4 (D) demoted → [A, B, C, D] (no visible change)
        let v = authors(4);
        let mut mgr = MultiSlotRankingManager::new(v.clone());

        mgr.update(3);
        // D moves to end, but was already at end — ranking unchanged
        assert_eq!(mgr.current_ranking(), &v);
    }

    #[test]
    fn test_ranking_demotion_multiple_slots() {
        // Simulate 3 slots of partial exclusion
        let v = authors(4); // [A, B, C, D]
        let mut mgr = MultiSlotRankingManager::new(v.clone());

        // Slot 1: ℓ=2 → party at position 3 (C) demoted → [A, B, D, C]
        mgr.update(2);
        assert_eq!(mgr.current_ranking(), &[v[0], v[1], v[3], v[2]]);

        // Slot 2: ℓ=1 → party at position 2 (B) demoted → [A, D, C, B]
        mgr.update(1);
        assert_eq!(mgr.current_ranking(), &[v[0], v[3], v[2], v[1]]);

        // Slot 3: ℓ=4 = n → no change
        mgr.update(4);
        assert_eq!(mgr.current_ranking(), &[v[0], v[3], v[2], v[1]]);
    }

    #[test]
    fn test_ranking_demotion_zero_prefix_length() {
        // ℓ=0 (empty v_high) → party at position 1 (A) demoted
        let v = authors(4); // [A, B, C, D]
        let mut mgr = MultiSlotRankingManager::new(v.clone());

        mgr.update(0);
        // A (index 0) moves to end → [B, C, D, A]
        assert_eq!(mgr.current_ranking(), &[v[1], v[2], v[3], v[0]]);
    }

    #[test]
    fn test_ranking_single_validator() {
        let v = authors(1); // [A]
        let mut mgr = MultiSlotRankingManager::new(v.clone());

        // ℓ=1 = n → no change
        mgr.update(1);
        assert_eq!(mgr.current_ranking(), &v);

        // ℓ=0 → A moves to end → still [A]
        mgr.update(0);
        assert_eq!(mgr.current_ranking(), &v);
    }

    #[test]
    fn test_ranking_position_of() {
        let v = authors(4);
        let mut mgr = MultiSlotRankingManager::new(v.clone());

        assert_eq!(mgr.position_of(&v[0]), Some(0));
        assert_eq!(mgr.position_of(&v[3]), Some(3));

        mgr.update(2); // C demoted → [A, B, D, C]
        assert_eq!(mgr.position_of(&v[2]), Some(3)); // C now at end
        assert_eq!(mgr.position_of(&v[3]), Some(2)); // D moved up

        let unknown = AccountAddress::from_hex_literal("0xFF").unwrap();
        assert_eq!(mgr.position_of(&unknown), None);
    }
}
