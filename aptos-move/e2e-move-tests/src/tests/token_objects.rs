// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_types::{
    account_address::{self, AccountAddress},
    event::EventHandle,
};
use move_core_types::{identifier::Identifier, language_storage::StructTag};
use serde::Deserialize;

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
struct ObjectCore {
    guid_creation_num: u64,
    owner: AccountAddress,
    allow_ungated_transfer: bool,
    transfer_events: EventHandle,
}

#[test]
fn test_basic_token() {
    let mut h = MoveHarness::new();

    let addr = AccountAddress::from_hex_literal("0xcafe").unwrap();
    let account = h.new_account_at(addr);

    let mut build_options = aptos_framework::BuildOptions::default();
    build_options
        .named_addresses
        .insert("hero".to_string(), addr);

    let result = h.publish_package_with_options(
        &account,
        &common::test_dir_path("../../../move-examples/token_objects/hero"),
        build_options,
    );
    assert_success!(result);

    let result = h.run_entry_function(
        &account,
        str::parse(&format!("0x{}::hero::mint_hero", addr.to_hex())).unwrap(),
        vec![],
        vec![
            bcs::to_bytes("The best hero ever!").unwrap(),
            bcs::to_bytes("Male").unwrap(),
            bcs::to_bytes("Wukong").unwrap(),
            bcs::to_bytes("Monkey God").unwrap(),
            bcs::to_bytes("404").unwrap(),
        ],
    );
    assert_success!(result);

    let token_addr = account_address::create_token_address(addr, "Hero Quest!", "Wukong");
    let obj_tag = StructTag {
        address: AccountAddress::from_hex_literal("0x1").unwrap(),
        module: Identifier::new("object").unwrap(),
        name: Identifier::new("ObjectCore").unwrap(),
        type_params: vec![],
    };
    let token_obj_tag = StructTag {
        address: AccountAddress::from_hex_literal("0x4").unwrap(),
        module: Identifier::new("token").unwrap(),
        name: Identifier::new("Token").unwrap(),
        type_params: vec![],
    };
    let obj_group_tag = StructTag {
        address: AccountAddress::from_hex_literal("0x1").unwrap(),
        module: Identifier::new("object").unwrap(),
        name: Identifier::new("ObjectGroup").unwrap(),
        type_params: vec![],
    };

    // Ensure that the group data can be read
    let object_0: ObjectCore = h
        .read_resource_from_resource_group(&token_addr, obj_group_tag.clone(), obj_tag.clone())
        .unwrap();
    let token_0: Token = h
        .read_resource_from_resource_group(
            &token_addr,
            obj_group_tag.clone(),
            token_obj_tag.clone(),
        )
        .unwrap();
    // Ensure that the original resources cannot be read
    assert!(h.read_resource_raw(&token_addr, obj_tag.clone()).is_none());
    assert!(h
        .read_resource_raw(&token_addr, token_obj_tag.clone())
        .is_none());

    let result = h.run_entry_function(
        &account,
        str::parse(&format!("0x{}::hero::set_hero_description", addr.to_hex())).unwrap(),
        vec![],
        vec![
            bcs::to_bytes("Hero Quest!").unwrap(),
            bcs::to_bytes("Wukong").unwrap(),
            bcs::to_bytes("Oh no!").unwrap(),
        ],
    );
    assert_success!(result);

    // verify all the data remains in a group even when updating just a single resource
    let object_1: ObjectCore = h
        .read_resource_from_resource_group(&token_addr, obj_group_tag.clone(), obj_tag)
        .unwrap();
    let mut token_1: Token = h
        .read_resource_from_resource_group(&token_addr, obj_group_tag, token_obj_tag)
        .unwrap();
    assert_eq!(object_0, object_1);
    assert_ne!(token_0, token_1);
    // Determine that the only difference is the mutated description
    assert_eq!(token_1.description, "Oh no!");
    token_1.description = "The best hero ever!".to_string();
    assert_eq!(token_0.mutation_events.key(), token_1.mutation_events.key());
}
