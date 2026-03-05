// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::MoveHarness;
use aptos_cached_packages::aptos_stdlib::aptos_account_transfer;
use aptos_crypto::HashValue;
use aptos_types::{
    on_chain_config::FeatureFlag,
    secret_sharing::{Ciphertext, EvalProof},
    state_store::state_key::StateKey,
    transaction::{
        encrypted_payload::EncryptedPayload, ExecutionStatus, TransactionExtraConfig,
        TransactionPayload,
    },
    write_set::WriteOp,
};
use aptos_vm::AptosVM;
use aptos_vm_environment::environment::AptosEnvironment;
use claims::{assert_ok_eq, assert_some};
use move_core_types::vm_status::{StatusCode, VMStatus};
use test_case::test_case;

// Make sure verification and invariant violation errors are kept.
#[test_case(StatusCode::TYPE_MISMATCH)]
#[test_case(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)]
fn failed_transaction_cleanup_charges_gas(status_code: StatusCode) {
    let mut h = MoveHarness::new();
    let sender = h.new_account_with_balance_and_sequence_number(1_000_000, 10);
    let receiver = h.new_account_with_balance_and_sequence_number(1_000_000, 10);

    let max_gas_amount = 100_000;
    let txn = sender
        .transaction()
        .sequence_number(10)
        .max_gas_amount(max_gas_amount)
        .payload(aptos_account_transfer(*receiver.address(), 1))
        .sign();

    let state_view = h.executor.get_state_view();
    let env = AptosEnvironment::new(&state_view);
    let vm = AptosVM::new(&env);

    let balance = 10_000;
    let output = vm
        .test_failed_transaction_cleanup(
            VMStatus::error(status_code, None),
            &txn,
            state_view,
            balance,
        )
        .1;
    let write_set: Vec<(&StateKey, &WriteOp)> = output
        .concrete_write_set_iter()
        .map(|(k, v)| (k, assert_some!(v)))
        .collect();
    assert!(!write_set.is_empty());
    assert_eq!(output.gas_used(), max_gas_amount - balance);
    assert!(!output.status().is_discarded());
    assert_ok_eq!(
        output.status().as_kept_status(),
        ExecutionStatus::MiscellaneousError(Some(status_code))
    );
}

// When an encrypted transaction fails decryption, it should still be kept on chain
// with the sequence number incremented and gas charged.
#[test]
fn failed_encrypted_transaction_increments_sequence_number() {
    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::ENCRYPTED_TRANSACTIONS], vec![]);

    let initial_seq_num = 10;
    // Needs sufficient balance to cover max_gas_amount (2M) * gas_unit_price (100) = 200M octas.
    let sender = h.new_account_with_balance_and_sequence_number(1_000_000_000, initial_seq_num);

    let ciphertext = Ciphertext::random();
    let extra_config = TransactionExtraConfig::V1 {
        multisig_address: None,
        replay_protection_nonce: None,
    };
    let payload_hash = HashValue::random();

    // Sign the transaction in the Encrypted state (the original state before decryption
    // is attempted). The signature is verified against this state.
    let encrypted_payload = EncryptedPayload::Encrypted {
        ciphertext: ciphertext.clone(),
        extra_config: extra_config.clone(),
        payload_hash,
    };
    let payload = TransactionPayload::EncryptedPayload(encrypted_payload);
    let mut txn = h.create_transaction_payload(&sender, payload);

    // Mutate the payload to simulate a failed decryption attempt, as the block executor
    // would do after failing to decrypt the ciphertext.
    let failed_payload = EncryptedPayload::FailedDecryption {
        ciphertext,
        extra_config,
        payload_hash,
        eval_proof: EvalProof::random(),
    };
    *txn.payload_mut() = TransactionPayload::EncryptedPayload(failed_payload);

    let output = h.run_raw(txn);

    // Transaction should be kept, not discarded.
    assert!(
        !output.status().is_discarded(),
        "Expected transaction to be kept, but got: {:?}",
        output.status()
    );

    // Gas should be charged.
    assert!(output.gas_used() > 0);

    // Sequence number should have been incremented.
    let new_seq_num = h.sequence_number(sender.address());
    assert_eq!(
        new_seq_num,
        initial_seq_num + 1,
        "Sequence number should be incremented after failed encrypted transaction"
    );
}
