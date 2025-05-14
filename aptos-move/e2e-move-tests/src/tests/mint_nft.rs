// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519Signature},
    SigningKey, ValidCryptoMaterialStringExt,
};
use aptos_framework::BuiltPackage;
use aptos_types::{
    account_address::{create_resource_address, AccountAddress},
    event::EventHandle,
    state_store::{state_key::StateKey, table::TableHandle},
};
use move_core_types::parser::parse_struct_tag;
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
    let resource_address = create_resource_address(*acc.address(), &[]);

    // give a named address to the `mint_nft` module publisher
    let mut build_options = aptos_framework::BuildOptions::default();
    build_options
        .named_addresses
        .insert("mint_nft".to_string(), resource_address);
    build_options
        .named_addresses
        .insert("source_addr".to_string(), *acc.address());

    // build the package from our example code
    let package = BuiltPackage::build(
        common::test_dir_path("../../../move-examples/mint_nft/4-Getting-Production-Ready"),
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

    // construct the token_data_id and mint_proof, which are required to mint the nft
    let token_data_id = TokenDataId {
        creator: resource_address,
        collection: String::from("Collection name").into_bytes(),
        name: String::from("Token name").into_bytes(),
    };

    let nft_receiver = h.new_account_with_key_pair();
    let mint_proof = MintProofChallenge {
        account_address: resource_address,
        module_name: String::from("create_nft_getting_production_ready"),
        struct_name: String::from("MintProofChallenge"),
        receiver_account_sequence_number: 0,
        receiver_account_address: *nft_receiver.address(),
        token_data_id,
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
            resource_address.to_hex()
        ))
        .unwrap(),
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
    let state_key = &StateKey::table_item(&token_store_table, &bcs::to_bytes(&token_id).unwrap());
    h.read_state_value_bytes(state_key).unwrap();
}

/// samples two signatures for unit tests in move-examples/
/// 4-Getting-Production-Ready/sources/create_nft_getting_production_ready.move
#[test]
fn sample_mint_nft_part4_unit_test_signature() {
    let mut h = MoveHarness::new();

    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let resource_address = create_resource_address(*acc.address(), &[]);

    let resource_account = h.new_account_at(resource_address);
    let nft_receiver1 = h.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    let nft_receiver2 = h.new_account_at(AccountAddress::from_hex_literal("0x234").unwrap());

    // construct the token_data_id and mint_proof, which are required to mint the nft
    let token_data_id1 = TokenDataId {
        creator: resource_address,
        collection: String::from("Collection name").into_bytes(),
        name: String::from("Token name").into_bytes(),
    };

    let token_data_id2 = TokenDataId {
        creator: resource_address,
        collection: String::from("Collection name").into_bytes(),
        name: String::from("Token name").into_bytes(),
    };

    let mint_proof1 = MintProofChallenge {
        account_address: resource_address,
        module_name: String::from("create_nft_getting_production_ready"),
        struct_name: String::from("MintProofChallenge"),
        receiver_account_sequence_number: 0,
        receiver_account_address: *nft_receiver1.address(),
        token_data_id: token_data_id1,
    };

    let mint_proof2 = MintProofChallenge {
        account_address: resource_address,
        module_name: String::from("create_nft_getting_production_ready"),
        struct_name: String::from("MintProofChallenge"),
        receiver_account_sequence_number: 0,
        receiver_account_address: *nft_receiver2.address(),
        token_data_id: token_data_id2,
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

/// Run `cargo test generate_nft_tutorial_part4_signature -- --nocapture`
/// to generate a valid signature for `[resource_account_address]::create_nft_getting_production_ready::mint_event_pass()` function
/// in `aptos-move/move-examples/mint_nft/4-Getting-Production-Ready/sources/create_nft_getting_production_ready.move`. åååååååå
#[test]
fn generate_nft_tutorial_part4_signature() {
    let mut h = MoveHarness::new();

    // When running this test to generate a valid signature, supply the actual resource_address to line 217.
    // Uncomment line 223 and comment out line 224 (it's just a placeholder).
    // let resource_address = h.new_account_at(AccountAddress::from_hex_literal("0x[resource account's address]").unwrap());
    let resource_address = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // When running this test to generate a valid signature, supply the actual nft_receiver's address to line 222.
    // Uncomment line 228 and comment out line 229.
    // let nft_receiver = h.new_account_at(AccountAddress::from_hex_literal("0x[nft-receiver's address]").unwrap());
    let nft_receiver = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // When running this test to generate a valid signature, supply the actual private key to replace the (0000...) in line 232.
    let admin_private_key = Ed25519PrivateKey::from_encoded_string(
        "0000000000000000000000000000000000000000000000000000000000000000",
    )
    .unwrap();

    // construct the token_data_id and mint_proof, which are required to mint the nft
    let token_data_id = TokenDataId {
        creator: *resource_address.address(),
        collection: String::from("Collection name").into_bytes(),
        name: String::from("Token name").into_bytes(),
    };

    let mint_proof = MintProofChallenge {
        account_address: *resource_address.address(),
        module_name: String::from("create_nft_getting_production_ready"),
        struct_name: String::from("MintProofChallenge"),
        // change the `receiver_account_sequence_number` to the right sequence number
        // you can find an account's sequence number by searching for the account's address on explorer.aptoslabs.com and going to the `Info` tab
        receiver_account_sequence_number: 0,
        receiver_account_address: *nft_receiver.address(),
        token_data_id,
    };

    // sign the MintProofChallenge using the resource account's private key
    let mint_proof_msg = bcs::to_bytes(&mint_proof);

    let mint_proof_signature = admin_private_key.sign_arbitrary_message(&mint_proof_msg.unwrap());
    println!(
        "Mint Proof Signature for NFT receiver: {:?}",
        mint_proof_signature
    );
}
