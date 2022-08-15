// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context;
use aptos_api_test_context::current_function_name;
use aptos_api_types::{new_vm_utf8_string, AsConverter, MoveConverter, MoveType};
use aptos_vm::{data_cache::AsMoveResolver, move_vm_ext::MoveResolverExt};
use move_deps::move_core_types::{
    account_address::AccountAddress,
    value::{MoveStruct, MoveValue as VmMoveValue},
};
use serde::Serialize;
use serde_json::json;
use std::convert::TryInto;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_value_conversion() {
    let context = new_test_context(current_function_name!());
    let address = AccountAddress::from_hex_literal("0x1").unwrap();

    let state_view = context.latest_state_view();
    let resolver = state_view.as_move_resolver();
    let converter = resolver.as_converter(context.db);

    assert_value_conversion(&converter, "u8", 1i32, VmMoveValue::U8(1));
    assert_value_conversion(&converter, "u64", "1", VmMoveValue::U64(1));
    assert_value_conversion(&converter, "u128", "1", VmMoveValue::U128(1));
    assert_value_conversion(&converter, "bool", true, VmMoveValue::Bool(true));
    assert_value_conversion(&converter, "address", "0x1", VmMoveValue::Address(address));
    assert_value_conversion(
        &converter,
        "0x1::string::String",
        "hello",
        new_vm_utf8_string("hello"),
    );
    assert_value_conversion(
        &converter,
        "vector<u8>",
        "0x0102",
        VmMoveValue::Vector(vec![VmMoveValue::U8(1), VmMoveValue::U8(2)]),
    );
    assert_value_conversion(
        &converter,
        "vector<u64>",
        ["1", "2"],
        VmMoveValue::Vector(vec![VmMoveValue::U64(1), VmMoveValue::U64(2)]),
    );
    assert_value_conversion(
        &converter,
        "0x1::guid::ID",
        json!({"addr": "0x1", "creation_num": "1"}),
        VmMoveValue::Struct(MoveStruct::Runtime(vec![
            VmMoveValue::U64(1),
            VmMoveValue::Address(address),
        ])),
    );
}

fn assert_value_conversion<'r, R: MoveResolverExt, V: Serialize>(
    converter: &MoveConverter<'r, R>,
    json_move_type: &str,
    json_value: V,
    expected_vm_value: VmMoveValue,
) {
    let move_type: MoveType = serde_json::from_value(json!(json_move_type)).unwrap();
    let type_tag = move_type.try_into().unwrap();
    let vm_value = converter
        .try_into_vm_value(&type_tag, json!(json_value))
        .unwrap();
    assert_eq!(vm_value, expected_vm_value);

    let vm_bytes = vm_value.undecorate().simple_serialize().unwrap();
    let move_value_back = converter.try_into_move_value(&type_tag, &vm_bytes).unwrap();
    let json_value_back = serde_json::to_value(move_value_back).unwrap();
    assert_eq!(json_value_back, json!(json_value));
}
