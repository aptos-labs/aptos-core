// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_transaction_builder::aptos_stdlib;
use aptos_types::transaction::{ExecutionStatus, TransactionStatus};
use language_e2e_tests::{
    gas_costs::TXN_RESERVED, test_with_different_versions, versioning::CURRENT_RELEASE_VERSIONS,
};

#[test]
fn mint_to_new_account() {
    test_with_different_versions! {CURRENT_RELEASE_VERSIONS, |test_env| {
        let mut executor = test_env.executor;

        let root = test_env.dr_account;

        // create and publish a sender with TXN_RESERVED coins
        let new_account = executor.create_raw_account_data(0, 0);
        executor.add_account_data(&new_account);

        let mint_amount = TXN_RESERVED;
        let output = executor.execute_transaction(
            root.transaction()
                .payload(aptos_stdlib::encode_test_coin_mint(
                    *new_account.address(),
                    mint_amount,
                ))
                .sequence_number(0)
                .sign(),
        );

        assert_eq!(
            output.status(),
            &TransactionStatus::Keep(ExecutionStatus::Success),
        );
    }
    }
}
