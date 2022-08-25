// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_crypto::{PrivateKey, SigningKey, Uniform};
use aptos_types::{
    account_address::AccountAddress, account_config::CORE_CODE_ADDRESS,
    state_store::state_key::StateKey, state_store::table::TableHandle,
    transaction::authenticator::AuthenticationKey,
};

use aptos::account::key_rotation::RotationProofChallenge;
use cached_packages::aptos_stdlib;
use e2e_move_tests::{assert_success, MoveHarness};
use move_deps::move_core_types::parser::parse_struct_tag;

#[test]
fn rotate_auth_key() {
    let mut harness = MoveHarness::new();

    let account1 = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    let address = account1.address();
    let new_private_key = Ed25519PrivateKey::generate_for_testing();
    let new_public_key = new_private_key.public_key();
    let new_auth_key = AuthenticationKey::ed25519(&new_public_key);

    // create an instance of RotationProofChallenge that includes information about the current account
    // and the new public key
    let rotation_proof = RotationProofChallenge {
        account_address: CORE_CODE_ADDRESS,
        module_name: String::from("account"),
        struct_name: String::from("RotationProofChallenge"),
        sequence_number: 10,
        originator: *account1.address(),
        current_auth_key: AccountAddress::from_bytes(&account1.auth_key()).unwrap(),
        new_public_key: new_public_key.to_bytes().to_vec(),
    };

    let rotation_msg = bcs::to_bytes(&rotation_proof);

    // sign the struct using both the current private key and the next private key
    let rotation_proof_signed_by_current_private_key = account1
        .privkey
        .sign_arbitrary_message(&rotation_msg.clone().unwrap());
    let rotation_proof_signed_by_new_private_key =
        new_private_key.sign_arbitrary_message(&rotation_msg.unwrap());

    assert_success!(harness.run_transaction_payload(
        &account1,
        aptos_stdlib::account_rotate_authentication_key_ed25519(
            rotation_proof_signed_by_current_private_key
                .to_bytes()
                .to_vec(),
            rotation_proof_signed_by_new_private_key.to_bytes().to_vec(),
            account1.pubkey.to_bytes().to_vec(),
            new_public_key.to_bytes().to_vec(),
        )
    ));

    // get the address redirection table
    let originating_address_handle = harness
        .read_resource::<TableHandle>(
            &CORE_CODE_ADDRESS,
            parse_struct_tag("0x1::account::OriginatingAddress").unwrap(),
        )
        .unwrap();
    let state_key = &StateKey::table_item(
        originating_address_handle,
        AccountAddress::from_bytes(new_auth_key).unwrap().to_vec(),
    );
    // verify that the value in the address redirection table is expected
    let result = harness.read_state_value(state_key).unwrap();
    assert_eq!(result, address.to_vec());
}
