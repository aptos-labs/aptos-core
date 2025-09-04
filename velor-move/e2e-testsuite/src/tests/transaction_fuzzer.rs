// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_cached_packages::velor_stdlib::EntryFunctionCall;
use velor_language_e2e_tests::{account::Account, executor::FakeExecutor};
use proptest::{collection::vec, prelude::*};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]
    #[test]
    fn fuzz_scripts_genesis_state(
        txns in vec(any::<EntryFunctionCall>(), 0..10),
    ) {
        let executor = FakeExecutor::from_head_genesis();
        let accounts = vec![
            (Account::new_velor_root(), 0),
        ];
        let num_accounts = accounts.len();

        for (i, txn) in txns.into_iter().enumerate() {
            let payload = txn.encode();
            let (account, account_sequence_number) = &accounts[i % num_accounts];
            let output = executor.execute_transaction(
                account.transaction()
                .payload(payload.clone())
                .sequence_number(*account_sequence_number)
                .sign());
                prop_assert!(!output.status().is_discarded());
        }
    }
}
