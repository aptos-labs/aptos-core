// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_language_e2e_tests::account::Account;
use aptos_types::account_config::AccountResource;
use aptos_types::state_store::table::TableHandle;
use aptos_types::{
    account_address::create_resource_address, account_address::AccountAddress, event::EventHandle,
};
use move_core_types::language_storage::CORE_CODE_ADDRESS;
use move_core_types::parser::parse_struct_tag;
use serde::{Deserialize, Serialize};

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
// This struct includes TypeInfo (account_address, module_name, and struct_name)
// and RotationProofChallenge-specific information (sequence_number, originator, current_auth_key, and new_public_key)
// Since the struct RotationProofChallenge is defined in "0x1::account::RotationProofChallenge",
// we will be passing in "0x1" to `account_address`, "account" to `module_name`, and "RotationProofChallenge" to `struct_name`
// Originator refers to the user's address
#[derive(Serialize, Deserialize)]
pub struct RotationProofChallenge {
    // Should be `CORE_CODE_ADDRESS`
    pub account_address: AccountAddress,
    // Should be `account`
    pub module_name: String,
    // Should be `RotationProofChallenge`
    pub struct_name: String,
    pub sequence_number: u64,
    pub originator: AccountAddress,
    pub current_auth_key: AccountAddress,
    pub new_public_key: Vec<u8>,
}

#[test]
fn test_account_recovery_valid() {
    let mut h = MoveHarness::new();

    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let resource_address = create_resource_address(*acc.address(), &[]);

    // give a named address to the `mint_nft` module publisher
    let mut build_options = aptos_framework::BuildOptions::default();
    build_options
        .named_addresses
        .insert("account_recovery".to_string(), resource_address);
    build_options
        .named_addresses
        .insert("source_addr".to_string(), *acc.address());

    // build the package from our example code
    let package = aptos_framework::BuiltPackage::build(
        common::test_dir_path("../../../move-examples/account_recovery"),
        build_options,
    )
    .expect("building package must succeed");

    let code = package.extract_code();
    let metadata = package
        .extract_metadata()
        .expect("extracting package metadata must succeed");

    // create the resource account and publish the module under the resource account's address
    let result = h.run_transaction_payload(
        &acc,
        aptos_cached_packages::aptos_stdlib::resource_account_create_resource_account_and_publish_package(
            vec![],
            bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
            code,
        ),
    );

    assert_success!(result);

    let owner_account = h.new_account_with_key_pair();
    let delegated_account = h.new_account_with_key_pair();
    register_account_recovery(
        &mut h,
        &resource_address,
        &owner_account,
        delegated_account.address(),
    );
    initiate_account_key_recovery(
        &mut h,
        &resource_address,
        &delegated_account,
        owner_account.address(),
    );
    let new_account = h.new_account_with_key_pair();

    rotate_key(
        &mut h,
        &resource_address,
        &delegated_account,
        &owner_account,
        &new_account,
    );
}

pub fn rotate_key(
    harness: &mut MoveHarness,
    resource_address: &AccountAddress,
    delegated_account: &Account,
    owner_account: &Account,
    new_account: &Account,
) {
    let rotation_proof = RotationProofChallenge {
        account_address: CORE_CODE_ADDRESS,
        module_name: String::from("account"),
        struct_name: String::from("RotationProofChallenge"),
        sequence_number: 0,
        originator: *owner_account.address(),
        current_auth_key: AccountAddress::from_bytes(&owner_account.auth_key()).unwrap(),
        new_public_key: new_account.pubkey.to_bytes().to_vec(),
    };

    let rotation_msg = bcs::to_bytes(&rotation_proof).unwrap();

    // sign the rotation message by the new private key
    let signature_by_new_privkey = new_account.privkey.sign_arbitrary_message(&rotation_msg);

    assert_success!(harness.run_entry_function(
        &delegated_account,
        str::parse(&format!("0x{}::hackathon::rotate_key", resource_address)).unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&owner_account.address()).unwrap(),
            bcs::to_bytes(&new_account.pubkey.to_bytes().to_vec()).unwrap(),
            bcs::to_bytes(&signature_by_new_privkey.to_bytes().to_vec()).unwrap(),
        ],
    ));
}

pub fn initiate_account_key_recovery(
    harness: &mut MoveHarness,
    resource_address: &AccountAddress,
    delegated_account: &Account,
    owner_address: &AccountAddress,
) {
    assert_success!(harness.run_entry_function(
        &delegated_account,
        str::parse(&format!(
            "0x{}::hackathon::initiate_account_key_recovery",
            resource_address
        ))
        .unwrap(),
        vec![],
        vec![bcs::to_bytes(&owner_address).unwrap(),],
    ));
}

pub fn register_account_recovery(
    harness: &mut MoveHarness,
    resource_address: &AccountAddress,
    offerer_account: &Account,
    delegate_address: &AccountAddress,
) {
    let rotation_capability_proof = RotationCapabilityOfferProofChallengeV2 {
        account_address: CORE_CODE_ADDRESS,
        module_name: String::from("account"),
        struct_name: String::from("RotationCapabilityOfferProofChallengeV2"),
        chain_id: 4,
        sequence_number: 0,
        source_address: *offerer_account.address(),
        recipient_address: *resource_address,
    };

    let rotation_capability_proof_msg = bcs::to_bytes(&rotation_capability_proof);
    let rotation_proof_signed = offerer_account
        .privkey
        .sign_arbitrary_message(&rotation_capability_proof_msg.unwrap());

    let authorized_address = delegate_address.clone();
    let required_delay_seconds = 0;

    assert_success!(harness.run_entry_function(
        &offerer_account,
        str::parse(&format!(
            "0x{}::hackathon::register_authorize_one",
            resource_address
        ))
        .unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&authorized_address).unwrap(),
            bcs::to_bytes::<u64>(&required_delay_seconds).unwrap(),
            bcs::to_bytes(&rotation_proof_signed.to_bytes().to_vec()).unwrap(),
            bcs::to_bytes(&offerer_account.pubkey.to_bytes().to_vec()).unwrap(),
        ],
    ));

    let account_resource = parse_struct_tag("0x1::account::Account").unwrap();
    assert_eq!(
        harness
            .read_resource::<AccountResource>(offerer_account.address(), account_resource)
            .unwrap()
            .rotation_capability_offer()
            .unwrap(),
        *resource_address
    );
}
