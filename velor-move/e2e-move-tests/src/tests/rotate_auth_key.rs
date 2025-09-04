// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    assert_abort, assert_success,
    tests::offer_rotation_capability::{offer_rotation_capability_v2, revoke_rotation_capability},
    MoveHarness,
};
use velor_cached_packages::velor_stdlib;
use velor_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    multi_ed25519::{MultiEd25519PrivateKey, MultiEd25519PublicKey},
    Signature, SigningKey, Uniform, ValidCryptoMaterial,
};
use velor_language_e2e_tests::account::Account;
use velor_types::{
    account_address::AccountAddress,
    account_config::{RotationProofChallenge, CORE_CODE_ADDRESS},
    state_store::{state_key::StateKey, table::TableHandle},
    transaction::{authenticator::AuthenticationKey, TransactionStatus},
};
use move_core_types::parser::parse_struct_tag;

#[test]
fn rotate_auth_key_ed25519_to_ed25519() {
    let mut harness = MoveHarness::new();
    let account1 = harness.new_account_with_key_pair();
    let account2 = harness.new_account_with_key_pair();

    // assert that the payload is successfully processed (the signatures are correct)
    assert_successful_key_rotation_transaction(
        0,
        0,
        &mut harness,
        account1.clone(),
        *account1.address(),
        0,
        account2.privkey.clone(),
        account2.pubkey.to_bytes(),
    );

    // verify that we can still get to account1's originating address
    verify_originating_address(&mut harness, account2.auth_key(), *account1.address());
}

#[test]
fn rotate_auth_key_ed25519_to_multi_ed25519() {
    let mut harness = MoveHarness::new();
    let account1 = harness.new_account_with_key_pair();
    let private_key = MultiEd25519PrivateKey::generate_for_testing();
    let public_key = MultiEd25519PublicKey::from(&private_key);
    let auth_key = AuthenticationKey::multi_ed25519(&public_key);

    // assert that the payload is successfully processed (the signatures are correct)
    assert_successful_key_rotation_transaction(
        0,
        1,
        &mut harness,
        account1.clone(),
        *account1.address(),
        0,
        private_key,
        public_key.to_bytes(),
    );

    // verify that we can still get to account1's originating address
    verify_originating_address(&mut harness, auth_key.to_vec(), *account1.address());
}

#[test]
fn rotate_auth_key_twice() {
    let mut harness = MoveHarness::new();
    let mut account1 = harness.new_account_with_key_pair();
    let account2 = harness.new_account_with_key_pair();

    // assert that the payload is successfully processed (the signatures are correct)
    assert_successful_key_rotation_transaction(
        0,
        0,
        &mut harness,
        account1.clone(),
        *account1.address(),
        0,
        account2.privkey.clone(),
        account2.pubkey.to_bytes(),
    );
    // rotate account1's keypair to account2
    account1.rotate_key(account2.privkey, account2.pubkey.as_ed25519().unwrap());
    // verify that we can still get to account1's originating address
    verify_originating_address(&mut harness, account1.auth_key(), *account1.address());

    let account3 = harness.new_account_with_key_pair();
    assert_successful_key_rotation_transaction(
        0,
        0,
        &mut harness,
        account1.clone(),
        *account1.address(),
        1,
        account3.privkey.clone(),
        account3.pubkey.to_bytes(),
    );
    account1.rotate_key(account3.privkey, account3.pubkey.as_ed25519().unwrap());
    verify_originating_address(&mut harness, account1.auth_key(), *account1.address());
}

