use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_crypto::{Length, PrivateKey, SigningKey, Uniform};
use aptos_types::{account_address::AccountAddress, account_config::CORE_CODE_ADDRESS, state_store::state_key::StateKey, state_store::table::TableHandle, transaction::authenticator::AuthenticationKey};
use cached_framework_packages::aptos_stdlib;
use e2e_move_tests::{assert_success, enable_golden, MoveHarness};
use move_deps::move_core_types::parser::parse_struct_tag;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct OriginatingAddress {
    handle: u128,
}

#[derive(Serialize, Deserialize)]
struct Proof {
    account_address: AccountAddress,
    module_name: String,
    struct_name: String,
    originator: AccountAddress,
    current_auth_key: AccountAddress,
}

#[test]
fn key_rotation() {
    let mut harness = MoveHarness::new();
    enable_golden!(harness);

    let account1 = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    let address = account1.address();
    let new_private_key = Ed25519PrivateKey::generate_for_testing();
    let new_public_key = new_private_key.public_key();
    let new_auth_key = AuthenticationKey::ed25519(&new_public_key);

    let rotation_proof = Proof {
        account_address: CORE_CODE_ADDRESS,
        module_name: String::from("account"),
        struct_name: String::from("RotationProof"),
        originator: *account1.address(),
        current_auth_key: AccountAddress::from_bytes(&account1.auth_key()).unwrap(),
    };

    let msg = bcs::to_bytes(&rotation_proof);
    let signature = new_private_key.sign_arbitrary_message(&msg.unwrap());

    assert_success!(harness.run_transaction_payload(
        &account1,
        aptos_stdlib::account_rotate_authentication_key_ed25519(
            signature.to_bytes().to_vec(),
            new_public_key.to_bytes().to_vec(),
        )
    ));

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
    let result = harness.read_state_value(state_key).unwrap();
    assert_eq!(result, address.to_vec());
}