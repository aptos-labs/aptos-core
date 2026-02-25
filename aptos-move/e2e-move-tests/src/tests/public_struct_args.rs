// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for public structs and enums with copy ability as transaction arguments.
//!
//! This module tests the feature that allows public structs/enums with the `copy` ability
//! to be passed as entry function arguments. When compiled with language version 2.4+,
//! pack functions are automatically generated for all public structs/enums.

use crate::{assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_types::{account_address::AccountAddress, on_chain_config::FeatureFlag};
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

/// Mimics `0xcafe::phantom_validation::TestResult`
#[derive(Serialize, Deserialize, Debug)]
struct PhantomTestResult {
    success: bool,
    value: u64,
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

fn assert_publish_fails(path: std::path::PathBuf) {
    let result = BuiltPackage::build(path, BuildOptions::move_2().set_latest_language());
    assert!(result.is_err(), "Expected compilation to fail");
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

/// Test that invalid entry function parameter types are rejected at compile time:
/// - private struct (extended checks require public visibility)
/// - non-copy struct (extended checks require copy ability)
/// - key struct (compiler rejects public structs with key ability)
#[test]
fn test_invalid_entry_params_rejected_at_compile_time() {
    assert_publish_fails(common::test_dir_path(
        "public_struct_args.data/negative_private",
    ));
    assert_publish_fails(common::test_dir_path(
        "public_struct_args.data/negative_nocopy",
    ));
    assert_publish_fails(common::test_dir_path(
        "public_struct_args.data/negative_key",
    ));
}

/// Test that generic container with private type argument is rejected at construction time.
/// Container<T> is public with copy, so it passes validation. When T=PrivatePoint (private struct),
/// construction fails because PrivatePoint has no public pack function.
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

    // Container<PrivatePoint> passes validation (Container is public copy, fields not checked).
    // Construction fails because PrivatePoint has no public pack function.
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

/// Test that vector of 11 Option<u64> elements is rejected when the feature is disabled.
/// With PUBLIC_STRUCT_ENUM_ARGS disabled, max_invocations is 10; 11 elements exceeds it.
/// This verifies the old (backwards-compatible) limit is still enforced when feature is off.
#[test]
fn test_option_u64_vector_exceeds_limit() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    h.enable_features(vec![], vec![FeatureFlag::PUBLIC_STRUCT_ENUM_ARGS]);

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

    // Create a vector with 11 Option<u64> elements (Some(1), Some(2), ..., Some(11))
    // Each Option::Some is represented as a vector with one element
    let mut options = vec![];
    for i in 1..=11 {
        let some_value = MoveValue::Vector(vec![MoveValue::U64(i)]);
        options.push(some_value);
    }
    let options_vector = MoveValue::Vector(options);

    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::test_option_u64_vector").unwrap(),
        vec![],
        vec![options_vector.simple_serialize().unwrap()],
    );

    // Should fail because max_invocations is 10 (feature disabled), and each Option counts as 1
    // invocation. The 11th Option will fail the check.
    assert!(!status.status().unwrap().is_success());
}

/// Test that vector of 100 Option<u64> elements succeeds (at the new limit).
/// With PUBLIC_STRUCT_ENUM_ARGS enabled, max_invocations is 100; exactly 100 elements is allowed.
#[test]
fn test_option_u64_vector_at_limit() {
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

    // Create a vector with exactly 100 Option<u64> elements (Some(1), ..., Some(100))
    // PUBLIC_STRUCT_ENUM_ARGS is enabled by default, so max_invocations = 100.
    let mut options = vec![];
    for i in 1..=100 {
        let some_value = MoveValue::Vector(vec![MoveValue::U64(i)]);
        options.push(some_value);
    }
    let options_vector = MoveValue::Vector(options);

    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::test_option_u64_vector").unwrap(),
        vec![],
        vec![options_vector.simple_serialize().unwrap()],
    );

    // Should succeed with exactly 100 elements (at the new limit)
    assert_success!(status);

    let result = get_test_result(&h, acc.address());
    // Expected sum: 1 + 2 + 3 + ... + 100 = 5050
    assert_eq!(result.value, 5050);
    assert_eq!(
        String::from_utf8(result.message).unwrap(),
        "option_u64_vector_received"
    );
}

