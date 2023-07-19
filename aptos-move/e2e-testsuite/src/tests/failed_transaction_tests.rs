// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_gas::{
    AptosGasParameters, StandardGasMeter, StorageGasParameters, LATEST_GAS_FEATURE_VERSION,
};
use aptos_language_e2e_tests::{common_transactions::peer_to_peer_txn, executor::FakeExecutor};
use aptos_memory_usage_tracker::MemoryTrackedGasMeter;
use aptos_state_view::TStateView;
use aptos_types::{
    transaction::ExecutionStatus,
    vm_status::{StatusCode, VMStatus},
};
use aptos_vm::{data_cache::AsMoveResolver, transaction_metadata::TransactionMetadata, AptosVM};
use aptos_vm_logging::log_schema::AdapterLogSchema;
use move_core_types::vm_status::StatusCode::TYPE_MISMATCH;

#[test]
fn failed_transaction_cleanup_test() {
    let mut executor = FakeExecutor::from_head_genesis();
    // TODO(Gas): double check this
    let sender = executor.create_raw_account_data(1_000_000, 10);
    executor.add_account_data(&sender);

    let log_context = AdapterLogSchema::new(executor.get_state_view().id(), 0);
    let aptos_vm = AptosVM::new(executor.get_state_view());
    let data_cache = executor.get_state_view().as_move_resolver();

    let txn_data = TransactionMetadata {
        sender: *sender.address(),
        max_gas_amount: 100_000.into(),
        gas_unit_price: 0.into(),
        sequence_number: 10,
        ..Default::default()
    };

    let gas_params = AptosGasParameters::zeros();
    let storage_gas_params = StorageGasParameters::free_and_unlimited();

    let change_set_configs = storage_gas_params.change_set_configs.clone();

    let mut gas_meter = MemoryTrackedGasMeter::new(StandardGasMeter::new(
        LATEST_GAS_FEATURE_VERSION,
        gas_params,
        storage_gas_params,
        10_000,
    ));

    // TYPE_MISMATCH should be kept and charged.
    let out1 = aptos_vm.failed_transaction_cleanup(
        VMStatus::error(StatusCode::TYPE_MISMATCH, None),
        &mut gas_meter,
        &txn_data,
        &data_cache,
        &log_context,
        &change_set_configs,
    );
    assert!(!out1.write_set().is_empty());
    assert_eq!(out1.gas_used(), 90_000);
    assert!(!out1.status().is_discarded());
    assert_eq!(
        out1.status().status(),
        // StatusCode::TYPE_MISMATCH
        Ok(ExecutionStatus::MiscellaneousError(Some(TYPE_MISMATCH)))
    );

    // Invariant violations should be charged.
    let out2 = aptos_vm.failed_transaction_cleanup(
        VMStatus::error(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR, None),
        &mut gas_meter,
        &txn_data,
        &data_cache,
        &log_context,
        &change_set_configs,
    );
    assert!(out2.gas_used() != 0);
    assert!(!out2.status().is_discarded());
    assert_eq!(
        out2.status().status(),
        Ok(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR
        )))
    );
}

#[test]
fn non_existent_sender() {
    let mut executor = FakeExecutor::from_head_genesis();
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
        0,
    );

    let output = &executor.execute_transaction(txn);
    assert_eq!(
        output.status().status(),
        Err(StatusCode::SENDING_ACCOUNT_DOES_NOT_EXIST),
    );
}
