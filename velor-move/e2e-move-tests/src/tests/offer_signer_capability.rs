// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, MoveHarness};
use velor_cached_packages::velor_stdlib;
use velor_crypto::SigningKey;
use velor_types::{
    account_address::AccountAddress,
    account_config::{AccountResource, CORE_CODE_ADDRESS},
};
use move_core_types::parser::parse_struct_tag;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SignerCapabilityOfferProofChallengeV2 {
    account_address: AccountAddress,
    module_name: String,
    struct_name: String,
    sequence_number: u64,
    source_address: AccountAddress,
    recipient_address: AccountAddress,
}

#[test]
/// Tests Alice offering Bob a signer for her account.
fn offer_signer_capability_v2() {
    let mut harness = MoveHarness::new();

    let account_alice = harness.new_account_with_key_pair();
    let account_bob = harness.new_account_at(AccountAddress::from_hex_literal("0x345").unwrap());

    // This struct fixes sequence number 0, which is what Alice's account is at in this e2e test
    let proof_struct = SignerCapabilityOfferProofChallengeV2 {
        account_address: CORE_CODE_ADDRESS,
        module_name: String::from("account"),
        struct_name: String::from("SignerCapabilityOfferProofChallengeV2"),
        sequence_number: 0,
        source_address: *account_alice.address(),
        recipient_address: *account_bob.address(),
    };

    let proof_struct_bytes = bcs::to_bytes(&proof_struct);
    let signature = account_alice
        .privkey
        .sign_arbitrary_message(&proof_struct_bytes.unwrap());

    assert_success!(harness.run_transaction_payload(
        &account_alice,
        velor_stdlib::account_offer_signer_capability(
            signature.to_bytes().to_vec(),
            0,
            account_alice.pubkey.to_bytes(),
            *account_bob.address(),
        )
    ));

    let account_resource = parse_struct_tag("0x1::account::Account").unwrap();
    assert_eq!(
        harness
            .read_resource::<AccountResource>(account_alice.address(), account_resource)
            .unwrap()
            .signer_capability_offer()
            .unwrap(),
        *account_bob.address()
    );
}

#[test]
/// Samples a test case for the Move tests for `offer_signer_capability`
fn sample_offer_signer_capability_v2_test_case_for_move() {
    let mut harness = MoveHarness::new();

    let account_alice = harness.new_account_with_key_pair();
    let account_bob = harness.new_account_at(AccountAddress::from_hex_literal("0x345").unwrap());

    // This struct fixes sequence number 0, which is what Alice's account is at in the Move test
    let proof_struct = SignerCapabilityOfferProofChallengeV2 {
        account_address: CORE_CODE_ADDRESS,
        module_name: String::from("account"),
        struct_name: String::from("SignerCapabilityOfferProofChallengeV2"),
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
        "SignerCapabilityOfferProofChallengeV2 signature: {}",
        hex::encode(signature.to_bytes().as_slice())
    );
}