/// Test that vector of 101 Option<u64> elements is rejected with the new limit.
/// With PUBLIC_STRUCT_ENUM_ARGS enabled, max_invocations is 100; 101 elements exceeds it.
#[test]
fn test_option_u64_vector_exceeds_new_limit() {
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

    // Create a vector with 101 Option<u64> elements (Some(1), ..., Some(101))
    // PUBLIC_STRUCT_ENUM_ARGS is enabled by default, so max_invocations = 100; 101 exceeds it.
    let mut options = vec![];
    for i in 1..=101 {
        let some_value = MoveValue::Vector(vec![MoveValue::U64(i)]);
        options.push(some_value);
    }
    let options_vector = MoveValue::Vector(options);

    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::test_option_u64_vector").unwrap(),
        vec![],
        vec![options_vector.simple_serialize().unwrap()],
    );

    // Should fail because max_invocations is 100 and 101 elements exceeds it
    assert!(!status.status().unwrap().is_success());
}

/// Test that vector<u64> with 100 elements succeeds (primitives have no limit)
/// This demonstrates that the max_invocations limit only applies to struct types,
/// not to primitive types like u64, which don't require constructor invocations
#[test]
fn test_u64_vector_no_limit() {
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

    // Create a vector with 100 u64 elements (1, 2, 3, ..., 100)
    // This is far beyond the max_invocations limit of 10, but should succeed
    // because primitives don't use constructors
    let mut values = vec![];
    for i in 1..=100 {
        values.push(MoveValue::U64(i));
    }
    let values_vector = MoveValue::Vector(values);

    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::test_u64_vector").unwrap(),
        vec![],
        vec![values_vector.simple_serialize().unwrap()],
    );

    // Should succeed because primitives don't count against max_invocations
    assert_success!(status);

    let result = get_test_result(&h, acc.address());
    // Expected sum: 1 + 2 + 3 + ... + 100 = 100 * 101 / 2 = 5050
    assert_eq!(result.value, 5050);
    assert_eq!(
        String::from_utf8(result.message).unwrap(),
        "u64_vector_received"
    );
}

// ========================================================================================
// Phantom Type Parameter Tests
// ========================================================================================

/// Test that Object<Hero> is accepted even when Hero is private.
/// Object<T>'s type parameter is phantom (not stored), so T doesn't need validation.
/// This test only verifies that the module compiles successfully.
#[test]
fn test_object_with_private_type_succeeds() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Publish the phantom validation test module
    // This should SUCCEED because Object<Hero> compiles even with private Hero
    // (phantom type parameters don't require the type to be public)
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/phantom_validation"),
        BuildOptions::move_2().set_latest_language(),
    ));
}

/// Test that Option<Hero> is accepted even when Hero is private.
/// Option is a whitelisted struct, so its type argument is not validated at compile time or
/// at VM validation time. The only value the caller can construct is None (since Hero has no
/// pack function), and None is accepted. Some(Hero) fails at construction time.
#[test]
fn test_option_with_private_type_none_allowed() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Option<Hero> compiles and publishes successfully even though Hero is private.
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/option_private_type"),
        BuildOptions::move_2().set_latest_language(),
    ));

    // Calling with None is accepted: Option<Hero> passes VM validation, and None needs no
    // inner constructor.
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::option_private_type::accept_option_hero").unwrap(),
        vec![],
        vec![bcs::to_bytes(&0u8).unwrap()], // BCS for None (0x00 tag)
    ));
}

