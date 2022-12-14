// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_language_e2e_tests::account::Account;
use aptos_types::state_store::table::TableHandle;
use aptos_types::{
    account_address::create_resource_address, account_address::AccountAddress, event::EventHandle,
};
use move_core_types::language_storage::CORE_CODE_ADDRESS;
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
    // register_account_recovery(
    //     &mut h,
    //     &resource_address,
    //     &owner_account,
    //     &delegated_account,
    // )
}

pub fn register_account_recovery(
    harness: &mut MoveHarness,
    resource_address: &AccountAddress,
    offerer_account: &Account,
    delegate_account: &Account,
) {
    let rotation_capability_proof = RotationCapabilityOfferProofChallengeV2 {
        account_address: CORE_CODE_ADDRESS,
        module_name: String::from("account"),
        struct_name: String::from("RotationCapabilityOfferProofChallengeV2"),
        chain_id: 4,
        sequence_number: 0,
        source_address: *offerer_account.address(),
        recipient_address: *delegate_account.address(),
    };

    let rotation_capability_proof_msg = bcs::to_bytes(&rotation_capability_proof);
    let rotation_proof_signed = offerer_account
        .privkey
        .sign_arbitrary_message(&rotation_capability_proof_msg.unwrap());

    let authorized_address = vec![delegate_account.address().clone()];
    let required_num_recovery = 1;
    let required_delay_seconds = 0;
    let rotate_valid_window_seconds = 0;
    let allow_unauthorized_initiation = false;

    assert_success!(harness.run_entry_function(
        &offerer_account,
        str::parse(&format!("0x{}::hackathon::register", resource_address)).unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&authorized_address).unwrap(),
            bcs::to_bytes::<u64>(&required_num_recovery).unwrap(),
            bcs::to_bytes::<u64>(&required_delay_seconds).unwrap(),
            bcs::to_bytes::<u64>(&rotate_valid_window_seconds).unwrap(),
            bcs::to_bytes(&allow_unauthorized_initiation).unwrap(),
            rotation_proof_signed.to_bytes().to_vec(),
            offerer_account.pubkey.to_bytes().to_vec(),
        ],
    ));

    // let account_resource = parse_struct_tag("0x1::account::Account").unwrap();
    // assert_eq!(
    //     harness
    //         .read_resource::<AccountResource>(offerer_account.address(), account_resource)
    //         .unwrap()
    //         .rotation_capability_offer()
    //         .unwrap(),
    //     *delegate_account.address()
    // );
}
