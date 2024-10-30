// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{assert_success, tests::common, MoveHarness};
use aptos_language_e2e_tests::account::Account;
use aptos_types::{
    account_address::{self, AccountAddress},
    account_config::ObjectCoreResource,
    event::EventHandle,
    move_utils::MemberId,
    transaction::{EntryFunction, TransactionPayload},
};
use move_core_types::{identifier::Identifier, language_storage::StructTag};
use rstest::rstest;
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

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_basic_token(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);

    let account = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });

    publish_object_token_example(&mut h, *account.address(), &account);

    let result = h.run_transaction_payload(
        &account,
        create_mint_hero_payload(account.address(), "The best hero ever!"),
    );
    assert_success!(result);

    let token_addr =
        account_address::create_token_address(*account.address(), "Hero Quest!", "Wukong");
    let obj_tag = StructTag {
        address: AccountAddress::from_hex_literal("0x1").unwrap(),
        module: Identifier::new("object").unwrap(),
        name: Identifier::new("ObjectCore").unwrap(),
        type_args: vec![],
    };
    let token_obj_tag = StructTag {
        address: AccountAddress::from_hex_literal("0x4").unwrap(),
        module: Identifier::new("token").unwrap(),
        name: Identifier::new("Token").unwrap(),
        type_args: vec![],
    };
    let obj_group_tag = StructTag {
        address: AccountAddress::from_hex_literal("0x1").unwrap(),
        module: Identifier::new("object").unwrap(),
        name: Identifier::new("ObjectGroup").unwrap(),
        type_args: vec![],
    };

    // Ensure that the group data can be read
    let object_0: ObjectCoreResource = h
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

    let result = h.run_transaction_payload(
        &account,
        create_set_hero_description_payload(account.address(), "Oh no!"),
    );
    assert_success!(result);

    // verify all the data remains in a group even when updating just a single resource
    let object_1: ObjectCoreResource = h
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

pub fn publish_object_token_example(h: &mut MoveHarness, addr: AccountAddress, account: &Account) {
    let mut build_options = aptos_framework::BuildOptions::default();
    build_options
        .named_addresses
        .insert("hero".to_string(), addr);

    let result = h.publish_package_with_options(
        account,
        &common::test_dir_path("../../../move-examples/token_objects/hero"),
        build_options,
    );
    assert_success!(result);
}

pub fn create_mint_hero_payload(addr: &AccountAddress, description: &str) -> TransactionPayload {
    let fun = str::parse(&format!("0x{}::hero::mint_hero", addr.to_hex())).unwrap();
    let MemberId {
        module_id,
        member_id: function_id,
    } = fun;

    TransactionPayload::EntryFunction(EntryFunction::new(module_id, function_id, vec![], vec![
        bcs::to_bytes(description).unwrap(),
        bcs::to_bytes("Male").unwrap(),
        bcs::to_bytes("Wukong").unwrap(),
        bcs::to_bytes("Monkey God").unwrap(),
        bcs::to_bytes("404").unwrap(),
    ]))
}

pub fn create_set_hero_description_payload(
    addr: &AccountAddress,
    description: &str,
) -> TransactionPayload {
    let fun = str::parse(&format!("0x{}::hero::set_hero_description", addr.to_hex())).unwrap();
    let MemberId {
        module_id,
        member_id: function_id,
    } = fun;

    TransactionPayload::EntryFunction(EntryFunction::new(module_id, function_id, vec![], vec![
        bcs::to_bytes("Hero Quest!").unwrap(),
        bcs::to_bytes("Wukong").unwrap(),
        bcs::to_bytes(description).unwrap(),
    ]))
}
