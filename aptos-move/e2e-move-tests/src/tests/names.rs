// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
//

use crate::{assert_success, MoveHarness};
use aptos_crypto::{
    multi_ed25519::MultiEd25519PublicKey, SigningKey, ValidCryptoMaterialStringExt,
};
use aptos_types::{account_address::AccountAddress, transaction::authenticator::AuthenticationKey};
use cached_packages::aptos_names_sdk_builder;
use serde::{Deserialize, Serialize};

/*
    Below values are for testing only!
    addresses: 0x0ee16f0e4b47d5972f63a642385d52d301e53716b4e1fbd637b9a91a7f1979ba, 0xe5a6fcac1fc4eeec1859d9e395d6c6bc49fa7dd29ca8681e581b0950dcec23df
    public_keys: 0xc5547463e44c3ad8ad52018f0aaf237d39e396b22815cf712493dd61cffabebf, 0xeea1decaa37eb5cdcf99262c6518053126e34283f42ad74f7b91b75fa625c6f8
    private_keys: 0x44c7eabad483e04ce6703a4518d0a74a1356b9c50a3f5cfd4a4c9285591caca6, 0x0afd9ed1d3c00ef22b78a7234f436132317d7fcc69824a16f0c651658929e7f8
    multisig_pub_key: 0xc5547463e44c3ad8ad52018f0aaf237d39e396b22815cf712493dd61cffabebfeea1decaa37eb5cdcf99262c6518053126e34283f42ad74f7b91b75fa625c6f801
    multisig_auth_key: 0x4407b9a063ac530f8b621f7d80b527a79c626791b14b51c1118763ce941b99ce
    threshold: 1/2
*/

// generate a sample signature for testing aptos_names::verify::verify_register_domain_signature()
#[derive(Deserialize, Serialize)]
struct RegisterDomainProofChallenge {
    account_address: AccountAddress,
    module_name: String,
    struct_name: String,
    sequence_number: u64,
    register_address: AccountAddress,
    domain_name: String,
    chain_id: u8,
}

fn get_test_ans_funds_address() -> AccountAddress {
    AccountAddress::from_hex_literal(
        "0x0ee16f0e4b47d5972f63a642385d52d301e53716b4e1fbd637b9a91a7f1979ba",
    )
    .unwrap()
}

fn get_test_ans_admin_multisig_auth_key() -> AuthenticationKey {
    let pub_key = MultiEd25519PublicKey::from_encoded_string(
        "0xc5547463e44c3ad8ad52018f0aaf237d39e396b22815cf712493dd61cffabebfeea1decaa37eb5cdcf99262c6518053126e34283f42ad74f7b91b75fa625c6f801",
    )
        .unwrap();
    let auth_key = AuthenticationKey::multi_ed25519(&pub_key);
    // Ensure the auth key matches the expected on in the comment above
    assert_eq!(
        auth_key.to_string(),
        "4407b9a063ac530f8b621f7d80b527a79c626791b14b51c1118763ce941b99ce".to_string()
    );
    auth_key
}

#[test]
fn test_names_end_to_end() {
    let mut harness = MoveHarness::new();

    let user1 = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    let user2 = harness.new_account_at(AccountAddress::from_hex_literal("0x456").unwrap());
    let ans_account = harness.new_account_at(AccountAddress::from_hex_literal("0x4").unwrap());

    // Run initialization. script. We expect this to be called from genesis or governance proposal script
    assert_success!(harness.run_transaction_payload(
        &ans_account,
        aptos_names_sdk_builder::domains_initialize(
            get_test_ans_funds_address(),
            get_test_ans_admin_multisig_auth_key().derived_address(),
        ),
    ));

    // Register a domain
    assert_success!(harness.run_transaction_payload(
        &user1,
        aptos_names_sdk_builder::domains_register_domain("max".to_string().into_bytes(), 2),
    ));

    // Set the name to point to user 2
    assert_success!(harness.run_transaction_payload(
        &user1,
        aptos_names_sdk_builder::domains_set_domain_address(
            "max".to_string().into_bytes(),
            *user2.address()
        ),
    ));

    // Clear the name, as user2
    assert_success!(harness.run_transaction_payload(
        &user2,
        aptos_names_sdk_builder::domains_clear_domain_address("max".to_string().into_bytes()),
    ));

    // ---- test registering a domain with a valid signature ----
    // set public key
    assert_success!(harness.run_transaction_payload(
        &ans_account,
        aptos_names_sdk_builder::config_set_public_key(ans_account.pubkey.to_bytes().to_vec()),
    ));

    // set unrestricted mint enabled to false, so we will need a valid signature to register domain
    assert_success!(harness.run_transaction_payload(
        &ans_account,
        aptos_names_sdk_builder::config_set_unrestricted_mint_enabled(false),
    ));

    let proof_struct = RegisterDomainProofChallenge {
        account_address: AccountAddress::from_hex_literal("0x4").unwrap(),
        module_name: String::from("verify"),
        struct_name: String::from("RegisterDomainProofChallenge"),
        sequence_number: 12,
        register_address: *user1.address(),
        domain_name: String::from("chloe"),
        chain_id: 4,
    };

    let proof_struct_bytes = bcs::to_bytes(&proof_struct);
    let signature = ans_account
        .privkey
        .sign_arbitrary_message(&proof_struct_bytes.unwrap())
        .to_bytes()
        .to_vec();

    // Register a domain with signature
    assert_success!(harness.run_transaction_payload(
        &user1,
        aptos_names_sdk_builder::domains_register_domain_with_signature(
            "chloe".to_string().into_bytes(),
            2,
            signature
        ),
    ));

    // Set the name to point to user 2
    assert_success!(harness.run_transaction_payload(
        &user1,
        aptos_names_sdk_builder::domains_set_domain_address(
            "chloe".to_string().into_bytes(),
            *user2.address()
        ),
    ));

    // Clear the name, as user2
    assert_success!(harness.run_transaction_payload(
        &user2,
        aptos_names_sdk_builder::domains_clear_domain_address("chloe".to_string().into_bytes()),
    ));
}

/// samples an example signature for `aptos_names::verify::verify_register_domain_signature` unit test
#[test]
fn generate_register_domain_signature_for_test() {
    let mut harness = MoveHarness::new();

    let register_account =
        harness.new_account_at(AccountAddress::from_hex_literal("0x077").unwrap());
    let admin = harness.new_account_at(AccountAddress::from_hex_literal("0x4").unwrap());

    let proof_struct = RegisterDomainProofChallenge {
        account_address: AccountAddress::from_hex_literal("0x4").unwrap(),
        module_name: String::from("verify"),
        struct_name: String::from("RegisterDomainProofChallenge"),
        sequence_number: 0,
        register_address: *register_account.address(),
        domain_name: String::from("test"),
        chain_id: 4,
    };

    let proof_struct_bytes = bcs::to_bytes(&proof_struct);
    let signature = admin
        .privkey
        .sign_arbitrary_message(&proof_struct_bytes.unwrap());

    println!("Admin's PK: {:?}", admin.pubkey);
    println!("signature: {:?}", signature);
}