/// Test that Option<NoCopyData> passes validation, None succeeds, but Some(NoCopyData{...})
/// fails at construction time because NoCopyData lacks copy ability.
/// Option is whitelisted so its type argument bypasses the copy check at pre-validation;
/// the copy check is enforced at construction time when the inner value is actually built.
/// None never triggers the inner type's constructor, so it always succeeds.
#[test]
fn test_option_with_nocopy_type_none_allowed_some_rejected() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Option<NoCopyData> publishes successfully even though NoCopyData lacks copy.
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/option_private_type"),
        BuildOptions::move_2().set_latest_language(),
    ));

    // None succeeds: no inner NoCopyData value is constructed, so the copy check is never
    // triggered.
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::option_private_type::accept_option_nocopy").unwrap(),
        vec![],
        vec![bcs::to_bytes(&Vec::<u8>::new()).unwrap()], // BCS for None (empty vec)
    ));

    // Some(NoCopyData{value: 42}) fails: NoCopyData lacks copy ability, so construct_public_copy_struct
    // rejects it when attempting to build the inner value.
    let some_nocopy = bcs::to_bytes(&vec![42u64]).unwrap(); // BCS for Some(NoCopyData{value:42})
    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::option_private_type::accept_option_nocopy").unwrap(),
        vec![],
        vec![some_nocopy],
    );
    assert_vm_status!(status, StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE);
}

/// View function equivalent of `test_option_with_nocopy_type_none_allowed_some_rejected`.
/// Verifies the same construction rules apply on the view function path.
#[test]
fn test_option_with_nocopy_type_view_function() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/option_private_type"),
        BuildOptions::move_2().set_latest_language(),
    ));

    // None succeeds: no inner NoCopyData constructed.
    let res = h.execute_view_function(
        str::parse("0xcafe::option_private_type::is_option_nocopy_none").unwrap(),
        vec![],
        vec![bcs::to_bytes(&Vec::<u8>::new()).unwrap()], // BCS for None (empty vec)
    );
    assert!(res.values.is_ok());
    let is_none: bool = bcs::from_bytes(&res.values.unwrap()[0]).unwrap();
    assert!(is_none);

    // Some(NoCopyData{value: 42}) fails: NoCopyData has no declared copy.
    let some_nocopy = bcs::to_bytes(&vec![42u64]).unwrap();
    let res = h.execute_view_function(
        str::parse("0xcafe::option_private_type::is_option_nocopy_none").unwrap(),
        vec![],
        vec![some_nocopy],
    );
    assert!(res.values.is_err());
}

/// Test Container<Hero> variant behavior: publish succeeds, the field-free Empty variant can be
/// passed, but the Value variant (which holds a private Hero) fails at construction time because
/// Hero has no public pack function. Mirrors Option<PrivateStruct> semantics: None succeeds,
/// Some(PrivateStruct{...}) fails with INVALID_MAIN_FUNCTION_SIGNATURE.
#[test]
fn test_user_enum_with_private_type_empty_succeeds_value_fails() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/negative_phantom_option"),
        BuildOptions::move_2().set_latest_language(),
    ));

    // Container<Hero>::Empty is variant index 1 (Value=0, Empty=1), no fields.
    let empty = MoveValue::Struct(MoveStruct::RuntimeVariant(1, vec![]));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::negative_phantom_option::test_container_hero").unwrap(),
        vec![],
        vec![empty.simple_serialize().unwrap()],
    ));

    // Container<Hero>::Value{data: Hero{health:1, level:2}} is variant index 0.
    // Hero is private (no public pack function): construction fails with INVALID_MAIN_FUNCTION_SIGNATURE.
    let hero = MoveValue::Struct(MoveStruct::Runtime(vec![
        MoveValue::U64(1), // health
        MoveValue::U64(2), // level
    ]));
    let value_variant = MoveValue::Struct(MoveStruct::RuntimeVariant(0, vec![hero]));
    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::negative_phantom_option::test_container_hero").unwrap(),
        vec![],
        vec![value_variant.simple_serialize().unwrap()],
    );
    assert_vm_status!(status, StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE);
}

