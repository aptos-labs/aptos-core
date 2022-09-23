// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use aptos_crypto::SigningKey;
use aptos_types::account_address::AccountAddress;
use e2e_move_tests::{MoveHarness};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
struct TokenDataId {
    creator: AccountAddress,
    collection: Vec<u8>,
    name: Vec<u8>,
}

#[derive(Deserialize, Serialize, Debug)]
struct MintProofChallenge {
    account_address: AccountAddress,
    module_name: String,
    struct_name: String,
    sequence_number: u64,
    token_data_id: TokenDataId,
}

// example to create a valid signature for minting an NFT
#[test]
fn mint_nft() {
    let mut harness = MoveHarness::new();
    let account = harness.new_account_at(AccountAddress::from_hex_literal("0x234").unwrap());

    let token_data_id = TokenDataId {
        creator: AccountAddress::from_hex_literal("0x0b6beee9bc1ad3177403a04efeefb1901c12b7b575ac5124c0205efc0dd2e32a").unwrap(),
        collection: String::from("Collection name").into_bytes(),
        name: String::from("Token name").into_bytes(),
    };

    let mint_proof = MintProofChallenge {
        account_address:  AccountAddress::from_hex_literal("0xcafe").unwrap(),
        module_name: String::from("minting"),
        struct_name: String::from("MintProofChallenge"),
        sequence_number: 0,
        token_data_id,
    };

    let mint_proof_msg = bcs::to_bytes(&mint_proof);

    println!("{:?}", account.pubkey);
    println!("{:?}", account.privkey.sign_arbitrary_message(&mint_proof_msg.unwrap()));
}