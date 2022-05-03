// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_transaction_builder::aptos_stdlib;
use aptos_types::{on_chain_config::Version, transaction::TransactionStatus};
use aptos_vm::AptosVM;
use language_e2e_tests::{
    common_transactions::peer_to_peer_txn, test_with_different_versions,
    versioning::CURRENT_RELEASE_VERSIONS,
};

#[test]
fn initial_aptos_version() {
    test_with_different_versions! {CURRENT_RELEASE_VERSIONS, |test_env| {
        let mut executor = test_env.executor;

        let vm = AptosVM::new(executor.get_state_view());

        assert_eq!(
            vm.internals().version().unwrap(),
            Version { major: test_env.version_number }
        );

        let account = test_env.dr_account;
        let txn = account
            .transaction()
            .payload(aptos_stdlib::encode_version_set_version(test_env.version_number + 1))
            .sequence_number(test_env.dr_sequence_number)
            .sign();
        executor.new_block();
        executor.execute_and_apply(txn);

        let new_vm = AptosVM::new(executor.get_state_view());
        assert_eq!(
            new_vm.internals().version().unwrap(),
            Version { major: test_env.version_number + 1 }
        );
    }
    }
}

#[test]
fn drop_txn_after_reconfiguration() {
    test_with_different_versions! {CURRENT_RELEASE_VERSIONS, |test_env| {
        let mut executor = test_env.executor;
        let vm = AptosVM::new(executor.get_state_view());

        assert_eq!(
            vm.internals().version().unwrap(),
            Version { major: test_env.version_number }
        );

        let account = test_env.dr_account;
        let txn = account
            .transaction()
            .payload(aptos_stdlib::encode_version_set_version(test_env.version_number + 1))
            .sequence_number(test_env.dr_sequence_number)
            .sign();
        executor.new_block();

        let sender = executor.create_raw_account_data(1_000_000, 10);
        let receiver = executor.create_raw_account_data(100_000, 10);
        let txn2 = peer_to_peer_txn(sender.account(), receiver.account(), 11, 1000);

        let mut output = executor.execute_block(vec![txn, txn2]).unwrap();
        assert_eq!(output.pop().unwrap().status(), &TransactionStatus::Retry)
    }
    }
}