/// Test Container<NoCopyData> behavior per variant.
///
/// Container<T> declares copy, so the struct *definition* has copy. Construction is allowed,
/// and validity is enforced per-variant at construction time — analogous to Option<T>:
/// - Empty variant: no inner value constructed → succeeds.
/// - Value variant: NoCopyData field must be constructed → NoCopyData has no declared copy
///   → rejected with INVALID_MAIN_FUNCTION_SIGNATURE.
#[test]
fn test_container_with_nocopy_type_empty_succeeds_value_fails() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/negative_phantom_option"),
        BuildOptions::move_2().set_latest_language(),
    ));

    // Empty variant (index 1): Container defines copy → passes declared-copy check.
    // No fields to construct → succeeds.
    let empty = MoveValue::Struct(MoveStruct::RuntimeVariant(1, vec![]));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::negative_phantom_option::test_container_nocopy").unwrap(),
        vec![],
        vec![empty.simple_serialize().unwrap()],
    ));

    // Value variant (index 0): NoCopyData field lacks declared copy → construction fails.
    let nocopy = MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::U64(42)]));
    let value_variant = MoveValue::Struct(MoveStruct::RuntimeVariant(0, vec![nocopy]));
    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::negative_phantom_option::test_container_nocopy").unwrap(),
        vec![],
        vec![value_variant.simple_serialize().unwrap()],
    );
    assert_vm_status!(status, StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE);
}

/// View function equivalent of `test_container_with_nocopy_type_empty_succeeds_value_fails`.
/// Verifies the same construction rules apply on the view function path.
#[test]
fn test_container_with_nocopy_type_view_function() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/negative_phantom_option"),
        BuildOptions::move_2().set_latest_language(),
    ));

    // Empty variant (index 1): Container declares copy, no fields → succeeds.
    let empty = MoveValue::Struct(MoveStruct::RuntimeVariant(1, vec![]));
    let res = h.execute_view_function(
        str::parse("0xcafe::negative_phantom_option::check_container_nocopy").unwrap(),
        vec![],
        vec![empty.simple_serialize().unwrap()],
    );
    assert!(res.values.is_ok());

    // Value variant (index 0): NoCopyData field has no declared copy → fails.
    let nocopy = MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::U64(42)]));
    let value_variant = MoveValue::Struct(MoveStruct::RuntimeVariant(0, vec![nocopy]));
    let res = h.execute_view_function(
        str::parse("0xcafe::negative_phantom_option::check_container_nocopy").unwrap(),
        vec![],
        vec![value_variant.simple_serialize().unwrap()],
    );
    assert!(res.values.is_err());
}

/// Test Option<CopyData<NoCopyData>> and Container<CopyData<NoCopyData>> where CopyData<T>
/// declares copy but T = NoCopyData does not. Two layers of generic wrapping: the outer
/// types (Option, Container, CopyData) all declare copy; only the innermost NoCopyData lacks
/// it. Construction fails only when that innermost value must actually be built.
///
/// Option<CopyData<NoCopyData>>:
///   - None  → succeeds (no CopyData or NoCopyData constructed)
///   - Some  → fails   (CopyData passes declared-copy check; NoCopyData fails it)
///
/// Container<CopyData<NoCopyData>>:
///   - Empty → succeeds (no CopyData or NoCopyData constructed)
///   - Value → fails   (same reason as Some above)
#[test]
fn test_nested_no_copy_type_entry_function() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/negative_phantom_option"),
        BuildOptions::move_2().set_latest_language(),
    ));

    // --- Option<CopyData<NoCopyData>> ---

    // None: no inner value constructed → succeeds.
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::negative_phantom_option::test_option_copy_wrapper_nocopy").unwrap(),
        vec![],
        vec![bcs::to_bytes(&Vec::<u8>::new()).unwrap()],
    ));

    // Some(CopyData{data: NoCopyData{value: 42}}):
    //   CopyData declares copy → passes; NoCopyData has no declared copy → fails.
    // BCS: Option<CopyData<NoCopyData>>::Some = [1, <CopyData BCS>]
    //      CopyData{data: NoCopyData{value: 42}} BCS = NoCopyData BCS = u64 42 BCS
    //      = bcs::to_bytes(&vec![42u64])
    let some_copy_wrapper = bcs::to_bytes(&vec![42u64]).unwrap();
    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::negative_phantom_option::test_option_copy_wrapper_nocopy").unwrap(),
        vec![],
        vec![some_copy_wrapper],
    );
    assert_vm_status!(status, StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE);

    // --- Container<CopyData<NoCopyData>> ---

    // Empty (variant index 1): no inner value constructed → succeeds.
    let empty = MoveValue::Struct(MoveStruct::RuntimeVariant(1, vec![]));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::negative_phantom_option::test_container_copy_wrapper_nocopy").unwrap(),
        vec![],
        vec![empty.simple_serialize().unwrap()],
    ));

    // Value{data: CopyData{data: NoCopyData{value: 42}}} (variant index 0):
    //   CopyData passes; NoCopyData has no declared copy → fails.
    let nocopy = MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::U64(42)]));
    let copy_wrapper = MoveValue::Struct(MoveStruct::Runtime(vec![nocopy]));
    let value_variant = MoveValue::Struct(MoveStruct::RuntimeVariant(0, vec![copy_wrapper]));
    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::negative_phantom_option::test_container_copy_wrapper_nocopy").unwrap(),
        vec![],
        vec![value_variant.simple_serialize().unwrap()],
    );
    assert_vm_status!(status, StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE);
}

