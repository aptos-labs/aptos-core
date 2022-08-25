// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519PublicKey};
use aptos_crypto::SigningKey;
use aptos_types::{
    account_address::AccountAddress, account_config::CORE_CODE_ADDRESS,
    state_store::state_key::StateKey, state_store::table::TableHandle,
};

use cached_packages::aptos_stdlib;
use e2e_move_tests::{assert_success, MoveHarness};
use language_e2e_tests::account::Account;
use move_deps::move_core_types::parser::parse_struct_tag;
use serde::{Deserialize, Serialize};

// This struct includes TypeInfo (account_address, module_name, and struct_name)
// and RotationProofChallenge-specific information (sequence_number, originator, current_auth_key, and new_public_key)
// Since the struct RotationProofChallenge is defined in "0x1::account::RotationProofChallenge",
// we will be passing in "0x1" to `account_address`, "account" to `module_name`, and "RotationProofChallenge" to `struct_name`
// Originator refers to the user's address
#[derive(Serialize, Deserialize)]
struct RotationProofChallenge {
    account_address: AccountAddress,
    module_name: String,
    struct_name: String,
    sequence_number: u64,
    originator: AccountAddress,
    current_auth_key: AccountAddress,
    new_public_key: Vec<u8>,
}

#[test]
fn rotate_auth_key_twice() {
    let mut harness = MoveHarness::new();
    let mut account1 = harness.new_account_with_key_pair();

    let account2 = harness.new_account_with_key_pair();
    // assert that the payload is successfully processed (the signatures are correct)
    assert_successful_payload_key_rotation(
        &mut harness,
        account1.clone(),
        *account1.address(),
        10,
        account2.privkey.clone(),
        account2.pubkey.clone(),
    );
    // rotate account1's keypair to account2
    account1.rotate_key(account2.privkey, account2.pubkey);
    // verify that we can still get to account1's originating address
    verify_originating_address(&mut harness, account1.auth_key(), *account1.address());

    let account3 = harness.new_account_with_key_pair();
    assert_successful_payload_key_rotation(
        &mut harness,
        account1.clone(),
        *account1.address(),
        11,
        account3.privkey.clone(),
        account3.pubkey.clone(),
    );
    account1.rotate_key(account3.privkey, account3.pubkey);
    verify_originating_address(&mut harness, account1.auth_key(), *account1.address());
}

pub fn assert_successful_payload_key_rotation(
    harness: &mut MoveHarness,
    current_account: Account,
    originator: AccountAddress,
    sequence_number: u64,
    new_private_key: Ed25519PrivateKey,
    new_public_key: Ed25519PublicKey,
) {
    // construct a proof challenge struct that proves that
    // the user intends to rotate their auth key
    let rotation_proof = RotationProofChallenge {
        account_address: CORE_CODE_ADDRESS,
        module_name: String::from("account"),
        struct_name: String::from("RotationProofChallenge"),
        sequence_number,
        originator,
        current_auth_key: AccountAddress::from_bytes(&current_account.auth_key()).unwrap(),
        new_public_key: new_public_key.to_bytes().to_vec(),
    };

    let rotation_msg = bcs::to_bytes(&rotation_proof);

    // sign the rotation message by the current private key and the new private key
    let signature_by_curr_privkey = current_account
        .privkey
        .sign_arbitrary_message(&rotation_msg.clone().unwrap());
    let signature_by_new_privkey = new_private_key.sign_arbitrary_message(&rotation_msg.unwrap());

    assert_success!(harness.run_transaction_payload(
        &current_account,
        aptos_stdlib::account_rotate_authentication_key_ed25519(
            signature_by_curr_privkey.to_bytes().to_vec(),
            signature_by_new_privkey.to_bytes().to_vec(),
            current_account.pubkey.to_bytes().to_vec(),
            new_public_key.to_bytes().to_vec(),
        )
    ));
}

pub fn verify_originating_address(
    harness: &mut MoveHarness,
    auth_key: Vec<u8>,
    expected_address: AccountAddress,
) {
    // get the address redirection table
    let originating_address_handle = harness
        .read_resource::<TableHandle>(
            &CORE_CODE_ADDRESS,
            parse_struct_tag("0x1::account::OriginatingAddress").unwrap(),
        )
        .unwrap();
    let state_key = &StateKey::table_item(
        originating_address_handle,
        AccountAddress::from_bytes(auth_key).unwrap().to_vec(),
    );
    // verify that the value in the address redirection table is expected
    let result = harness.read_state_value(state_key).unwrap();
    assert_eq!(result, expected_address.to_vec());
}

