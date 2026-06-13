// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests that `0x1::account` functions verifying sequence-number-based proof challenges reject
//! orderless transactions. Orderless transactions do not advance the sender's sequence number,
//! so a signed proof embedding the sequence number would stay valid and could be replayed.

use crate::MoveHarness;
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::SigningKey;
use aptos_language_e2e_tests::account::Account;
use aptos_move_e2e_test_harness::{assert_abort, assert_success};
use aptos_types::{
    account_config::{AccountResource, CORE_CODE_ADDRESS},
    transaction::{
        ExecutionStatus, TransactionExecutable, TransactionExtraConfig, TransactionPayload,
        TransactionPayloadInner, TransactionStatus,
    },
};
use move_core_types::{account_address::AccountAddress, parser::parse_struct_tag};
use serde::{Deserialize, Serialize};

/// `error::invalid_state(ESEQ_NUM_PROOF_IN_ORDERLESS_TXN)` in `0x1::account`.
const ESEQ_NUM_PROOF_IN_ORDERLESS_TXN: u64 = 0x3_0000 + 30;
/// `error::invalid_state(ESEQ_NUM_PROOF_IN_ORDERLESS_TXN)` in `0x1::multisig_account`.
const EMULTISIG_SEQ_NUM_PROOF_IN_ORDERLESS_TXN: u64 = 0x3_0000 + 26;

#[derive(Serialize, Deserialize)]
struct SignerCapabilityOfferProofChallengeV2 {
    account_address: AccountAddress,
    module_name: String,
    struct_name: String,
    sequence_number: u64,
    source_address: AccountAddress,
    recipient_address: AccountAddress,
}

/// Re-submits the entry function from `payload` as an orderless transaction (replay-protected
/// by `nonce` instead of the account's sequence number) and runs it.
fn run_as_orderless_txn(
    harness: &mut MoveHarness,
    account: &Account,
    payload: TransactionPayload,
    nonce: u64,
) -> TransactionStatus {
    let executable = match payload {
        TransactionPayload::EntryFunction(entry_fn) => {
            TransactionExecutable::EntryFunction(entry_fn)
        },
        _ => panic!("expected an entry function payload"),
    };
    let payload = TransactionPayload::Payload(TransactionPayloadInner::V1 {
        executable,
        extra_config: TransactionExtraConfig::V1 {
            multisig_address: None,
            replay_protection_nonce: Some(nonce),
        },
    });
    // Orderless transactions must expire within a short window
    // (MAX_EXP_TIME_SECONDS_FOR_ORDERLESS_TXNS in transaction_validation.move), so override
    // the harness's default one-hour TTL.
    let expiration_secs = harness.executor.get_block_time_seconds() + 60;
    let txn = harness
        .create_transaction_without_sign(account, payload)
        .ttl(expiration_secs)
        .sign();
    harness.run(txn)
}

/// Asserts the transaction aborted, but not with either orderless guard code — used to show the
/// guards only fire for orderless transactions.
fn assert_aborted_with_other_code(status: &TransactionStatus) {
    match status {
        TransactionStatus::Keep(ExecutionStatus::MoveAbort { code, .. }) => {
            assert_ne!(*code, ESEQ_NUM_PROOF_IN_ORDERLESS_TXN);
            assert_ne!(*code, EMULTISIG_SEQ_NUM_PROOF_IN_ORDERLESS_TXN);
        },
        _ => panic!(
            "expected the transaction to abort past the orderless guard, got {:?}",
            status
        ),
    }
}

/// A valid signer capability offer proof is rejected when submitted in an orderless
/// transaction, and the very same proof succeeds in a sequence-number transaction.
#[test]
fn offer_signer_capability_rejected_in_orderless_txn() {
    let mut harness = MoveHarness::new();
    let alice = harness.new_account_with_key_pair();
    let bob = harness.new_account_at(AccountAddress::from_hex_literal("0x345").unwrap());

    let proof_struct = SignerCapabilityOfferProofChallengeV2 {
        account_address: CORE_CODE_ADDRESS,
        module_name: String::from("account"),
        struct_name: String::from("SignerCapabilityOfferProofChallengeV2"),
        sequence_number: 0,
        source_address: *alice.address(),
        recipient_address: *bob.address(),
    };
    let signature = alice
        .privkey
        .sign_arbitrary_message(&bcs::to_bytes(&proof_struct).unwrap());
    let payload = aptos_stdlib::account_offer_signer_capability(
        signature.to_bytes().to_vec(),
        0,
        alice.pubkey.to_bytes(),
        *bob.address(),
    );

    // The orderless transaction must be rejected by the guard, before signature verification.
    let status = run_as_orderless_txn(&mut harness, &alice, payload.clone(), 1234);
    assert_abort!(status, ESEQ_NUM_PROOF_IN_ORDERLESS_TXN);

    // No offer must have been stored.
    let account_resource_tag = parse_struct_tag("0x1::account::Account").unwrap();
    assert!(harness
        .read_resource::<AccountResource>(alice.address(), account_resource_tag.clone())
        .unwrap()
        .signer_capability_offer()
        .is_none());

    // The exact same proof is accepted in an ordinary sequence-number transaction (the
    // orderless attempt did not consume Alice's sequence number, which is still 0).
    assert_success!(harness.run_transaction_payload(&alice, payload));
    assert_eq!(
        harness
            .read_resource::<AccountResource>(alice.address(), account_resource_tag)
            .unwrap()
            .signer_capability_offer()
            .unwrap(),
        *bob.address()
    );
}