/// Test that Object<u64> is accepted (even with primitive type parameter).
/// This verifies Object works correctly with any type parameter.
/// This test only verifies that the module compiles successfully.
#[test]
fn test_object_with_primitive_type_succeeds() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Publish the phantom validation test module
    // This should SUCCEED because Object<u64> compiles with any type parameter
    // (phantom type parameters don't require validation)
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/phantom_validation"),
        BuildOptions::move_2().set_latest_language(),
    ));
}

/// Test that user-defined phantom enum Wrapper<Hero> is accepted even when Hero is private.
/// This demonstrates that phantom type parameters work the same way for user-defined enums
/// as they do for framework types like Object<T>.
#[test]
fn test_user_enum_phantom_with_private_type_succeeds() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Publish the phantom validation test module
    // This should SUCCEED because Wrapper<Hero> compiles even with private Hero
    // (phantom type parameters don't require the type to be public)
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/phantom_validation"),
        BuildOptions::move_2().set_latest_language(),
    ));

    // Initialize test result
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::phantom_validation::initialize").unwrap(),
        vec![],
        vec![],
    ));

    // Create a Wrapper<Hero>::Some { id: 42 } (variant index 0)
    let wrapper_hero = MoveValue::Struct(MoveStruct::RuntimeVariant(0, vec![
        MoveValue::U64(42), // id
    ]));

    // Call test_wrapper_hero with Wrapper<Hero>
    // This should SUCCEED because Wrapper<T>'s type parameter is phantom
    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::phantom_validation::test_wrapper_hero").unwrap(),
        vec![],
        vec![wrapper_hero.simple_serialize().unwrap()],
    );
    assert_success!(status);

    // Verify the function executed successfully by reading the resource
    let result: PhantomTestResult = h
        .read_resource_raw(
            acc.address(),
            "0xcafe::phantom_validation::TestResult".parse().unwrap(),
        )
        .map(|bytes| bcs::from_bytes(&bytes).unwrap())
        .unwrap();

    assert!(result.success); // success = true
    assert_eq!(result.value, 77); // value = 77
}

/// Test that user-defined phantom enum Wrapper<u64> is accepted.
/// This verifies user-defined phantom enums work correctly with any type parameter.
#[test]
fn test_user_enum_phantom_with_primitive_type_succeeds() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Publish the phantom validation test module
    // This should SUCCEED because Wrapper<u64> compiles with any type parameter
    // (phantom type parameters don't require validation)
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/phantom_validation"),
        BuildOptions::move_2().set_latest_language(),
    ));

    // Initialize test result
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::phantom_validation::initialize").unwrap(),
        vec![],
        vec![],
    ));

    // Create a Wrapper<u64>::None (variant index 1)
    let wrapper_u64 = MoveValue::Struct(MoveStruct::RuntimeVariant(1, vec![]));

    // Call test_wrapper_u64 with Wrapper<u64>
    // This should SUCCEED because Wrapper<u64> is a valid transaction argument
    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::phantom_validation::test_wrapper_u64").unwrap(),
        vec![],
        vec![wrapper_u64.simple_serialize().unwrap()],
    );
    assert_success!(status);

    // Verify the function executed successfully by reading the resource
    let result: PhantomTestResult = h
        .read_resource_raw(
            acc.address(),
            "0xcafe::phantom_validation::TestResult".parse().unwrap(),
        )
        .map(|bytes| bcs::from_bytes(&bytes).unwrap())
        .unwrap();

    assert!(result.success); // success = true
    assert_eq!(result.value, 88); // value = 88
}

