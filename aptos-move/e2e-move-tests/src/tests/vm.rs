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
        encrypted_payload::{
            DecryptedPlaintext, DecryptionFailureReason, EncryptedInner, EncryptedPayload,
        },
        ExecutionStatus, TransactionExecutable, TransactionExtraConfig, TransactionPayload,
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

fn build_encrypted_inner() -> EncryptedInner {
    EncryptedInner {
        ciphertext: Ciphertext::random(),
        extra_config: TransactionExtraConfig::V1 {
            multisig_address: None,
            replay_protection_nonce: None,
        },
        payload_hash: HashValue::random(),
        encryption_epoch: 1,
        claimed_entry_fun: None,
    }
}

// When an encrypted transaction fails decryption, it should still be kept on chain
// with the sequence number incremented and gas charged.
#[test]
fn failed_encrypted_transaction_increments_sequence_number() {
    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::ENCRYPTED_TRANSACTIONS], vec![]);
    // Encrypted txns require a higher gas unit price.
    h.default_gas_unit_price = 200;

    let initial_seq_num = 10;
    // Needs sufficient balance to cover max_gas_amount (20M) * gas_unit_price (200) = 4B octas.
    let sender = h.new_account_with_balance_and_sequence_number(10_000_000_000, initial_seq_num);

    let original = build_encrypted_inner();

    // Sign the transaction in the Encrypted state (the original state before decryption
    // is attempted). The signature is verified against this state.
    let encrypted_payload = EncryptedPayload::Encrypted(original.clone());
    let payload = TransactionPayload::EncryptedPayload(encrypted_payload);
    let mut txn = h.create_transaction_payload(&sender, payload);

    // Mutate the payload to simulate a failed decryption attempt, as the block executor
    // would do after failing to decrypt the ciphertext.
    let failed_payload = EncryptedPayload::FailedDecryption {
        original,
        eval_proof: Some(EvalProof::random()),
        reason: DecryptionFailureReason::CryptoFailure,
    };
    *txn.payload_mut() = TransactionPayload::EncryptedPayload(failed_payload);

    let output = h.run_raw(txn);

    // Transaction should be kept, not discarded.
    assert!(
        !output.status().is_discarded(),
        "Expected transaction to be kept, but got: {:?}",
        output.status()
    );

    // Gas should include at least the decryption surcharge (375 external gas units) even
    // though execution failed. The surcharge is charged in the prologue before the failure.
    let surcharge_external_gas = 375u64;
    assert!(
        output.gas_used() >= surcharge_external_gas,
        "Gas used ({}) should include the decryption surcharge ({})",
        output.gas_used(),
        surcharge_external_gas,
    );

    // Sequence number should have been incremented.
    let new_seq_num = h.sequence_number(sender.address());
    assert_eq!(
        new_seq_num,
        initial_seq_num + 1,
        "Sequence number should be incremented after failed encrypted transaction"
    );
}

// Encrypted transactions with gas_unit_price below encrypted_txn_min_price_per_gas_unit
// should be rejected with ENCRYPTED_TXN_GAS_UNIT_PRICE_BELOW_MIN_BOUND.
#[test]
fn encrypted_transaction_rejected_below_min_gas_price() {
    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::ENCRYPTED_TRANSACTIONS], vec![]);
    // Use gas_unit_price below the encrypted minimum (200).
    h.default_gas_unit_price = 100;

    let sender = h.new_account_with_balance_and_sequence_number(10_000_000_000, 10);

    let original = build_encrypted_inner();

    let encrypted_payload = EncryptedPayload::Encrypted(original.clone());
    let payload = TransactionPayload::EncryptedPayload(encrypted_payload);
    let mut txn = h.create_transaction_payload(&sender, payload);

    // Simulate failed decryption.
    let failed_payload = EncryptedPayload::FailedDecryption {
        original,
        eval_proof: Some(EvalProof::random()),
        reason: DecryptionFailureReason::CryptoFailure,
    };
    *txn.payload_mut() = TransactionPayload::EncryptedPayload(failed_payload);

    let output = h.run_raw(txn);

    // Transaction should be discarded with the specific error for encrypted gas price below min.
    use aptos_types::transaction::TransactionStatus;
    match output.status() {
        TransactionStatus::Discard(StatusCode::ENCRYPTED_TXN_GAS_UNIT_PRICE_BELOW_MIN_BOUND) => {},
        other => panic!(
            "Expected ENCRYPTED_TXN_GAS_UNIT_PRICE_BELOW_MIN_BOUND, got: {:?}",
            other
        ),
    }
}

// Encrypted transactions should use more gas than equivalent non-encrypted transactions
// due to the decryption surcharge.
#[test]
fn encrypted_transaction_charges_decryption_surcharge() {
    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::ENCRYPTED_TRANSACTIONS], vec![]);
    h.default_gas_unit_price = 200;

    let sender = h.new_account_with_balance_and_sequence_number(10_000_000_000, 10);
    let receiver = h.new_account_with_balance_and_sequence_number(1_000_000, 10);

    // Run a normal (non-encrypted) transfer first.
    let normal_payload = aptos_account_transfer(*receiver.address(), 1);
    let normal_txn = h.create_transaction_payload(&sender, normal_payload);
    let normal_output = h.run_raw(normal_txn);
    assert!(
        !normal_output.status().is_discarded(),
        "Normal transfer should succeed"
    );
    let normal_gas = normal_output.gas_used();

    // Run an encrypted transfer (Decrypted state, simulating successful decryption).
    let entry_fn =
        aptos_cached_packages::aptos_stdlib::aptos_account_transfer(*receiver.address(), 1);
    let entry_fn_executable = match entry_fn {
        TransactionPayload::EntryFunction(ef) => TransactionExecutable::EntryFunction(ef),
        _ => panic!("Expected EntryFunction payload"),
    };

    let original = build_encrypted_inner();

    // Sign in Encrypted state first.
    let encrypted_payload = EncryptedPayload::Encrypted(original.clone());
    let payload = TransactionPayload::EncryptedPayload(encrypted_payload);
    let mut txn = h.create_transaction_payload(&sender, payload);

    // Mutate to Decrypted state (simulating successful decryption by block executor).
    let decrypted_payload = EncryptedPayload::Decrypted {
        original,
        eval_proof: EvalProof::random(),
        decrypted: DecryptedPlaintext::new(entry_fn_executable, [0u8; 16]),
    };
    *txn.payload_mut() = TransactionPayload::EncryptedPayload(decrypted_payload);

    let encrypted_output = h.run_raw(txn);
    assert!(
        !encrypted_output.status().is_discarded(),
        "Encrypted transfer should not be discarded, got: {:?}",
        encrypted_output.status()
    );
    let encrypted_gas = encrypted_output.gas_used();

    // The encrypted transaction should use at least 375 more gas units (the decryption surcharge)
    // than the normal transaction.
    let surcharge_external_gas = 375u64; // 375_000_000 internal gas / 1_000_000 scaling factor
    assert!(
        encrypted_gas >= normal_gas + surcharge_external_gas,
        "Encrypted txn gas ({}) should be at least {} more than normal txn gas ({})",
        encrypted_gas,
        surcharge_external_gas,
        normal_gas
    );
}
