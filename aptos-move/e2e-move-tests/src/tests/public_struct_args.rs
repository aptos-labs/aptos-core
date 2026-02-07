// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for public structs and enums with copy ability as transaction arguments.
//!
//! This module tests the feature that allows public structs/enums with the `copy` ability
//! to be passed as entry function arguments. When compiled with language version 2.4+,
//! pack functions are automatically generated for public structs/enums with copy ability.

use crate::{assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_types::account_address::AccountAddress;
use move_core_types::{
    value::{MoveStruct, MoveValue},
    vm_status::StatusCode,
};
use serde::{Deserialize, Serialize};

/// Mimics `0xcafe::public_struct_test::TestResult`
#[derive(Serialize, Deserialize, Debug)]
struct TestResult {
    value: u64,
    message: Vec<u8>,
}

fn setup_harness() -> MoveHarness {
    MoveHarness::new()
}

fn get_test_result(h: &MoveHarness, addr: &AccountAddress) -> TestResult {
    h.read_resource_raw(
        addr,
        "0xcafe::public_struct_test::TestResult".parse().unwrap(),
    )
    .map(|bytes| bcs::from_bytes(&bytes).unwrap())
    .unwrap()
}

/// Test that the module with public copy structs compiles and publishes
/// with language version 2.4 (which auto-generates pack functions)
#[test]
fn test_module_compiles_with_public_copy_struct_params() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Compile with language 2.4+ to auto-generate pack functions
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/pack"),
        BuildOptions::move_2().set_latest_language(),
    ));
}

/// Test passing a simple public struct (Point) as a transaction argument
#[test]
fn test_public_struct_point() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Compile with language 2.4+ to auto-generate pack functions
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/pack"),
        BuildOptions::move_2().set_latest_language(),
    ));

    // Initialize the test result
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::initialize").unwrap(),
        vec![],
        vec![],
    ));

    // Create a Point struct: Point { x: 10, y: 20 }
    // BCS serialization: serialize the fields in order
    let point_value = MoveValue::Struct(MoveStruct::Runtime(vec![
        MoveValue::U64(10), // x
        MoveValue::U64(20), // y
    ]));

    // Call test_point with the Point argument
    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::test_point").unwrap(),
        vec![],
        vec![point_value.simple_serialize().unwrap()],
    );
    assert_success!(status);

    // Verify the result
    let result = get_test_result(&h, acc.address());
    assert_eq!(result.value, 30); // 10 + 20
    assert_eq!(String::from_utf8(result.message).unwrap(), "point_received");
}

/// Test passing a nested public struct (Rectangle) as a transaction argument
#[test]
fn test_public_struct_nested() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/pack"),
        BuildOptions::move_2().set_latest_language(),
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::initialize").unwrap(),
        vec![],
        vec![],
    ));

    // Create a Rectangle struct with nested Points
    // Rectangle { top_left: Point { x: 1, y: 2 }, bottom_right: Point { x: 3, y: 4 } }
    let top_left = MoveValue::Struct(MoveStruct::Runtime(vec![
        MoveValue::U64(1),
        MoveValue::U64(2),
    ]));
    let bottom_right = MoveValue::Struct(MoveStruct::Runtime(vec![
        MoveValue::U64(3),
        MoveValue::U64(4),
    ]));
    let rectangle_value = MoveValue::Struct(MoveStruct::Runtime(vec![top_left, bottom_right]));

    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::test_rectangle").unwrap(),
        vec![],
        vec![rectangle_value.simple_serialize().unwrap()],
    );
    assert_success!(status);

    let result = get_test_result(&h, acc.address());
    assert_eq!(result.value, 10); // 1 + 2 + 3 + 4
    assert_eq!(
        String::from_utf8(result.message).unwrap(),
        "rectangle_received"
    );
}