// ========================================================================================
// Multiple Type Parameter Tests (Pair<T, U>)
// ========================================================================================

/// Test Pair<PublicPoint, PublicPoint>: both type arguments are valid public copy structs.
/// Both fields are constructable, so the whole Pair is constructable. Expect success.
#[test]
fn test_pair_both_type_params_valid() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/pair_type_params"),
        BuildOptions::move_2().set_latest_language(),
    ));

    // Pair<PublicPoint, PublicPoint> { first: {x:10, y:20}, second: {x:30, y:40} }
    let pair = MoveValue::Struct(MoveStruct::Runtime(vec![
        MoveValue::Struct(MoveStruct::Runtime(vec![
            MoveValue::U64(10),
            MoveValue::U64(20),
        ])),
        MoveValue::Struct(MoveStruct::Runtime(vec![
            MoveValue::U64(30),
            MoveValue::U64(40),
        ])),
    ]));

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::pair_type_params::test_pair_both_valid").unwrap(),
        vec![],
        vec![pair.simple_serialize().unwrap()],
    ));
}

/// Test Pair<PublicPoint, PrivateData>: first type argument is valid, second is private.
/// Construction fails on the second field because PrivateData has no public pack function.
#[test]
fn test_pair_second_type_param_private() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/pair_type_params"),
        BuildOptions::move_2().set_latest_language(),
    ));

    // Pair<PublicPoint, PrivateData> { first: {x:10, y:20}, second: {value:99} }
    let pair = MoveValue::Struct(MoveStruct::Runtime(vec![
        MoveValue::Struct(MoveStruct::Runtime(vec![
            MoveValue::U64(10),
            MoveValue::U64(20),
        ])),
        MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::U64(99)])),
    ]));

    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::pair_type_params::test_pair_second_invalid").unwrap(),
        vec![],
        vec![pair.simple_serialize().unwrap()],
    );
    assert_vm_status!(status, StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE);
}

/// Test Pair<PrivateData, PublicPoint>: first type argument is private, second is valid.
/// Construction fails immediately on the first field because PrivateData has no public pack function.
#[test]
fn test_pair_first_type_param_private() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/pair_type_params"),
        BuildOptions::move_2().set_latest_language(),
    ));

    // Pair<PrivateData, PublicPoint> { first: {value:99}, second: {x:10, y:20} }
    let pair = MoveValue::Struct(MoveStruct::Runtime(vec![
        MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::U64(99)])),
        MoveValue::Struct(MoveStruct::Runtime(vec![
            MoveValue::U64(10),
            MoveValue::U64(20),
        ])),
    ]));

    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::pair_type_params::test_pair_first_invalid").unwrap(),
        vec![],
        vec![pair.simple_serialize().unwrap()],
    );
    assert_vm_status!(status, StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE);
}

// ========================================================================================
// vector<Option<Struct>> Tests
// ========================================================================================