#[test]
fn rotate_auth_key_with_rotation_capability_e2e() {
    let mut harness = MoveHarness::new();
    let delegate_account = harness.new_account_with_key_pair();
    let mut offerer_account = harness.new_account_with_key_pair();

    offer_rotation_capability_v2(&mut harness, &offerer_account, &delegate_account);
    let new_private_key = Ed25519PrivateKey::generate_for_testing();
    let new_public_key = Ed25519PublicKey::from(&new_private_key);
    assert_success!(run_rotate_auth_key_with_rotation_capability(
        &mut harness,
        &mut offerer_account,
        &delegate_account,
        &new_private_key,
        &new_public_key
    ));
    offerer_account.rotate_key(new_private_key.clone(), new_public_key.clone());
    verify_originating_address(
        &mut harness,
        offerer_account.auth_key(),
        *offerer_account.address(),
    );

    revoke_rotation_capability(&mut harness, &offerer_account, *delegate_account.address());
    assert_abort!(
        run_rotate_auth_key_with_rotation_capability(
            &mut harness,
            &mut offerer_account,
            &delegate_account,
            &new_private_key,
            &new_public_key
        ),
        _
    );
}

fn run_rotate_auth_key_with_rotation_capability(
    harness: &mut MoveHarness,
    offerer_account: &mut Account,
    delegate_account: &Account,
    new_private_key: &Ed25519PrivateKey,
    new_public_key: &Ed25519PublicKey,
) -> TransactionStatus {
    let rotation_proof = RotationProofChallenge {
        account_address: CORE_CODE_ADDRESS,
        module_name: String::from("account"),
        struct_name: String::from("RotationProofChallenge"),
        sequence_number: 0,
        originator: *offerer_account.address(),
        current_auth_key: AccountAddress::from_bytes(offerer_account.auth_key()).unwrap(),
        new_public_key: new_public_key.to_bytes().to_vec(),
    };

    let rotation_msg = bcs::to_bytes(&rotation_proof).unwrap();

    // sign the rotation message by the new private key
    let signature_by_new_privkey = new_private_key.sign_arbitrary_message(&rotation_msg);

    harness.run_transaction_payload(
        delegate_account,
        velor_stdlib::account_rotate_authentication_key_with_rotation_capability(
            *offerer_account.address(),
            0,
            new_public_key.to_bytes().to_vec(),
            signature_by_new_privkey.to_bytes().to_vec(),
        ),
    )
}

pub fn assert_successful_key_rotation_transaction<S: SigningKey + ValidCryptoMaterial>(
    from_scheme: u8,
    to_scheme: u8,
    harness: &mut MoveHarness,
    current_account: Account,
    originator: AccountAddress,
    sequence_number: u64,
    new_private_key: S,
    new_public_key_bytes: Vec<u8>,
) {
    // Construct a proof challenge struct that proves that
    // the user intends to rotate their auth key.
    let rotation_proof = RotationProofChallenge {
        account_address: CORE_CODE_ADDRESS,
        module_name: String::from("account"),
        struct_name: String::from("RotationProofChallenge"),
        sequence_number,
        originator,
        current_auth_key: AccountAddress::from_bytes(current_account.auth_key()).unwrap(),
        new_public_key: new_public_key_bytes.clone(),
    };

    let rotation_msg = bcs::to_bytes(&rotation_proof).unwrap();

    // Sign the rotation message by the current private key and the new private key.
    let signature_by_curr_privkey = current_account
        .privkey
        .sign_arbitrary_message(&rotation_msg);
    let signature_by_new_privkey = new_private_key.sign_arbitrary_message(&rotation_msg);

    assert_success!(harness.run_transaction_payload(
        &current_account,
        velor_stdlib::account_rotate_authentication_key(
            from_scheme,
            current_account.pubkey.to_bytes(),
            to_scheme,
            new_public_key_bytes,
            signature_by_curr_privkey.to_bytes().to_vec(),
            signature_by_new_privkey.to_bytes().to_vec(),
        )
    ));
}

pub fn verify_originating_address(
    harness: &mut MoveHarness,
    auth_key: Vec<u8>,
    expected_address: AccountAddress,
) {
    // Get the address redirection table
    let originating_address_handle = harness
        .read_resource::<TableHandle>(
            &CORE_CODE_ADDRESS,
            parse_struct_tag("0x1::account::OriginatingAddress").unwrap(),
        )
        .unwrap();
    let state_key = &StateKey::table_item(&originating_address_handle, &auth_key);
    // Verify that the value in the address redirection table is expected
    let result = harness.read_state_value_bytes(state_key).unwrap();
    assert_eq!(result, expected_address.to_vec());
}
