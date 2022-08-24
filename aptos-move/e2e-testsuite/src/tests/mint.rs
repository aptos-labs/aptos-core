// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::{ExecutionStatus, TransactionStatus};
use cached_packages::aptos_stdlib;
use language_e2e_tests::{
    coin_supply, gas_costs::TXN_RESERVED, test_with_different_versions,
    versioning::CURRENT_RELEASE_VERSIONS,
};

#[test]
fn mint_to_new_account() {
    test_with_different_versions! {CURRENT_RELEASE_VERSIONS, |test_env| {
        let mut executor = test_env.executor;

        let root = test_env.dr_account;

        // Create and publish a sender with TXN_RESERVED coins, also note how
        // many were there before.
        let new_account = executor.create_raw_account_data(0, 0);
        executor.add_account_data(&new_account);
        let supply_before = coin_supply::fetch_coin_supply(executor.get_state_view()).unwrap();

        let mint_amount = TXN_RESERVED;
        let txn = root.transaction().payload(aptos_stdlib::aptos_coin_mint(*new_account.address(), mint_amount)).sequence_number(0).sign();
        let output = executor.execute_transaction(txn);

        // Check that supply changed.
        executor.apply_write_set(output.write_set());
        let supply_after = coin_supply::fetch_coin_supply(executor.get_state_view()).unwrap();
        assert_eq!(supply_after, supply_before + (mint_amount as u128));

        assert_eq!(
            output.status(),
            &TransactionStatus::Keep(ExecutionStatus::Success),
        );
    }
    }
}