/// Test a small vector<Option<Point>> with a mix of Some and None values.
/// Verifies the end-to-end path for option vectors containing public structs.
#[test]
fn test_vector_option_struct_basic() {
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

    // [Some(Point{1,2}), None, Some(Point{3,4})]
    // Option<Point> uses vector-based BCS: Some(p) = [p], None = []
    let opts = MoveValue::Vector(vec![
        MoveValue::Vector(vec![MoveValue::Struct(MoveStruct::Runtime(vec![
            MoveValue::U64(1),
            MoveValue::U64(2),
        ]))]),
        MoveValue::Vector(vec![]),
        MoveValue::Vector(vec![MoveValue::Struct(MoveStruct::Runtime(vec![
            MoveValue::U64(3),
            MoveValue::U64(4),
        ]))]),
    ]);

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::test_option_point_vector").unwrap(),
        vec![],
        vec![opts.simple_serialize().unwrap()],
    ));

    let result = get_test_result(&h, acc.address());
    assert_eq!(result.value, 10); // (1+2) + (3+4)
    assert_eq!(
        String::from_utf8(result.message).unwrap(),
        "option_point_vector_received"
    );
}

/// Test vector<Option<Point>> with exactly 50 Some(Point) elements.
/// Each Some(Point) costs 2 invocations (1 for Option + 1 for Point).
/// 50 × 2 = 100 invocations, exactly at the limit. Expect success.
#[test]
fn test_vector_option_struct_at_limit() {
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

    // 50 × Some(Point{i, i}) — 50 × 2 = 100 invocations (exactly at the limit)
    let opts = MoveValue::Vector(
        (0u64..50)
            .map(|i| {
                MoveValue::Vector(vec![MoveValue::Struct(MoveStruct::Runtime(vec![
                    MoveValue::U64(i),
                    MoveValue::U64(i),
                ]))])
            })
            .collect(),
    );

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::test_option_point_vector").unwrap(),
        vec![],
        vec![opts.simple_serialize().unwrap()],
    ));

    let result = get_test_result(&h, acc.address());
    // sum of 2*i for i in 0..50 = 2 * (0+1+...+49) = 2 * 1225 = 2450
    assert_eq!(result.value, 2450);
    assert_eq!(
        String::from_utf8(result.message).unwrap(),
        "option_point_vector_received"
    );
}

/// Test vector<Option<Point>> with 51 Some(Point) elements, which exceeds the limit.
/// 51 × 2 = 102 invocations > 100. Expect failure.
#[test]
fn test_vector_option_struct_exceeds_limit() {
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

    // 51 × Some(Point{1,1}) — 51 × 2 = 102 invocations > 100 limit
    let opts = MoveValue::Vector(
        (0..51)
            .map(|_| {
                MoveValue::Vector(vec![MoveValue::Struct(MoveStruct::Runtime(vec![
                    MoveValue::U64(1),
                    MoveValue::U64(1),
                ]))])
            })
            .collect(),
    );

    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::test_option_point_vector").unwrap(),
        vec![],
        vec![opts.simple_serialize().unwrap()],
    );
    assert!(!status.status().unwrap().is_success());
}

// ========================================================================================
// 3-Level Nesting Tests (Option<CopyData<CopyData<NoCopyData>>>)
// ========================================================================================

/// Test Option<CopyData<CopyData<NoCopyData>>> — three levels of generic wrapping.
/// All outer types (Option, CopyData, CopyData) declare copy; only the innermost
/// NoCopyData lacks it. The recursive validation must descend all three levels.
///
/// - None  → succeeds: no value constructed at any level.
/// - Some(CopyData{CopyData{NoCopyData{7}}}) → fails at NoCopyData (no declared copy).
#[test]
fn test_triple_nested_nocopy_entry_function() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/negative_phantom_option"),
        BuildOptions::move_2().set_latest_language(),
    ));

    // None: BCS for empty vector (Option uses vector-based encoding)
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::negative_phantom_option::test_option_triple_nested_nocopy").unwrap(),
        vec![],
        vec![bcs::to_bytes(&Vec::<u8>::new()).unwrap()],
    ));

    // Some(CopyData{CopyData{NoCopyData{value:7}}}):
    // Each CopyData<T> wrapper is a single-field struct transparent at BCS level.
    // BCS = [1 (vec length)] ++ [7u64 as LE bytes] — same pattern as the 2-level test.
    let some_triple = bcs::to_bytes(&vec![7u64]).unwrap();
    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::negative_phantom_option::test_option_triple_nested_nocopy").unwrap(),
        vec![],
        vec![some_triple],
    );
    assert_vm_status!(status, StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE);
}

