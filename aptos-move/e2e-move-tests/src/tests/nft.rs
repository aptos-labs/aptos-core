// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, MoveHarness};
use aptos_types::account_address::AccountAddress;
use cached_packages::aptos_token_sdk_builder;

#[test]
fn test_nft_end_to_end() {
    let mut harness = MoveHarness::new();
    let creator = harness.new_account_at(AccountAddress::from_hex_literal("0x11").unwrap());
    let owner = harness.new_account_at(AccountAddress::from_hex_literal("0x21").unwrap());

    let collection_name = "collection name".to_owned().into_bytes();
    let token_name = "token name".to_owned().into_bytes();
    assert_success!(harness.run_transaction_payload(
        &creator,
        aptos_token_sdk_builder::token_create_collection_script(
            collection_name.clone(),
            "description".to_owned().into_bytes(),
            "uri".to_owned().into_bytes(),
            20_000_000,
            vec![false, false, false],
        )
    ));
    assert_success!(harness.run_transaction_payload(
        &creator,
        aptos_token_sdk_builder::token_create_token_script(
            collection_name.clone(),
            token_name.clone(),
            "collection description".to_owned().into_bytes(),
            1,
            4,
            "uri".to_owned().into_bytes(),
            *creator.address(),
            0,
            0,
            vec![false, false, false, false, true],
            vec!["age".as_bytes().to_vec()],
            vec!["3".as_bytes().to_vec()],
            vec!["int".as_bytes().to_vec()],
        )
    ));
    assert_success!(harness.run_transaction_payload(
        &creator,
        aptos_token_sdk_builder::token_mint_script(
            *creator.address(),
            collection_name.clone(),
            token_name.clone(),
            1,
        )
    ));
    assert_success!(harness.run_transaction_payload(
        &creator,
        aptos_token_sdk_builder::token_mutate_token_properties(
            *creator.address(),
            *creator.address(),
            collection_name.clone(),
            token_name.clone(),
            0,
            1,
            vec![],
            vec![],
            vec![]
        )
    ));
}
