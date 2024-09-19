// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_shuffler::deprecated_fairness::{
    conflict_key::ConflictKeyRegistry, FairnessShuffler, FairnessShufflerImpl,
};

struct TestCase {
    shuffler: FairnessShuffler,
    conflict_key_registries: [ConflictKeyRegistry; 3],
    expected_order: Vec<usize>,
}

impl TestCase {
    fn run(self) {
        let Self {
            shuffler,
            conflict_key_registries,
            expected_order,
        } = self;

        let order =
            FairnessShufflerImpl::new(&conflict_key_registries, shuffler.window_sizes()).shuffle();
        assert_eq!(order, expected_order);
    }
}

#[test]
fn test_all_exempt() {
    TestCase {
        shuffler: FairnessShuffler::new_for_test(2, 2, 2),
        conflict_key_registries: [
            ConflictKeyRegistry::all_exempt(9),
            ConflictKeyRegistry::all_exempt(9),
            ConflictKeyRegistry::all_exempt(9),
        ],
        expected_order: (0..9).collect(),
    }
    .run()
}

#[test]
fn test_non_conflict() {
    TestCase {
        shuffler: FairnessShuffler::new_for_test(2, 2, 2),
        conflict_key_registries: [
            ConflictKeyRegistry::non_conflict(9),
            ConflictKeyRegistry::non_conflict(9),
            ConflictKeyRegistry::non_conflict(9),
        ],
        expected_order: (0..9).collect(),
    }
    .run()
}

#[test]
fn test_full_conflict() {
    TestCase {
        shuffler: FairnessShuffler::new_for_test(2, 2, 2),
        conflict_key_registries: [
            ConflictKeyRegistry::full_conflict(9),
            ConflictKeyRegistry::full_conflict(9),
            ConflictKeyRegistry::full_conflict(9),
        ],
        expected_order: (0..9).collect(),
    }
    .run()
}

#[test]
fn test_modules_ignored_by_window_size() {
    TestCase {
        shuffler: FairnessShuffler::new_for_test(2, 0, 2),
        conflict_key_registries: [
            // [A0, A1, A2, ...]
            ConflictKeyRegistry::non_conflict(8),
            // [M0, M0, M0, M0, M1, M1, M2, M2]
            ConflictKeyRegistry::nums_per_key([4, 2, 2]),
            // [M0::E0, M0::E1, M0::E0, M0::E1, M1::E0, M1::E0, M2::E0, M2::E0]
            ConflictKeyRegistry::nums_per_round_per_key([[1, 1, 0], [1, 1, 4]]),
        ],
        // [M0::E0, M0::E1, M1::E0, M0::E0, M0::E1, M1::E0, M2::E0, M2::E0]
        expected_order: vec![0, 1, 4, 2, 3, 5, 6, 7],
    }
    .run()
}

#[test]
fn test_modules_and_entry_funs_ignored_by_window_size() {
    TestCase {
        shuffler: FairnessShuffler::new_for_test(2, 0, 0),
        conflict_key_registries: [
            // [A0, A1, A2, ...]
            ConflictKeyRegistry::non_conflict(8),
            // [M0, M0, M0, M0, M1, M1, M1, M1]
            ConflictKeyRegistry::nums_per_key([4, 4]),
            // [M0::E0, M0::E0, M0::E1, M0::E1, M1::E0, M1::E0, M1::E1, M1::E1]
            ConflictKeyRegistry::nums_per_key([2, 2, 2, 2]),
        ],
        expected_order: (0..8).collect(),
    }
    .run()
}

#[test]
fn test_exempted_modules() {
    // think "full block of p2p txns"
    TestCase {
        shuffler: FairnessShuffler::new_for_test(3, 2, 2),
        conflict_key_registries: [
            // [0:A0, 1:A0, 2:A0, 3:A0, 4:A1, 5:A1, 6:A1, 7:A2, 8:A2, 9:A3]
            ConflictKeyRegistry::nums_per_key([4, 3, 2, 1]),
            ConflictKeyRegistry::all_exempt(10),
            ConflictKeyRegistry::all_exempt(10),
        ],
        // [A0, A1, A2, A3, A0, A1, A2, A0, A1]
        expected_order: vec![0, 4, 7, 9, 1, 5, 8, 2, 3, 6],
    }
    .run()
}

#[test]
fn test_dominating_module() {
    TestCase {
        shuffler: FairnessShuffler::new_for_test(4, 1, 1),
        conflict_key_registries: [
            ConflictKeyRegistry::non_conflict(7),
            // [M0, M0, M0, M1, M2, M3, M4]
            ConflictKeyRegistry::nums_per_key([3, 1, 1, 1, 1]),
            ConflictKeyRegistry::nums_per_key([3, 1, 1, 1, 1]),
        ],
        // [M0, M1, M0, M2, M0, M3, M4]
        expected_order: vec![0, 3, 1, 4, 2, 5, 6],
    }
    .run()
}

#[test]
fn test_dominating_module2() {
    TestCase {
        shuffler: FairnessShuffler::new_for_test(4, 1, 1),
        conflict_key_registries: [
            ConflictKeyRegistry::non_conflict(8),
            // [M0, M0, M0, M1, M2, M3, M4, M0]
            ConflictKeyRegistry::nums_per_round_per_key([[3, 1, 1, 1, 1], [1, 0, 0, 0, 0]]),
            ConflictKeyRegistry::nums_per_round_per_key([[3, 1, 1, 1, 1], [1, 0, 0, 0, 0]]),
        ],
        // [M0, M1, M0, M2, M0, M3, M4, M0]
        expected_order: vec![0, 3, 1, 4, 2, 5, 6, 7],
    }
    .run()
}

#[test]
fn test_multiple_entry_funs() {
    TestCase {
        shuffler: FairnessShuffler::new_for_test(4, 1, 2),
        conflict_key_registries: [
            ConflictKeyRegistry::non_conflict(10),
            // [M0, M0, M0, M0, M1, M1, M1, M1, M2, M2]
            ConflictKeyRegistry::nums_per_key([4, 4, 2]),
            // [M0::E0, M0::E1, M0::E0, M0::E1, M1::E0, M1::E0, M1::E0, M1::E0, M2::E0, M2::E0]
            ConflictKeyRegistry::nums_per_round_per_key([[1, 1, 0, 0], [1, 1, 4, 2]]),
        ],
        // [M0::E0, M1::E0, M0::E1, M2::E0, M0::E0, M1::E0, M0:E1, M2::E0, M1::E0, M1::E0]
        expected_order: vec![0, 4, 1, 8, 2, 5, 3, 9, 6, 7],
    }
    .run()
}