/// View function equivalent of `test_triple_nested_nocopy_entry_function`.
/// Verifies the same 3-level construction rules apply on the view function path.
#[test]
fn test_triple_nested_nocopy_view_function() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/negative_phantom_option"),
        BuildOptions::move_2().set_latest_language(),
    ));

    // None: succeeds — no value constructed at any level.
    let res = h.execute_view_function(
        str::parse("0xcafe::negative_phantom_option::check_option_triple_nested_nocopy").unwrap(),
        vec![],
        vec![bcs::to_bytes(&Vec::<u8>::new()).unwrap()],
    );
    assert!(res.values.is_ok());

    // Some(CopyData{CopyData{NoCopyData{7}}}): fails at NoCopyData construction.
    let some_triple = bcs::to_bytes(&vec![7u64]).unwrap();
    let res = h.execute_view_function(
        str::parse("0xcafe::negative_phantom_option::check_option_triple_nested_nocopy").unwrap(),
        vec![],
        vec![some_triple],
    );
    assert!(res.values.is_err());
}

/// Tests that a public copy struct with an `Option<PrivateT>` field is a valid transaction
/// argument type, illustrating the full flow from extended checker through execution.
///
/// `Wrapper<Hero>` has a field `o: Option<Hero>` where `Hero` is private. The extended checker
/// does not recurse into Option's type argument (Option is whitelisted), so the module compiles.
/// The VM validation also passes. At construction time:
/// - `None`  → no Hero value needed → construction succeeds → execution proceeds
/// - `Some(Hero)` → Hero has no pack function → construction fails with INVALID_MAIN_FUNCTION_SIGNATURE
///
/// Both entry function and view function paths are exercised.
#[test]
fn test_option_in_public_struct() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Step 1: module compiles and publishes — the extended checker does not reject
    // Wrapper<Hero> even though Hero is private, because Option is whitelisted.
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/option_in_wrapper"),
        BuildOptions::move_2().set_latest_language(),
    ));

    // Wrapper<Hero> { o: None }  — Option<Hero> = None = empty vector in Move's runtime repr.
    let wrapper_none = MoveValue::Struct(MoveStruct::Runtime(vec![
        MoveValue::Vector(vec![]), // Option<Hero> = None
    ]));

    // Wrapper<Hero> { o: Some(Hero { health: 42 }) } — cannot be constructed since Hero is private.
    let hero = MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::U64(42)]));
    let wrapper_some = MoveValue::Struct(MoveStruct::Runtime(vec![
        MoveValue::Vector(vec![hero]), // Option<Hero> = Some(Hero)
    ]));

    // --- Entry function ---

    // None: passes VM validation, construction succeeds (no Hero value needed), executes cleanly.
    // The entry function asserts is_none, so success also confirms the value arrived correctly.
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::option_in_wrapper::check_none").unwrap(),
        vec![],
        vec![wrapper_none.simple_serialize().unwrap()],
    ));

    // Some(Hero): passes VM validation (Wrapper<Hero> is still a valid type), but construction
    // fails because Hero has no pack function — INVALID_MAIN_FUNCTION_SIGNATURE.
    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::option_in_wrapper::check_none").unwrap(),
        vec![],
        vec![wrapper_some.simple_serialize().unwrap()],
    );
    assert_vm_status!(status, StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE);

    // --- View function ---

    // None: view function accepts it, construction succeeds, returns true.
    let res = h.execute_view_function(
        str::parse("0xcafe::option_in_wrapper::check_none_view").unwrap(),
        vec![],
        vec![wrapper_none.simple_serialize().unwrap()],
    );
    assert!(res.values.is_ok());
    let is_none: bool = bcs::from_bytes(&res.values.unwrap()[0]).unwrap();
    assert!(is_none);

    // Some(Hero): view function also fails at construction for the same reason.
    let res = h.execute_view_function(
        str::parse("0xcafe::option_in_wrapper::check_none_view").unwrap(),
        vec![],
        vec![wrapper_some.simple_serialize().unwrap()],
    );
    assert!(res.values.is_err());
}