/// Test passing a public struct with String field (Data) as a transaction argument
#[test]
fn test_public_struct_with_string() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/pack"),
        BuildOptions::move_2().set_latest_language(),
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::initialize").unwrap(),
        vec![],
        vec![],
    ));

    // Create a Data struct: Data { values: [5, 10, 15], name: "test_data" }
    // String in Move is struct { bytes: vector<u8> }
    let string_value = MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::Vector(
        "test_data"
            .as_bytes()
            .iter()
            .map(|b| MoveValue::U8(*b))
            .collect(),
    )]));
    let data_value = MoveValue::Struct(MoveStruct::Runtime(vec![
        MoveValue::Vector(vec![
            MoveValue::U64(5),
            MoveValue::U64(10),
            MoveValue::U64(15),
        ]),
        string_value,
    ]));

    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::test_data").unwrap(),
        vec![],
        vec![data_value.simple_serialize().unwrap()],
    );
    assert_success!(status);

    let result = get_test_result(&h, acc.address());
    assert_eq!(result.value, 30); // 5 + 10 + 15
    assert_eq!(String::from_utf8(result.message).unwrap(), "test_data");
}

/// Test passing a public enum (Color::Red) as a transaction argument
#[test]
fn test_public_enum_unit_variant() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/pack"),
        BuildOptions::move_2().set_latest_language(),
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::initialize").unwrap(),
        vec![],
        vec![],
    ));

    // Create Color::Red (variant index 0, no fields)
    let color_red = MoveValue::Struct(MoveStruct::RuntimeVariant(0, vec![]));

    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::test_color").unwrap(),
        vec![],
        vec![color_red.simple_serialize().unwrap()],
    );
    assert_success!(status);

    let result = get_test_result(&h, acc.address());
    assert_eq!(result.value, 1);
    assert_eq!(String::from_utf8(result.message).unwrap(), "red");
}

/// Test passing a public enum with fields (Color::Custom) as a transaction argument
#[test]
fn test_public_enum_with_fields() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/pack"),
        BuildOptions::move_2().set_latest_language(),
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::initialize").unwrap(),
        vec![],
        vec![],
    ));

    // Create Color::Custom { r: 100, g: 50, b: 25 } (variant index 3)
    let color_custom = MoveValue::Struct(MoveStruct::RuntimeVariant(3, vec![
        MoveValue::U8(100),
        MoveValue::U8(50),
        MoveValue::U8(25),
    ]));

    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::test_color").unwrap(),
        vec![],
        vec![color_custom.simple_serialize().unwrap()],
    );
    assert_success!(status);

    let result = get_test_result(&h, acc.address());
    assert_eq!(result.value, 175); // 100 + 50 + 25
    assert_eq!(String::from_utf8(result.message).unwrap(), "custom");
}

/// Test passing a public enum with struct fields (Shape::Circle) as a transaction argument
#[test]
fn test_public_enum_with_struct_fields() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/pack"),
        BuildOptions::move_2().set_latest_language(),
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::initialize").unwrap(),
        vec![],
        vec![],
    ));

    // Create Shape::Circle { center: Point { x: 5, y: 10 }, radius: 15 } (variant index 0)
    let center = MoveValue::Struct(MoveStruct::Runtime(vec![
        MoveValue::U64(5),
        MoveValue::U64(10),
    ]));
    let shape_circle = MoveValue::Struct(MoveStruct::RuntimeVariant(0, vec![
        center,
        MoveValue::U64(15),
    ]));

    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::test_shape").unwrap(),
        vec![],
        vec![shape_circle.simple_serialize().unwrap()],
    );
    assert_success!(status);

    let result = get_test_result(&h, acc.address());
    assert_eq!(result.value, 30); // 5 + 10 + 15
    assert_eq!(String::from_utf8(result.message).unwrap(), "circle");
}

