// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_language_e2e_tests::{
    account_universe::log_balance_strategy, executor::FakeExecutor, loader::DepGraph,
};
use proptest::prelude::*;

/// Run these transactions and verify the expected output.
pub fn run_and_assert_universe(universe: DepGraph) {
    let mut executor = FakeExecutor::from_head_genesis();
    universe.setup(&mut executor);
    universe.execute(&mut executor);
}

proptest! {
    // These tests are pretty slow but quite comprehensive, so run a smaller number of them.
    #![proptest_config(ProptestConfig::with_cases(32))]
    #[test]
    fn all_transactions(
        universe in DepGraph::strategy(
            20,
            30..150,
            log_balance_strategy(1_000_000),
        )
    ) {
        run_and_assert_universe(universe);
    }
}