#[test]
fn rotate_auth_key_twice() {
    let mut harness = MoveHarness::new();

    let account1 = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    let address = account1.address();
    let account2 = harness.new_account_at(AccountAddress::from_hex_literal("0x234").unwrap());
    let new_private_key = account2.privkey.clone();
    let new_public_key = account2.pubkey.clone();
    let new_auth_key = account2.auth_key();

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

    let new_private_key2 = Ed25519PrivateKey::generate_for_testing();
    let new_public_key2 = new_private_key2.public_key();
    let new_auth_key2 = AuthenticationKey::ed25519(&new_public_key2);

    let rotation_proof2 = RotationProofChallenge {
        account_address: CORE_CODE_ADDRESS,
        module_name: String::from("account"),
        struct_name: String::from("RotationProofChallenge"),
        sequence_number: 11,
        originator: *account1.address(),
        current_auth_key: AccountAddress::from_bytes(account2.auth_key()).unwrap(),
        new_public_key: new_public_key2.to_bytes().to_vec(),
    };

    let rotation_msg2  = bcs::to_bytes(&rotation_proof2);

    let rotation_proof_signed_by_current_private_key2 = account1.privkey.sign_arbitrary_message(&rotation_msg2.clone().unwrap());
    let rotation_proof_signed_by_new_private_key2 = new_private_key2.sign_arbitrary_message(&rotation_msg2.unwrap());

    assert_success!(harness.run_transaction_payload(
        &account1,
        aptos_stdlib::account_rotate_authentication_key_ed25519(
            rotation_proof_signed_by_current_private_key2.to_bytes().to_vec(),
            rotation_proof_signed_by_new_private_key2.to_bytes().to_vec(),
            account1.pubkey.to_bytes().to_vec(),
            new_public_key2.to_bytes().to_vec(),
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
        AccountAddress::from_bytes(new_auth_key2).unwrap().to_vec(),
    );
    // verify that the value in the address redirection table is expected
    let result = harness.read_state_value(state_key).unwrap();
    assert_eq!(result, address.to_vec());
}


#[test]
fn rotate_auth_key_twice2() {
    let mut harness = MoveHarness::new();

    let account1 = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    let address = account1.address();
    let account2 = harness.new_account_at(AccountAddress::from_hex_literal("0x234").unwrap());
    let new_private_key = account2.privkey.clone();
    let new_public_key = account2.pubkey.clone();
    let new_auth_key = account2.auth_key();

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

    // second time rotating key
    let new_private_key2 = Ed25519PrivateKey::generate_for_testing();
    let new_public_key2 = new_private_key2.public_key();
    let new_auth_key2 = AuthenticationKey::ed25519(&new_public_key2);

    let rotation_proof2 = RotationProofChallenge {
        account_address: CORE_CODE_ADDRESS,
        module_name: String::from("account"),
        struct_name: String::from("RotationProofChallenge"),
        sequence_number: 10,
        originator: *account1.address(),
        current_auth_key: AccountAddress::from_bytes(account2.auth_key()).unwrap(),
        new_public_key: new_public_key2.to_bytes().to_vec(),
    };

    let rotation_msg2  = bcs::to_bytes(&rotation_proof2);

    let rotation_proof_signed_by_current_private_key2 = account2.privkey.sign_arbitrary_message(&rotation_msg2.clone().unwrap());
    let rotation_proof_signed_by_new_private_key2 = new_private_key2.sign_arbitrary_message(&rotation_msg2.unwrap());

    assert_success!(harness.run_transaction_payload(
        &account2,
        aptos_stdlib::account_rotate_authentication_key_ed25519(
            rotation_proof_signed_by_current_private_key2.to_bytes().to_vec(),
            rotation_proof_signed_by_new_private_key2.to_bytes().to_vec(),
            account2.pubkey.to_bytes().to_vec(),
            new_public_key2.to_bytes().to_vec(),
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
        AccountAddress::from_bytes(new_auth_key2).unwrap().to_vec(),
    );
    // verify that the value in the address redirection table is expected
    let result = harness.read_state_value(state_key).unwrap();
    assert_eq!(result, address.to_vec());
}