/// Test passing a vector of public structs as a transaction argument
#[test]
fn test_vector_of_public_structs() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/pack"),
        BuildOptions::move_2().set_latest_language(),
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::initialize").unwrap(),
        vec![],
        vec![],
    ));

    // Create a vector of Points: [Point { x: 1, y: 2 }, Point { x: 3, y: 4 }, Point { x: 5, y: 6 }]
    let points = MoveValue::Vector(vec![
        MoveValue::Struct(MoveStruct::Runtime(vec![
            MoveValue::U64(1),
            MoveValue::U64(2),
        ])),
        MoveValue::Struct(MoveStruct::Runtime(vec![
            MoveValue::U64(3),
            MoveValue::U64(4),
        ])),
        MoveValue::Struct(MoveStruct::Runtime(vec![
            MoveValue::U64(5),
            MoveValue::U64(6),
        ])),
    ]);

    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::test_point_vector").unwrap(),
        vec![],
        vec![points.simple_serialize().unwrap()],
    );
    assert_success!(status);

    let result = get_test_result(&h, acc.address());
    assert_eq!(result.value, 21); // (1+2) + (3+4) + (5+6)
    assert_eq!(
        String::from_utf8(result.message).unwrap(),
        "point_vector_received"
    );
}

/// Test that whitelisted String type continues to work
#[test]
fn test_whitelisted_string_works() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/pack"),
        BuildOptions::move_2().set_latest_language(),
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::initialize").unwrap(),
        vec![],
        vec![],
    ));

    // Create a String value: String { bytes: vector<u8> }
    let string_value = MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::Vector(
        "hello_world"
            .as_bytes()
            .iter()
            .map(|b| MoveValue::U8(*b))
            .collect(),
    )]));

    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::test_string").unwrap(),
        vec![],
        vec![string_value.simple_serialize().unwrap()],
    );
    assert_success!(status);

    let result = get_test_result(&h, acc.address());
    assert_eq!(result.value, 11); // length of "hello_world"
    assert_eq!(String::from_utf8(result.message).unwrap(), "hello_world");
}

// ========================================================================================
// Negative Tests
// ========================================================================================

/// Test that private struct as entry function parameter is rejected.
/// Private structs don't get pack functions generated, so they cannot be used as txn args.
#[test]
fn test_private_struct_rejected() {
    let _h = setup_harness();

    // Try to compile a package with private struct as entry function parameter.
    // This should fail during compilation because no pack function is generated for private structs,
    // making them invalid as entry function parameters.
    let result = BuiltPackage::build(
        common::test_dir_path("public_struct_args.data/negative_private"),
        BuildOptions::move_2().set_latest_language(),
    );

    // The compilation should fail with an error about invalid entry function parameter
    assert!(
        result.is_err(),
        "Expected private struct as entry function parameter to be rejected during compilation"
    );
}

/// Test that non-copy struct as entry function parameter is rejected.
/// Structs without copy ability don't get pack functions generated.
#[test]
fn test_no_copy_struct_rejected() {
    let _h = setup_harness();

    // Try to compile a package with non-copy struct as entry function parameter.
    // This should fail during compilation because no pack function is generated for non-copy structs,
    // making them invalid as entry function parameters.
    let result = BuiltPackage::build(
        common::test_dir_path("public_struct_args.data/negative_nocopy"),
        BuildOptions::move_2().set_latest_language(),
    );

    // The compilation should fail with an error about invalid entry function parameter
    assert!(
        result.is_err(),
        "Expected non-copy struct as entry function parameter to be rejected during compilation"
    );
}

