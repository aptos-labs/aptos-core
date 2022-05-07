// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_state_view::StateView;
use aptos_types::{
    transaction::ExecutionStatus,
    vm_status::{StatusCode, VMStatus},
};
use aptos_vm::{
    data_cache::{IntoMoveResolver, StateViewCache},
    logging::AdapterLogSchema,
    transaction_metadata::TransactionMetadata,
    AptosVM,
};
use language_e2e_tests::{
    common_transactions::peer_to_peer_txn, test_with_different_versions,
    versioning::CURRENT_RELEASE_VERSIONS,
};
use move_deps::{
    move_binary_format::file_format::NUMBER_OF_NATIVE_FUNCTIONS,
    move_core_types::{
        gas_schedule::{GasAlgebra, GasPrice, GasUnits},
        vm_status::StatusCode::TYPE_MISMATCH,
    },
    move_vm_types::gas_schedule::{zero_cost_schedule, GasStatus},
};

#[test]
fn failed_transaction_cleanup_test() {
    test_with_different_versions! {CURRENT_RELEASE_VERSIONS, |test_env| {
        let mut executor = test_env.executor;
        let sender = executor.create_raw_account_data(1_000_000, 10);
        executor.add_account_data(&sender);

        let log_context = AdapterLogSchema::new(executor.get_state_view().id(), 0);
        let aptos_vm = AptosVM::new(executor.get_state_view());
        let data_cache = StateViewCache::new(executor.get_state_view()).into_move_resolver();

        let txn_data = TransactionMetadata {
            sender: *sender.address(),
            max_gas_amount: GasUnits::new(100_000),
            gas_unit_price: GasPrice::new(0),
            sequence_number: 10,
            ..Default::default()
        };

        let gas_schedule = zero_cost_schedule(NUMBER_OF_NATIVE_FUNCTIONS);
        let mut gas_status = GasStatus::new(&gas_schedule, GasUnits::new(10_000));

        // TYPE_MISMATCH should be kept and charged.
        let out1 = aptos_vm.failed_transaction_cleanup(
            VMStatus::Error(StatusCode::TYPE_MISMATCH),
            &mut gas_status,
            &txn_data,
            &data_cache,
            &log_context,
        );
        assert!(!out1.write_set().is_empty());
        assert_eq!(out1.gas_used(), 90_000);
        assert!(!out1.status().is_discarded());
        assert_eq!(
            out1.status().status(),
            // StatusCode::TYPE_MISMATCH
            Ok(ExecutionStatus::MiscellaneousError(Some(TYPE_MISMATCH)))
        );

        // Invariant violations should be discarded and not charged.
        let out2 = aptos_vm.failed_transaction_cleanup(
            VMStatus::Error(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR),
            &mut gas_status,
            &txn_data,
            &data_cache,
            &log_context,
        );
        assert!(out2.write_set().is_empty());
        assert!(out2.gas_used() == 0);
        assert!(out2.status().is_discarded());
        assert_eq!(
            out2.status().status(),
            Err(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
        );
    }
    }
}

#[test]
fn non_existent_sender() {
    test_with_different_versions! {CURRENT_RELEASE_VERSIONS, |test_env| {
        let mut executor = test_env.executor;
        let sequence_number = 0;
        let sender = executor.create_raw_account();
        let receiver = executor.create_raw_account_data(100_000, sequence_number);
        executor.add_account_data(&receiver);

        let transfer_amount = 10;
        let txn = peer_to_peer_txn(
            &sender,
            receiver.account(),
            sequence_number,
            transfer_amount,
        );

        let output = &executor.execute_transaction(txn);
        assert_eq!(
            output.status().status(),
            Err(StatusCode::SENDING_ACCOUNT_DOES_NOT_EXIST),
        );
    }
    }
}
