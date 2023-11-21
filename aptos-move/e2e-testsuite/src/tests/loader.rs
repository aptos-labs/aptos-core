// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_language_e2e_tests::{
    account_universe::log_balance_strategy, executor::FakeExecutor, loader::DependencyGraph,
};
use proptest::prelude::*;

/// Run these transactions and verify the expected output.
pub fn run_and_assert_universe(mut universe: DependencyGraph) {
    let mut executor = FakeExecutor::from_head_genesis();
    universe.setup(&mut executor);
    universe.caculate_expected_values();
    universe.execute(&mut executor);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]
    #[test]
    fn all_transactions(
        universe in DependencyGraph::strategy(
            // Number of modules
            20,
            // Number of dependency edges
            30..150,
            log_balance_strategy(1_000_000),
        )
    ) {
        run_and_assert_universe(universe);
    }
}
