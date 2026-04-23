// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::MoveHarness;
use aptos_types::{account_address::AccountAddress, move_utils::MemberId};
use move_core_types::{
    ability::AbilitySet,
    identifier::Identifier,
    language_storage::{FunctionTag, ModuleId, StructTag, TypeTag},
};
use std::str::FromStr;

#[test]
fn test_txn_payload_validation_discards_bad_inputs() {
    let mut h = MoveHarness::new();

    // Use a fresh account for each (kept, discarded) pair because discarded
    // transactions do not consume the on-chain sequence number, but the
    // harness still increments its internal counter, causing subsequent
    // transactions to fail with SEQUENCE_NUMBER_TOO_NEW.

    let account = h.new_account_at(AccountAddress::from_hex_literal("0x100").unwrap());
    let status = h.run_entry_function(
        &account,
        MemberId::from_str("0x1::a::b").unwrap(),
        vec![],
        vec![],
    );
    assert!(status.is_kept());

    // Entry function module name length.
    let account = h.new_account_at(AccountAddress::from_hex_literal("0x101").unwrap());
    let status = h.run_entry_function(
        &account,
        MemberId::from_str(&format!("0x1::{}::b", "a".repeat(128))).unwrap(),
        vec![],
        vec![],
    );
    assert!(status.is_kept());
    let status = h.run_entry_function(
        &account,
        MemberId::from_str(&format!("0x1::{}::b", "a".repeat(129))).unwrap(),
        vec![],
        vec![],
    );
    assert!(status.is_discarded());

    // Entry function name length.
    let account = h.new_account_at(AccountAddress::from_hex_literal("0x102").unwrap());
    let status = h.run_entry_function(
        &account,
        MemberId::from_str(&format!("0x1::a::{}", "b".repeat(128))).unwrap(),
        vec![],
        vec![],
    );
    assert!(status.is_kept());
    let status = h.run_entry_function(
        &account,
        MemberId::from_str(&format!("0x1::a::{}", "b".repeat(129))).unwrap(),
        vec![],
        vec![],
    );
    assert!(status.is_discarded());

    // Type tag identifier length.
    let account = h.new_account_at(AccountAddress::from_hex_literal("0x103").unwrap());
    let status = h.run_entry_function(
        &account,
        MemberId::from_str("0x1::a::b").unwrap(),
        vec![TypeTag::from_str(&format!("0x1::{}::B", "a".repeat(128))).unwrap()],
        vec![],
    );
    assert!(status.is_kept());
    let status = h.run_entry_function(
        &account,
        MemberId::from_str("0x1::a::b").unwrap(),
        vec![TypeTag::from_str(&format!("0x1::{}::B", "a".repeat(129))).unwrap()],
        vec![],
    );
    assert!(status.is_discarded());

    let account = h.new_account_at(AccountAddress::from_hex_literal("0x104").unwrap());
    let status = h.run_entry_function(
        &account,
        MemberId::from_str("0x1::a::b").unwrap(),
        vec![TypeTag::from_str(&format!("0x1::a::{}", "B".repeat(128))).unwrap()],
        vec![],
    );
    assert!(status.is_kept());
    let status = h.run_entry_function(
        &account,
        MemberId::from_str("0x1::a::b").unwrap(),
        vec![TypeTag::from_str(&format!("0x1::a::{}", "B".repeat(129))).unwrap()],
        vec![],
    );
    assert!(status.is_discarded());

    // Type arguments count.
    let account = h.new_account_at(AccountAddress::from_hex_literal("0x105").unwrap());
    let status = h.run_entry_function(
        &account,
        MemberId::from_str("0x1::a::b").unwrap(),
        vec![TypeTag::U8; 32],
        vec![],
    );
    assert!(status.is_kept());
    let status = h.run_entry_function(
        &account,
        MemberId::from_str("0x1::a::b").unwrap(),
        vec![TypeTag::U8; 33],
        vec![],
    );
    assert!(status.is_discarded());

    // Type tag node count.
    let struct_with_u8_args = |n: usize| {
        TypeTag::Struct(Box::new(StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new_unchecked("a"),
            name: Identifier::new_unchecked("B"),
            type_args: vec![TypeTag::U8; n],
        }))
    };
    let account = h.new_account_at(AccountAddress::from_hex_literal("0x106").unwrap());
    let status = h.run_entry_function(
        &account,
        MemberId::from_str("0x1::a::b").unwrap(),
        vec![struct_with_u8_args(7)],
        vec![],
    );
    assert!(status.is_kept());
    let status = h.run_entry_function(
        &account,
        MemberId::from_str("0x1::a::b").unwrap(),
        vec![struct_with_u8_args(8)],
        vec![],
    );
    assert!(status.is_discarded());

    // Function type tags are rejected.
    let account = h.new_account_at(AccountAddress::from_hex_literal("0x107").unwrap());
    let tag = TypeTag::Function(Box::new(FunctionTag {
        args: vec![],
        results: vec![],
        abilities: AbilitySet::EMPTY,
    }));
    let status = h.run_entry_function(
        &account,
        MemberId::from_str("0x1::a::b").unwrap(),
        vec![tag],
        vec![],
    );
    assert!(status.is_discarded());

    // Invalid identifier charset in module name.
    let account = h.new_account_at(AccountAddress::from_hex_literal("0x108").unwrap());
    let status = h.run_entry_function(
        &account,
        MemberId {
            module_id: ModuleId::new(AccountAddress::ONE, Identifier::new_unchecked("bad name")),
            member_id: Identifier::new_unchecked("b"),
        },
        vec![],
        vec![],
    );
    assert!(status.is_discarded());

    // Invalid identifier charset in function name.
    let account = h.new_account_at(AccountAddress::from_hex_literal("0x109").unwrap());
    let status = h.run_entry_function(
        &account,
        MemberId {
            module_id: ModuleId::new(AccountAddress::ONE, Identifier::new_unchecked("a")),
            member_id: Identifier::new_unchecked("bad name"),
        },
        vec![],
        vec![],
    );
    assert!(status.is_discarded());

    // Invalid identifier charset inside a type tag.
    let account = h.new_account_at(AccountAddress::from_hex_literal("0x10a").unwrap());
    let bad_struct = TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ONE,
        module: Identifier::new_unchecked("bad name"),
        name: Identifier::new_unchecked("B"),
        type_args: vec![],
    }));
    let status = h.run_entry_function(
        &account,
        MemberId::from_str("0x1::a::b").unwrap(),
        vec![bad_struct],
        vec![],
    );
    assert!(status.is_discarded());
}
