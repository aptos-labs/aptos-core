// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_crypto::SigningKey;
use aptos_types::{account_address::AccountAddress, event::EventHandle, state_store::state_key::StateKey};
use serde::{Deserialize, Serialize};
use aptos_crypto::ed25519::{Ed25519PublicKey, Ed25519Signature};
use aptos_types::state_store::table::TableHandle;
use move_deps::move_core_types::parser::parse_struct_tag;

#[derive(Deserialize, Serialize)]
struct ModuleData {
    resource_account_address: AccountAddress,
}

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
    sequence_number: u64,
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
    // publish the smart contract @ 0xcafe
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("../../../move-examples/mint_nft"),
    ));

    // getting the module data from the module publisher's (0xcafe) address
    let resource = h.read_resource::<ModuleData>(acc.address(), parse_struct_tag("0xcafe::minting::ModuleData").unwrap()).unwrap();

    let nft_receiver = h.new_account_with_key_pair();

    // construct the token_data_id and mint_proof, which are required to mint the nft
    let token_data_id = TokenDataId {
        creator: resource.resource_account_address,
        collection: String::from("Collection name").into_bytes(),
        name: String::from("Token name").into_bytes(),
    };

    let mint_proof = MintProofChallenge {
        account_address:  AccountAddress::from_hex_literal("0xcafe").unwrap(),
        module_name: String::from("minting"),
        struct_name: String::from("MintProofChallenge"),
        sequence_number: 10,
        token_data_id,
    };

    // sign the mint proof challenge, which indicates that the user intends to claim this tokens
    let mint_proof_msg = bcs::to_bytes(&mint_proof);
    let mint_proof_signature = nft_receiver.privkey.sign_arbitrary_message(&mint_proof_msg.unwrap());

    // call mint_nft function with the user's mint proof signature and public key
    assert_success!(h.run_entry_function(
        &nft_receiver,
        str::parse("0xcafe::minting::mint_nft").unwrap(),
        vec![],
        vec![bcs::to_bytes::<Ed25519Signature>(&mint_proof_signature).unwrap(), bcs::to_bytes::<Ed25519PublicKey>(&nft_receiver.pubkey).unwrap()],
    ));

    // construct a token_id to check if the nft mint is successful (if the token id exists in this nft receiver's token store)
    let token_id = TokenId {
        token_data_id: TokenDataId {
            creator: resource.resource_account_address,
            collection: String::from("Collection name").into_bytes(),
            name: String::from("Token name").into_bytes(),
        },
        property_version: 1,
    };

    // get the token store of the nft receiver
    let token_store = h.read_resource::<TokenStore>(
        &nft_receiver.address(),
        parse_struct_tag("0x3::token::TokenStore").unwrap(),
    ).unwrap();
    let token_store_table = token_store.tokens;

    // assert that the token id exists in the nft receiver's token store
    let state_key = &StateKey::table_item(
        token_store_table,
        bcs::to_bytes(&token_id).unwrap(),
    );
    // read_state_value() will only be successful if the nft receiver's token store has this token id
    h.read_state_value(state_key).unwrap();
}
