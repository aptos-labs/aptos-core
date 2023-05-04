// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519Signature},
    SigningKey, ValidCryptoMaterialStringExt,
};
use aptos_types::{
    account_address::{create_resource_address, AccountAddress},
    event::EventHandle,
    state_store::{table::TableHandle},
};
use move_core_types::{parser::parse_struct_tag};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct Token {
    collection: AccountAddress,
    index: u64,
    description: String,
    name: String,
    uri: String,
    mutation_events: EventHandle,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct Collection {
    creator: AccountAddress,
    description: String,
    name: String,
    uri: String,
    mutation_events: EventHandle,
}

#[derive(Deserialize, Serialize)]
struct MintProofChallenge {
    account_address: AccountAddress,
    module_name: String,
    struct_name: String,
    receiver_account_sequence_number: u64,
    receiver_account_address: AccountAddress,
    collection_name: String,
    creator: AccountAddress,
    token_name: String,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct ObjectCore {
    guid_creation_num: u64,
    owner: AccountAddress,
    allow_ungated_transfer: bool,
    transfer_events: EventHandle,
}

#[derive(Deserialize, Serialize, Debug)]
struct ObjectAddresses {
    collection_object_address: AccountAddress,
    last_minted_token_object_address: Option<AccountAddress>,
}

#[derive(Deserialize, Serialize)]
struct TokenStore {
    tokens: TableHandle,
    direct_transfer: bool,
    deposit_events: EventHandle,
    withdraw_events: EventHandle,
    burn_events: EventHandle,
    mutate_token_property_events: EventHandle,
}

#[test]
fn mint_nft_v2_e2e() {
    let mut h = MoveHarness::new();

    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let resource_address = create_resource_address(*acc.address(), &[]);

    // give a named address to the `mint_nft_v2` module publisher
    let mut build_options = aptos_framework::BuildOptions::default();
    build_options
        .named_addresses
        .insert("mint_nft_v2".to_string(), resource_address);
    build_options
        .named_addresses
        .insert("source_addr".to_string(), *acc.address());

    // build the package from our example code
    let package = aptos_framework::BuiltPackage::build(
        common::test_dir_path("../../../move-examples/mint_nft_v2/4-Getting-Production-Ready"),
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

    let nft_receiver = h.new_account_with_key_pair();
    let mint_proof = MintProofChallenge {
        account_address: resource_address,
        module_name: String::from("create_nft_getting_production_ready"),
        struct_name: String::from("MintProofChallenge"),
        receiver_account_sequence_number: 0,
        receiver_account_address: *nft_receiver.address(),
        collection_name: String::from("Collection name"),
        creator: resource_address,
        token_name: String::from("Token name"),
    };

    // sign the MintProofChallenge using the resource account's private key
    let mint_proof_msg = bcs::to_bytes(&mint_proof);
    let resource_account = h.new_account_at(resource_address);
    let mint_proof_signature = resource_account
        .privkey
        .sign_arbitrary_message(&mint_proof_msg.unwrap());

    // call mint_event_ticket function with the user's mint proof signature and public key
    assert_success!(h.run_entry_function(
        &nft_receiver,
        str::parse(&format!(
            "0x{}::create_nft_getting_production_ready::mint_event_ticket",
            resource_address
        ))
        .unwrap(),
        vec![],
        // pass in resource account's signature
        vec![bcs::to_bytes::<Ed25519Signature>(&mint_proof_signature).unwrap(),],
    ));

    let object_addresses = h
        .read_resource::<ObjectAddresses>(
            &resource_address,
            parse_struct_tag(&format!("0x{}::create_nft_getting_production_ready::ObjectAddresses", resource_address.to_string())).unwrap(),
        )
        .unwrap();

    assert!(object_addresses.last_minted_token_object_address.is_some());

    let obj_tag = parse_struct_tag("0x1::object::ObjectCore").unwrap();
    let token_obj_tag = parse_struct_tag("0x4::token::Token").unwrap();
    let collection_obj_tag = parse_struct_tag("0x4::collection::Collection").unwrap();
    let obj_group_tag = parse_struct_tag("0x1::object::ObjectGroup").unwrap();

    let collection_addr = object_addresses.collection_object_address;
    let token_addr = object_addresses.last_minted_token_object_address.unwrap();
    // Ensure that the group data can be read
    let token_object: ObjectCore = h
        .read_resource_from_resource_group(&token_addr, obj_group_tag.clone(), obj_tag.clone())
        .unwrap();
    let collection_object: ObjectCore = h
        .read_resource_from_resource_group(&collection_addr, obj_group_tag.clone(), obj_tag.clone())
        .unwrap();
    let token_data: Token = h
        .read_resource_from_resource_group(&token_addr, obj_group_tag.clone(), token_obj_tag.clone())
        .unwrap();
    let collection_data: Collection = h
        .read_resource_from_resource_group(&collection_addr, obj_group_tag.clone(),  collection_obj_tag.clone())
        .unwrap();

    assert!(
        token_object.owner == *nft_receiver.address() &&
        collection_object.owner == resource_address &&
        token_data.name == "Token name" &&
        token_data.collection == collection_addr &&
        collection_data.creator == resource_address &&
        collection_data.name == "Collection name",
    );
}

/// samples two signatures for unit tests in move-examples/
/// 4-Getting-Production-Ready/sources/create_nft_getting_production_ready.move
#[test]
fn sample_mint_nft_v2_part4_unit_test_signature() {
    let mut h = MoveHarness::new();

    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let resource_address = create_resource_address(*acc.address(), &[]);

    let resource_account = h.new_account_at(resource_address);
    let nft_receiver1 = h.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    let nft_receiver2 = h.new_account_at(AccountAddress::from_hex_literal("0x234").unwrap());

    let mint_proof1 = MintProofChallenge {
        account_address: resource_address,
        module_name: String::from("create_nft_getting_production_ready"),
        struct_name: String::from("MintProofChallenge"),
        receiver_account_sequence_number: 0,
        receiver_account_address: *nft_receiver1.address(),
        collection_name: String::from("Collection name"),
        creator: resource_address,
        token_name: String::from("Token name1"),
    };

    let mint_proof2 = MintProofChallenge {
        account_address: resource_address,
        module_name: String::from("create_nft_getting_production_ready"),
        struct_name: String::from("MintProofChallenge"),
        receiver_account_sequence_number: 0,
        receiver_account_address: *nft_receiver2.address(),
        collection_name: String::from("Collection name"),
        creator: resource_address,
        token_name: String::from("Token name2"),
    };

    // sign the MintProofChallenge using the resource account's private key
    let mint_proof_msg1 = bcs::to_bytes(&mint_proof1);
    let mint_proof_signature1 = resource_account
        .privkey
        .sign_arbitrary_message(&mint_proof_msg1.unwrap());

    let mint_proof_msg2 = bcs::to_bytes(&mint_proof2);
    let mint_proof_signature2 = resource_account
        .privkey
        .sign_arbitrary_message(&mint_proof_msg2.unwrap());

    println!(
        "Mint Proof Signature for NFT receiver 1: {:?}",
        mint_proof_signature1
    );
    println!(
        "Mint Proof Signature for NFT receiver 2: {:?}",
        mint_proof_signature2
    );
}

/// Run `cargo test generate_nft_v2_tutorial_part4_signature -- --nocapture`
/// to generate a valid signature for `[resource_account_address]::create_nft_getting_production_ready::mint_event_pass()` function
/// in `aptos-move/move-examples/mint_nft_v2/4-Getting-Production-Ready/sources/create_nft_getting_production_ready.move`.
#[test]
fn generate_nft_v2_tutorial_part4_signature() {
    let mut h = MoveHarness::new();

    // When running this test to generate a valid signature, supply the actual resource_address to line 217.
    // Uncomment line 223 and comment out line 224 (it's just a placeholder).
    // let resource_address = h.new_account_at(AccountAddress::from_hex_literal("0x[resource account's address]").unwrap());
    let resource_address = h.new_account_at(AccountAddress::from_hex_literal("0xef6a3dcd3846a953f329a7acc6a67b0eebebe6405d9f7b13b278282f8937dedd").unwrap());

    // When running this test to generate a valid signature, supply the actual nft_receiver's address to line 222.
    // Uncomment line 228 and comment out line 229.
    // let nft_receiver = h.new_account_at(AccountAddress::from_hex_literal("0x[nft-receiver's address]").unwrap());
    let nft_receiver = h.new_account_at(AccountAddress::from_hex_literal("0xde6d490bb68ecb648b14fe4550093ef49d32bc98e41d6393ce5f83cb397f7065").unwrap());

    // When running this test to generate a valid signature, supply the actual private key to replace the (0000...) in line 232.
    let admin_private_key = Ed25519PrivateKey::from_encoded_string(
        "f3569cace7aebe6a4adca6399ffe063e816564bedaa97eb5eb334f9c431af76e",
    )
    .unwrap();

    let mint_proof = MintProofChallenge {
        account_address: *resource_address.address(),
        module_name: String::from("create_nft_getting_production_ready"),
        struct_name: String::from("MintProofChallenge"),
        // change the `receiver_account_sequence_number` to the right sequence number
        // you can find an account's sequence number by searching for the account's address on explorer.aptoslabs.com and going to the `Info` tab
        receiver_account_sequence_number: 2,
        receiver_account_address: *nft_receiver.address(),
        collection_name: String::from("Collection name"),
        creator: *resource_address.address(),
        token_name: String::from("Token name"),
    };

    // sign the MintProofChallenge using the resource account's private key
    let mint_proof_msg = bcs::to_bytes(&mint_proof);

    let mint_proof_signature = admin_private_key.sign_arbitrary_message(&mint_proof_msg.unwrap());
    println!(
        "Mint Proof Signature for NFT receiver: {:?}",
        mint_proof_signature
    );
}
