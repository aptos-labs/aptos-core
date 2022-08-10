use aptos_types::account_address::AccountAddress;
use e2e_move_tests::{assert_success, enable_golden, MoveHarness};
use move_deps::move_core_types::parser::parse_struct_tag;
use serde::{Deserialize, Serialize};
use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519PublicKey};
use aptos_crypto::{SigningKey, Uniform};
use aptos_types::account_config::CORE_CODE_ADDRESS;
use aptos_types::state_store::state_key::StateKey;
use aptos_types::state_store::table::TableHandle;
use aptos_types::transaction::authenticator::AuthenticationKey;
use cached_framework_packages::aptos_stdlib;

#[derive(Serialize, Deserialize)]
struct OriginatingAddress {
    table_handle: TableHandle,
}

#[derive(Serialize, Deserialize)]
struct RotationProof {
    account_address: AccountAddress,
    module_name: String,
    struct_name: String,
    originator: AccountAddress,
    current_auth_key: Vec<u8>,
}

#[test]
fn key_rotation() {
    let mut harness = MoveHarness::new();
    enable_golden!(harness);

    let account1 = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    let address = account1.address();
    let new_private_key = Ed25519PrivateKey::generate_for_testing();
    let new_public_key = Ed25519PublicKey::from(&new_private_key);
    let new_auth_key = AuthenticationKey::ed25519(&new_public_key);

    let rotation_proof = RotationProof {
        account_address: CORE_CODE_ADDRESS,
        module_name: String::from("account"),
        struct_name: String::from("RotationProof"),
        originator: *account1.address(),
        current_auth_key: account1.auth_key(),
    };

    let msg = bcs::to_bytes(&rotation_proof);
    let signature = account1.privkey.sign_arbitrary_message(&msg.unwrap());

    assert_success!(harness.run_transaction_payload(&account1,
        aptos_stdlib::account_rotate_authentication_key_ed25519(new_public_key.to_bytes().to_vec(), signature.to_bytes().to_vec())));
    let address_map = get_originating_address(&harness, &CORE_CODE_ADDRESS);
    let state_key = StateKey::table_item(address_map.table_handle, new_auth_key.to_vec());
    let value = harness.read_state_value(&state_key);
    let value: AccountAddress = bcs::from_bytes(&value.unwrap()).unwrap();
    assert_eq!(value, *address);
}

fn get_originating_address(harness: &MoveHarness, pool_address: &AccountAddress) -> OriginatingAddress {
    harness.read_resource::<OriginatingAddress>(
        pool_address,
        parse_struct_tag("0x1::account::OriginatingAddress").unwrap(),
    ).unwrap()
}
