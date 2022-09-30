// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_crypto::ed25519::Ed25519Signature;
use aptos_crypto::SigningKey;
use aptos_types::state_store::table::TableHandle;
use aptos_types::{
    account_address::AccountAddress, event::EventHandle, state_store::state_key::StateKey,
};
use move_deps::move_core_types::parser::parse_struct_tag;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct TokenDataId {
    creator: AccountAddress,
    collection: Vec<u8>,
    name: Vec<u8>,
}

#[derive(Deserialize, Serialize)]
struct TokenId {
    token_data_id: TokenDataId,
    property_version: u64,
}

#[derive(Deserialize, Serialize)]
struct MintProofChallenge {
    account_address: AccountAddress,
    module_name: String,
    struct_name: String,
    receiver_account_sequence_number: u64,
    receiver_account_address: AccountAddress,
    token_data_id: TokenDataId,
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
fn mint_nft_e2e() {
    let mut h = MoveHarness::new();

    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let resource_address =
        aptos_types::account_address::create_resource_address(*acc.address(), vec![].as_slice());

    // give a named address to the `mint_nft` module
    let mut build_options = framework::BuildOptions::default();
    build_options
        .named_addresses
        .insert("mint_nft".to_string(), resource_address);

    // build the package from our example code
    let package = framework::BuiltPackage::build(
        common::test_dir_path("../../../move-examples/mint_nft"),
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
        cached_packages::aptos_stdlib::resource_account_create_resource_account_and_publish_package(
            vec![],
            bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
            code,
        ),
    );

    assert_success!(result);

    let resource_accoount = h.new_account_at(resource_address);
    let nft_receiver = h.new_account_with_key_pair();

    // construct the token_data_id and mint_proof, which are required to mint the nft
    let token_data_id = TokenDataId {
        creator: resource_address,
        collection: String::from("Collection name").into_bytes(),
        name: String::from("Token name").into_bytes(),
    };

    let mint_proof = MintProofChallenge {
        account_address: resource_address,
        module_name: String::from("minting"),
        struct_name: String::from("MintProofChallenge"),
        receiver_account_sequence_number: 10,
        receiver_account_address: *nft_receiver.address(),
        token_data_id,
    };

    // sign the MintProofChallenge using the resource account's private key
    let mint_proof_msg = bcs::to_bytes(&mint_proof);
    let mint_proof_signature = resource_accoount
        .privkey
        .sign_arbitrary_message(&mint_proof_msg.unwrap());

    // call mint_nft function with the user's mint proof signature and public key
    assert_success!(h.run_entry_function(
        &nft_receiver,
        str::parse(&format!("0x{}::minting::mint_nft", resource_address)).unwrap(),
        vec![],
        // pass in resource account's signature
        vec![bcs::to_bytes::<Ed25519Signature>(&mint_proof_signature).unwrap(),],
    ));

    // construct a token_id to check if the nft mint is successful / if the token id exists in this nft receiver's token store
    let token_id = TokenId {
        token_data_id: TokenDataId {
            creator: resource_address,
            collection: String::from("Collection name").into_bytes(),
            name: String::from("Token name").into_bytes(),
        },
        property_version: 1,
    };

    // get the token store of the nft receiver
    let token_store = h
        .read_resource::<TokenStore>(
            nft_receiver.address(),
            parse_struct_tag("0x3::token::TokenStore").unwrap(),
        )
        .unwrap();
    let token_store_table = token_store.tokens;

    // assert that the token id exists in the nft receiver's token store
    // read_state_value() will only be successful if the nft receiver's token store has this token id
    let state_key = &StateKey::table_item(token_store_table, bcs::to_bytes(&token_id).unwrap());
    h.read_state_value(state_key).unwrap();
}