#[test]
fn offer_rotation_capability_rejected_in_orderless_txn() {
    let mut harness = MoveHarness::new();
    let alice = harness.new_account_with_key_pair();
    let bob = harness.new_account_with_key_pair();

    // The guard fires before any proof deserialization or signature verification, so dummy
    // proof bytes are sufficient to show the orderless rejection.
    let payload = aptos_stdlib::account_offer_rotation_capability(
        vec![0u8; 64],
        0,
        alice.pubkey.to_bytes(),
        *bob.address(),
    );

    let status = run_as_orderless_txn(&mut harness, &alice, payload.clone(), 1234);
    assert_abort!(status, ESEQ_NUM_PROOF_IN_ORDERLESS_TXN);

    // In a sequence-number transaction the same call gets past the guard and fails later
    // (invalid signature), proving the guard is specific to orderless transactions.
    let status = harness.run_transaction_payload(&alice, payload);
    assert_aborted_with_other_code(&status);
}

#[test]
fn rotate_authentication_key_rejected_in_orderless_txn() {
    let mut harness = MoveHarness::new();
    let alice = harness.new_account_with_key_pair();

    let payload = aptos_stdlib::account_rotate_authentication_key(
        0, // ED25519_SCHEME
        alice.pubkey.to_bytes(),
        0, // ED25519_SCHEME
        alice.pubkey.to_bytes(),
        vec![0u8; 64],
        vec![0u8; 64],
    );

    let status = run_as_orderless_txn(&mut harness, &alice, payload.clone(), 1234);
    assert_abort!(status, ESEQ_NUM_PROOF_IN_ORDERLESS_TXN);

    let status = harness.run_transaction_payload(&alice, payload);
    assert_aborted_with_other_code(&status);
}

#[test]
fn rotate_authentication_key_with_rotation_capability_rejected_in_orderless_txn() {
    let mut harness = MoveHarness::new();
    let alice = harness.new_account_with_key_pair();
    let bob = harness.new_account_with_key_pair();

    let payload = aptos_stdlib::account_rotate_authentication_key_with_rotation_capability(
        *alice.address(),
        0, // ED25519_SCHEME
        bob.pubkey.to_bytes(),
        vec![0u8; 64],
    );

    let status = run_as_orderless_txn(&mut harness, &bob, payload.clone(), 1234);
    assert_abort!(status, ESEQ_NUM_PROOF_IN_ORDERLESS_TXN);

    let status = harness.run_transaction_payload(&bob, payload);
    assert_aborted_with_other_code(&status);
}

/// `multisig_account::create_with_existing_account` is callable by anyone holding a signed
/// creation message for the target account, so a captured message must not be executable from
/// an orderless transaction.
#[test]
fn multisig_create_with_existing_account_rejected_in_orderless_txn() {
    let mut harness = MoveHarness::new();
    let alice = harness.new_account_with_key_pair();
    let attacker = harness.new_account_with_key_pair();

    let payload = aptos_stdlib::multisig_account_create_with_existing_account(
        *alice.address(),
        vec![*attacker.address()],
        1,
        0, // ED25519_SCHEME
        alice.pubkey.to_bytes(),
        vec![0u8; 64],
        vec![],
        vec![],
    );

    let status = run_as_orderless_txn(&mut harness, &attacker, payload.clone(), 1234);
    assert_abort!(status, EMULTISIG_SEQ_NUM_PROOF_IN_ORDERLESS_TXN);

    let status = harness.run_transaction_payload(&attacker, payload);
    assert_aborted_with_other_code(&status);
}

#[test]
fn multisig_create_with_existing_account_and_revoke_auth_key_rejected_in_orderless_txn() {
    let mut harness = MoveHarness::new();
    let alice = harness.new_account_with_key_pair();
    let attacker = harness.new_account_with_key_pair();

    let payload = aptos_stdlib::multisig_account_create_with_existing_account_and_revoke_auth_key(
        *alice.address(),
        vec![*attacker.address()],
        1,
        0, // ED25519_SCHEME
        alice.pubkey.to_bytes(),
        vec![0u8; 64],
        vec![],
        vec![],
    );

    let status = run_as_orderless_txn(&mut harness, &attacker, payload.clone(), 1234);
    assert_abort!(status, EMULTISIG_SEQ_NUM_PROOF_IN_ORDERLESS_TXN);

    let status = harness.run_transaction_payload(&attacker, payload);
    assert_aborted_with_other_code(&status);
}

#[test]
fn upsert_ed25519_backup_key_rejected_in_orderless_txn() {
    let mut harness = MoveHarness::new();
    let alice = harness.new_account_with_key_pair();

    let payload = aptos_stdlib::account_upsert_ed25519_backup_key_on_keyless_account(
        vec![0u8; 32],
        alice.pubkey.to_bytes(),
        vec![0u8; 64],
    );

    let status = run_as_orderless_txn(&mut harness, &alice, payload.clone(), 1234);
    assert_abort!(status, ESEQ_NUM_PROOF_IN_ORDERLESS_TXN);

    let status = harness.run_transaction_payload(&alice, payload);
    assert_aborted_with_other_code(&status);
}
