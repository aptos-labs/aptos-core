// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests that `0x1::account` functions verifying sequence-number-based proof challenges reject
//! execution contexts where the proof-bearing account's sequence number does not advance. This
//! includes orderless transactions (which use a nonce instead of a sequence number) and the
//! inner payload of a multisig transaction (which advances only the outer submitter's sequence
//! number), since a signed proof embedding the sequence number would stay valid and could be
//! replayed.

use crate::MoveHarness;
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::SigningKey;
use aptos_language_e2e_tests::account::Account;
use aptos_move_e2e_test_harness::{assert_abort, assert_success};
use aptos_types::{
    account_config::{AccountResource, CORE_CODE_ADDRESS},
    chain_id::ChainId,
    transaction::{
        EntryFunction, ExecutionStatus, MultisigTransactionPayload, TransactionExecutable,
        TransactionExtraConfig, TransactionPayload, TransactionPayloadInner, TransactionStatus,
    },
};
use move_core_types::{
    account_address::AccountAddress, ident_str, language_storage::ModuleId,
    parser::parse_struct_tag,
};
use serde::{Deserialize, Serialize};

/// `error::invalid_state(ESEQ_NUM_PROOF_REPLAYABLE_CONTEXT)` in `0x1::account`.
const ESEQ_NUM_PROOF_REPLAYABLE_CONTEXT: u64 = 0x3_0000 + 31;
/// `error::invalid_state(ESEQ_NUM_PROOF_REPLAYABLE_CONTEXT)` in `0x1::multisig_account`.
const EMULTISIG_SEQ_NUM_PROOF_REPLAYABLE_CONTEXT: u64 = 0x3_0000 + 26;

#[derive(Serialize, Deserialize)]
struct SignerCapabilityOfferProofChallengeV2 {
    account_address: AccountAddress,
    module_name: String,
    struct_name: String,
    sequence_number: u64,
    source_address: AccountAddress,
    recipient_address: AccountAddress,
}

#[derive(Serialize, Deserialize)]
struct RotationCapabilityOfferProofChallengeV2 {
    account_address: AccountAddress,
    module_name: String,
    struct_name: String,
    chain_id: u8,
    sequence_number: u64,
    source_address: AccountAddress,
    recipient_address: AccountAddress,
}

const ED25519_SCHEME: u8 = 0;

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
            assert_ne!(*code, ESEQ_NUM_PROOF_REPLAYABLE_CONTEXT);
            assert_ne!(*code, EMULTISIG_SEQ_NUM_PROOF_REPLAYABLE_CONTEXT);
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
    assert_abort!(status, ESEQ_NUM_PROOF_REPLAYABLE_CONTEXT);

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
    assert_abort!(status, ESEQ_NUM_PROOF_REPLAYABLE_CONTEXT);

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
    assert_abort!(status, ESEQ_NUM_PROOF_REPLAYABLE_CONTEXT);

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
    assert_abort!(status, ESEQ_NUM_PROOF_REPLAYABLE_CONTEXT);

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
    assert_abort!(status, EMULTISIG_SEQ_NUM_PROOF_REPLAYABLE_CONTEXT);

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
    assert_abort!(status, EMULTISIG_SEQ_NUM_PROOF_REPLAYABLE_CONTEXT);

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
    assert_abort!(status, ESEQ_NUM_PROOF_REPLAYABLE_CONTEXT);

    let status = harness.run_transaction_payload(&alice, payload);
    assert_aborted_with_other_code(&status);
}

/// A proof is replayable inside a multisig payload because the proof-bearing account's sequence
/// number does not advance when the inner payload executes. The guard must reject proof-bearing
/// account operations before signature verification when invoked through a multisig transaction.
#[test]
fn offer_rotation_capability_rejected_in_multisig_payload() {
    let mut harness = MoveHarness::new();
    let alice = harness.new_account_with_key_pair();
    let bob = harness.new_account_with_key_pair();
    let charlie = harness.new_account_at(AccountAddress::from_hex_literal("0x345").unwrap());

    // Migrate Alice's existing account to a multisig account while preserving Alice's auth key.
    // Bob owns the resulting multisig account and submits/executes its transactions.
    let migrate_payload = aptos_stdlib::multisig_account_create_with_existing_account_call(
        vec![*bob.address()],
        1,
        vec![],
        vec![],
    );
    assert_success!(harness.run_transaction_payload(&alice, migrate_payload));

    // Build a valid proof for Alice's current sequence number. Without the multisig-payload
    // guard, this proof would verify and install the capability offer while leaving the same
    // sequence number available for replay.
    let account_resource_tag = parse_struct_tag("0x1::account::Account").unwrap();
    let alice_sequence_number = harness
        .read_resource::<AccountResource>(alice.address(), account_resource_tag.clone())
        .unwrap()
        .sequence_number();
    let proof = RotationCapabilityOfferProofChallengeV2 {
        account_address: CORE_CODE_ADDRESS,
        module_name: String::from("account"),
        struct_name: String::from("RotationCapabilityOfferProofChallengeV2"),
        chain_id: ChainId::test().id(),
        sequence_number: alice_sequence_number,
        source_address: *alice.address(),
        recipient_address: *charlie.address(),
    };
    let proof_signature = alice
        .privkey
        .sign_arbitrary_message(&bcs::to_bytes(&proof).unwrap());
    let inner_entry = EntryFunction::new(
        ModuleId::new(CORE_CODE_ADDRESS, ident_str!("account").to_owned()),
        ident_str!("offer_rotation_capability").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&proof_signature.to_bytes().to_vec()).unwrap(),
            bcs::to_bytes(&ED25519_SCHEME).unwrap(),
            bcs::to_bytes(&alice.pubkey.to_bytes()).unwrap(),
            bcs::to_bytes(charlie.address()).unwrap(),
        ],
    );
    let multisig_payload = MultisigTransactionPayload::EntryFunction(inner_entry);

    // The creator's vote satisfies the single-signature threshold, so the next transaction can
    // be executed immediately.
    let create_txn_payload = aptos_stdlib::multisig_account_create_transaction(
        *alice.address(),
        bcs::to_bytes(&multisig_payload).unwrap(),
    );
    assert_success!(harness.run_transaction_payload(&bob, create_txn_payload));
    let status = harness.run_multisig(&bob, *alice.address(), Some(multisig_payload));
    // The outer multisig transaction succeeds even though the inner payload aborts. The failure
    // is recorded on chain, and the guarded operation must not have had any effect.
    assert_success!(status);

    // The transaction should have been resolved (failed executions still advance the queue).
    assert!(harness
        .read_resource::<AccountResource>(
            alice.address(),
            parse_struct_tag("0x1::account::Account").unwrap()
        )
        .unwrap()
        .rotation_capability_offer()
        .is_none());
}
