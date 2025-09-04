// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_language_e2e_tests::{
    executor::FakeExecutor,
    loader::{DependencyGraph, LoaderTransactionGen},
};
use proptest::{collection::vec, prelude::*};

/// Run these transactions and verify the expected output.
pub fn run_and_assert_universe(
    mut universe: DependencyGraph,
    additional_txns: Vec<LoaderTransactionGen>,
) {
    let mut executor = FakeExecutor::from_head_genesis().set_parallel();

    universe.setup(&mut executor);
    universe.caculate_expected_values();
    universe.execute(&mut executor, additional_txns);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]
    #[test]
    fn all_transactions(
        universe in DependencyGraph::strategy(
            // Number of modules
            20,
            // Number of dependency edges
            30..150,
        ),
        additional_txns in vec(any::<LoaderTransactionGen>(), 10..80),
    ) {
        run_and_assert_universe(universe, additional_txns);
    }

    #[test]
    fn smaller_world(
        universe in DependencyGraph::strategy(
            // Number of modules
            5,
            // Number of dependency edges
            10..20,
        ),
        additional_txns in vec(any::<LoaderTransactionGen>(), 10..20),
    ) {
        run_and_assert_universe(universe, additional_txns);
    }

    #[test]
    fn smaller_world_and_test_deps_charging(
        mut universe in DependencyGraph::strategy(
            // Number of modules
            5,
            // Number of dependency edges
            10..20,
        ),
    ) {
        let mut executor = FakeExecutor::from_head_genesis().set_parallel();
        universe.setup(&mut executor);
        universe.caculate_expected_values();
        universe.execute_and_check_deps_sizes(&mut executor);
    }
}
