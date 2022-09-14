// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, MoveHarness};
use aptos_crypto::SigningKey;
use aptos_types::account_config::AccountResource;
use aptos_types::{account_address::AccountAddress, account_config::CORE_CODE_ADDRESS};
use cached_packages::aptos_stdlib;
use move_deps::move_core_types::parser::parse_struct_tag;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct RotationCapabilityOfferProofChallengeV2 {
    account_address: AccountAddress,
    module_name: String,
    struct_name: String,
    sequence_number: u64,
    source_address: AccountAddress,
    recipient_address: AccountAddress,
}

#[test]
fn offer_rotation_capability_v2() {
    let mut harness = MoveHarness::new();

    let account1 = harness.new_account_with_key_pair();
    let account2 = harness.new_account_with_key_pair();

    let rotation_capability_proof = RotationCapabilityOfferProofChallengeV2 {
        account_address: CORE_CODE_ADDRESS,
        module_name: String::from("account"),
        struct_name: String::from("RotationCapabilityOfferProofChallengeV2"),
        sequence_number: 0,
        source_address: *account1.address(),
        recipient_address: *account2.address(),
    };

    let rotation_capability_proof_msg = bcs::to_bytes(&rotation_capability_proof);
    let rotation_proof_signed = account1
        .privkey
        .sign_arbitrary_message(&rotation_capability_proof_msg.unwrap());

    assert_success!(harness.run_transaction_payload(
        &account1,
        aptos_stdlib::account_offer_rotation_capability(
            rotation_proof_signed.to_bytes().to_vec(),
            0,
            account1.pubkey.to_bytes().to_vec(),
            *account2.address(),
        )
    ));

    let account_resource = parse_struct_tag("0x1::account::Account").unwrap();
    assert_eq!(
        harness
            .read_resource::<AccountResource>(account1.address(), account_resource)
            .unwrap()
            .rotation_capability_offer()
            .unwrap(),
        *account2.address()
    );
}

/// Samples a test case for the Move unit tests for `offer_rotation_capability`
/// in aptos-move/framework/aptos-framework/sources/account.move
#[test]
fn sample_offer_rotation_capability_v2_test_case_for_move() {
    let mut harness = MoveHarness::new();

    let account_alice = harness.new_account_with_key_pair();
    let account_bob = harness.new_account_at(AccountAddress::from_hex_literal("0x345").unwrap());

    // This struct fixes sequence number 0, which is what Alice's account is at in the Move test
    let proof_struct = RotationCapabilityOfferProofChallengeV2 {
        account_address: CORE_CODE_ADDRESS,
        module_name: String::from("account"),
        struct_name: String::from("RotationCapabilityOfferProofChallengeV2"),
        sequence_number: 0,
        source_address: *account_alice.address(),
        recipient_address: *account_bob.address(),
    };

    let proof_struct_bytes = bcs::to_bytes(&proof_struct);
    let signature = account_alice
        .privkey
        .sign_arbitrary_message(&proof_struct_bytes.unwrap());

    println!(
        "Alice's PK: {}",
        hex::encode(account_alice.pubkey.to_bytes().as_slice())
    );
    println!("Alice's address: {}", hex::encode(account_alice.address()));
    println!(
        "RotationCapabilityOfferProofChallengeV2 signature: {}",
        hex::encode(signature.to_bytes().as_slice())
    );
}
