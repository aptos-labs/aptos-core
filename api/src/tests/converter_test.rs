// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{current_function_name, tests::new_test_context};
use aptos_api_types::{AsConverter, MoveConverter, MoveType};
use aptos_vm::data_cache::AsMoveResolver;
use move_core_types::{
    account_address::AccountAddress,
    resolver::MoveResolver,
    value::{MoveStruct, MoveValue as VmMoveValue},
};
use serde::Serialize;
use serde_json::json;
use std::convert::TryInto;

#[tokio::test]
async fn test_parse_move_value() {
    let context = new_test_context(current_function_name!());
    let address = AccountAddress::from_hex_literal("0x1").unwrap();

    let state_view = context.latest_state_view();
    let resolver = state_view.as_move_resolver();
    let converter = resolver.as_converter();

    assert_parse_move_value(&converter, "u8", 1i32, VmMoveValue::U8(1));
    assert_parse_move_value(&converter, "u64", "1", VmMoveValue::U64(1));
    assert_parse_move_value(&converter, "u128", "1", VmMoveValue::U128(1));
    assert_parse_move_value(&converter, "bool", true, VmMoveValue::Bool(true));
    assert_parse_move_value(&converter, "address", "0x1", VmMoveValue::Address(address));
    assert_parse_move_value(
        &converter,
        "vector<u8>",
        "0x0102",
        VmMoveValue::Vector(vec![VmMoveValue::U8(1), VmMoveValue::U8(2)]),
    );
    assert_parse_move_value(
        &converter,
        "vector<u64>",
        ["1", "2"],
        VmMoveValue::Vector(vec![VmMoveValue::U64(1), VmMoveValue::U64(2)]),
    );
    assert_parse_move_value(
        &converter,
        "0x1::GUID::ID",
        json!({"addr": "0x1", "creation_num": "1"}),
        VmMoveValue::Struct(MoveStruct::Runtime(vec![
            VmMoveValue::U64(1),
            VmMoveValue::Address(address),
        ])),
    );
}

fn assert_parse_move_value<'r, R: MoveResolver, V: Serialize>(
    converter: &MoveConverter<'r, R>,
    json_move_type: &str,
    json_value: V,
    expected_move_value: VmMoveValue,
) {
    let move_type: MoveType = serde_json::from_value(json!(json_move_type)).unwrap();
    let type_tag = move_type.try_into().unwrap();
    let move_value = converter
        .try_into_vm_value(&type_tag, json!(json_value))
        .unwrap();
    assert_eq!(move_value, expected_move_value);
}