/// Test that generic container with private type argument is rejected at runtime.
/// Container<T> is public with copy, but when T=PrivatePoint (private struct),
/// the transaction should be rejected during validation.
#[test]
fn test_generic_container_with_private_type_arg_rejected() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // First compile and publish the package - this should succeed because the generic function
    // signature is valid (T just needs copy + drop trait bounds)
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/negative_generic_private"),
        BuildOptions::move_2().set_latest_language(),
    ));

    // Try to call test_generic_container<PrivatePoint> with Container<PrivatePoint>
    // This should FAIL during transaction validation because PrivatePoint is not public

    // Create Container<PrivatePoint> { value: PrivatePoint { x: 10, y: 20 } }
    let private_point = MoveValue::Struct(MoveStruct::Runtime(vec![
        MoveValue::U64(10), // x
        MoveValue::U64(20), // y
    ]));
    let container_value = MoveValue::Struct(MoveStruct::Runtime(vec![private_point]));

    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::negative_generic_private::test_generic_container").unwrap(),
        vec![str::parse("0xcafe::negative_generic_private::PrivatePoint").unwrap()], // Type argument
        vec![container_value.simple_serialize().unwrap()],
    );

    // The transaction should fail during validation because PrivatePoint is not a valid txn arg
    // Expected error: INVALID_MAIN_FUNCTION_SIGNATURE because the struct doesn't have a pack function
    assert_vm_status!(status, StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE);
}

// ========================================================================================
// Option Tests
// ========================================================================================

/// Test passing Option<Point> with Some value
#[test]
fn test_option_some_struct() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/pack"),
        BuildOptions::move_2().set_latest_language(),
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::initialize").unwrap(),
        vec![],
        vec![],
    ));

    // Create Option<Point>::Some(Point { x: 10, y: 20 })
    // Option uses vector-based BCS representation: Some(x) = vector with one element
    let point = MoveValue::Struct(MoveStruct::Runtime(vec![
        MoveValue::U64(10),
        MoveValue::U64(20),
    ]));
    let some_point = MoveValue::Vector(vec![point]);

    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::test_option_point").unwrap(),
        vec![],
        vec![some_point.simple_serialize().unwrap()],
    );
    assert_success!(status);

    let result = get_test_result(&h, acc.address());
    assert_eq!(result.value, 30); // 10 + 20
    assert_eq!(String::from_utf8(result.message).unwrap(), "some_point");
}

/// Test passing Option<Point> with None value
#[test]
fn test_option_none_struct() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/pack"),
        BuildOptions::move_2().set_latest_language(),
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::initialize").unwrap(),
        vec![],
        vec![],
    ));

    // Create Option<Point>::None
    // Option uses vector-based BCS representation: None = empty vector
    let none_point = MoveValue::Vector(vec![]);

    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::test_option_point").unwrap(),
        vec![],
        vec![none_point.simple_serialize().unwrap()],
    );
    assert_success!(status);

    let result = get_test_result(&h, acc.address());
    assert_eq!(result.value, 0);
    assert_eq!(String::from_utf8(result.message).unwrap(), "none_point");
}

/// Test passing Option<Color> with Some(Red)
#[test]
fn test_option_some_enum() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/pack"),
        BuildOptions::move_2().set_latest_language(),
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::initialize").unwrap(),
        vec![],
        vec![],
    ));

    // Create Option<Color>::Some(Color::Red)
    // Option uses vector-based BCS representation: Some(x) = vector with one element
    let red = MoveValue::Struct(MoveStruct::RuntimeVariant(0, vec![]));
    let some_red = MoveValue::Vector(vec![red]);

    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::test_option_color").unwrap(),
        vec![],
        vec![some_red.simple_serialize().unwrap()],
    );
    assert_success!(status);

    let result = get_test_result(&h, acc.address());
    assert_eq!(result.value, 1);
    assert_eq!(String::from_utf8(result.message).unwrap(), "some_red");
}

/// Test passing Option<Color> with None
#[test]
fn test_option_none_enum() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/pack"),
        BuildOptions::move_2().set_latest_language(),
    ));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::initialize").unwrap(),
        vec![],
        vec![],
    ));

    // Create Option<Color>::None
    // Option uses vector-based BCS representation: None = empty vector
    let none_color = MoveValue::Vector(vec![]);

    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::test_option_color").unwrap(),
        vec![],
        vec![none_color.simple_serialize().unwrap()],
    );
    assert_success!(status);

    let result = get_test_result(&h, acc.address());
    assert_eq!(result.value, 0);
    assert_eq!(String::from_utf8(result.message).unwrap(), "none_color");
}
