// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for public structs and enums with copy ability as transaction arguments.
//!
//! This module tests the feature that allows public structs/enums with the `copy` ability
//! to be passed as entry function arguments. When compiled with language version 2.4+,
//! pack functions are automatically generated for all public structs/enums.
//!
//! Tests are grouped by the package they publish, so each package is compiled and published
//! only once per group, dramatically reducing total test time.

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

// ========================================================================================
// Group 1: Tests using `pack` package with default features
// ========================================================================================

/// Consolidation of all tests that publish the `pack` package with default features.
/// Publishes once, initializes once, then runs all sub-tests sequentially.
///
/// Run with: RUST_MIN_STACK=104857600 cargo test -p e2e-move-tests -- public_struct_args
#[test]
fn test_pack_default_features() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Compile with language 2.4+ to auto-generate pack functions
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/pack"),
        BuildOptions::move_2().set_latest_language(),
    ));

    // Initialize the test result (uses move_to; subsequent entry functions use borrow_global_mut)
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::public_struct_test::initialize").unwrap(),
        vec![],
        vec![],
    ));

    // --- test_public_struct_point ---
    {
        let point_value = MoveValue::Struct(MoveStruct::Runtime(vec![
            MoveValue::U64(10),
            MoveValue::U64(20),
        ]));

        let status = h.run_entry_function(
            &acc,
            str::parse("0xcafe::public_struct_test::test_point").unwrap(),
            vec![],
            vec![point_value.simple_serialize().unwrap()],
        );
        assert_success!(status);

        let result = get_test_result(&h, acc.address());
        assert_eq!(result.value, 30);
        assert_eq!(String::from_utf8(result.message).unwrap(), "point_received");
    }

    // --- test_public_struct_nested ---
    {
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
        assert_eq!(result.value, 10);
        assert_eq!(
            String::from_utf8(result.message).unwrap(),
            "rectangle_received"
        );
    }

    // --- test_public_struct_with_string ---
    {
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
        assert_eq!(result.value, 30);
        assert_eq!(String::from_utf8(result.message).unwrap(), "test_data");
    }

    // --- test_public_enum_unit_variant ---
    {
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

    // --- test_public_enum_with_fields ---
    {
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
        assert_eq!(result.value, 175);
        assert_eq!(String::from_utf8(result.message).unwrap(), "custom");
    }

    // --- test_public_enum_with_struct_fields ---
    {
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
        assert_eq!(result.value, 30);
        assert_eq!(String::from_utf8(result.message).unwrap(), "circle");
    }

    // --- test_vector_of_public_structs ---
    {
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
        assert_eq!(result.value, 21);
        assert_eq!(
            String::from_utf8(result.message).unwrap(),
            "point_vector_received"
        );
    }

    // --- test_whitelisted_string_works ---
    {
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
        assert_eq!(result.value, 11);
        assert_eq!(String::from_utf8(result.message).unwrap(), "hello_world");
    }

    // --- test_option_some_struct ---
    {
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
        assert_eq!(result.value, 30);
        assert_eq!(String::from_utf8(result.message).unwrap(), "some_point");
    }

    // --- test_option_none_struct ---
    {
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

    // --- test_option_some_enum ---
    {
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

    // --- test_option_none_enum ---
    {
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

    // --- test_option_u64_vector_at_limit ---
    {
        let mut options = vec![];
        for i in 1..=32 {
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
        assert_success!(status);

        let result = get_test_result(&h, acc.address());
        assert_eq!(result.value, 528);
        assert_eq!(
            String::from_utf8(result.message).unwrap(),
            "option_u64_vector_received"
        );
    }

    // --- test_option_u64_vector_exceeds_new_limit ---
    {
        let mut options = vec![];
        for i in 1..=33 {
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
        assert!(!status.status().unwrap().is_success());
    }

    // --- test_u64_vector_no_limit ---
    {
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
        assert_success!(status);

        let result = get_test_result(&h, acc.address());
        assert_eq!(result.value, 5050);
        assert_eq!(
            String::from_utf8(result.message).unwrap(),
            "u64_vector_received"
        );
    }

    // --- test_vector_option_struct_basic ---
    {
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
        assert_eq!(result.value, 10);
        assert_eq!(
            String::from_utf8(result.message).unwrap(),
            "option_point_vector_received"
        );
    }

    // --- test_vector_option_struct_at_limit ---
    {
        let opts = MoveValue::Vector(
            (0u64..16)
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
        assert_eq!(result.value, 240);
        assert_eq!(
            String::from_utf8(result.message).unwrap(),
            "option_point_vector_received"
        );
    }

    // --- test_vector_option_struct_exceeds_limit ---
    {
        let opts = MoveValue::Vector(
            (0..17)
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

    // --- test_vector_of_vector_of_structs ---
    {
        let inner1 = MoveValue::Vector(vec![
            MoveValue::Struct(MoveStruct::Runtime(vec![
                MoveValue::U64(1),
                MoveValue::U64(2),
            ])),
            MoveValue::Struct(MoveStruct::Runtime(vec![
                MoveValue::U64(3),
                MoveValue::U64(4),
            ])),
        ]);
        let inner2 = MoveValue::Vector(vec![MoveValue::Struct(MoveStruct::Runtime(vec![
            MoveValue::U64(5),
            MoveValue::U64(6),
        ]))]);
        let nested = MoveValue::Vector(vec![inner1, inner2]);

        assert_success!(h.run_entry_function(
            &acc,
            str::parse("0xcafe::public_struct_test::test_nested_point_vector").unwrap(),
            vec![],
            vec![nested.simple_serialize().unwrap()],
        ));

        let result = get_test_result(&h, acc.address());
        assert_eq!(result.value, 21);
        assert_eq!(
            String::from_utf8(result.message).unwrap(),
            "nested_point_vector_received"
        );
    }

    // --- test_struct_with_enum_field ---
    {
        let color_green = MoveValue::Struct(MoveStruct::RuntimeVariant(1, vec![]));
        let labeled = MoveValue::Struct(MoveStruct::Runtime(vec![color_green, MoveValue::U64(10)]));

        assert_success!(h.run_entry_function(
            &acc,
            str::parse("0xcafe::public_struct_test::test_labeled").unwrap(),
            vec![],
            vec![labeled.simple_serialize().unwrap()],
        ));

        let result = get_test_result(&h, acc.address());
        assert_eq!(result.value, 12);
        assert_eq!(
            String::from_utf8(result.message).unwrap(),
            "labeled_received"
        );
    }
}

// ========================================================================================
// Group 2: Tests using `pack` package with PUBLIC_STRUCT_ENUM_ARGS disabled
// ========================================================================================

/// Tests that publish `pack` but disable `PUBLIC_STRUCT_ENUM_ARGS`.
#[test]
fn test_pack_feature_disabled() {
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

    // --- test_option_u64_vector_exceeds_limit ---
    // With PUBLIC_STRUCT_ENUM_ARGS disabled, max_invocations is 10; 11 elements exceeds it.
    {
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
        assert!(!status.status().unwrap().is_success());
    }

    // --- test_public_struct_rejected_when_feature_flag_disabled ---
    // Point is valid when feature is on, but rejected when off.
    {
        let point = MoveValue::Struct(MoveStruct::Runtime(vec![
            MoveValue::U64(1),
            MoveValue::U64(2),
        ]));

        let status = h.run_entry_function(
            &acc,
            str::parse("0xcafe::public_struct_test::test_point").unwrap(),
            vec![],
            vec![point.simple_serialize().unwrap()],
        );
        assert_vm_status!(status, StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE);
    }
}

// ========================================================================================
// Group 3: Phantom validation package tests
// ========================================================================================

/// Tests using the `phantom_validation` package.
/// Publishes once, initializes once, then runs phantom type parameter sub-tests.
#[test]
fn test_phantom_validation_package() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Publish succeeds because Object<Hero> and Wrapper<Hero> compile even with private Hero
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

    // --- test_user_enum_phantom_with_private_type_succeeds ---
    {
        let wrapper_hero =
            MoveValue::Struct(MoveStruct::RuntimeVariant(0, vec![MoveValue::U64(42)]));

        let status = h.run_entry_function(
            &acc,
            str::parse("0xcafe::phantom_validation::test_wrapper_hero").unwrap(),
            vec![],
            vec![wrapper_hero.simple_serialize().unwrap()],
        );
        assert_success!(status);

        let result: PhantomTestResult = h
            .read_resource_raw(
                acc.address(),
                "0xcafe::phantom_validation::TestResult".parse().unwrap(),
            )
            .map(|bytes| bcs::from_bytes(&bytes).unwrap())
            .unwrap();

        assert!(result.success);
        assert_eq!(result.value, 77);
    }

    // --- test_user_enum_phantom_with_primitive_type_succeeds ---
    {
        let wrapper_u64 = MoveValue::Struct(MoveStruct::RuntimeVariant(1, vec![]));

        let status = h.run_entry_function(
            &acc,
            str::parse("0xcafe::phantom_validation::test_wrapper_u64").unwrap(),
            vec![],
            vec![wrapper_u64.simple_serialize().unwrap()],
        );
        assert_success!(status);

        let result: PhantomTestResult = h
            .read_resource_raw(
                acc.address(),
                "0xcafe::phantom_validation::TestResult".parse().unwrap(),
            )
            .map(|bytes| bcs::from_bytes(&bytes).unwrap())
            .unwrap();

        assert!(result.success);
        assert_eq!(result.value, 88);
    }
}

// ========================================================================================
// Group 4: Option with private type tests
// ========================================================================================

/// Tests using the `option_private_type` package. Stateless accept/reject tests.
#[test]
fn test_option_private_type_package() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/option_private_type"),
        BuildOptions::move_2().set_latest_language(),
    ));

    // --- test_option_with_private_type_none_allowed ---
    {
        assert_success!(h.run_entry_function(
            &acc,
            str::parse("0xcafe::option_private_type::accept_option_hero").unwrap(),
            vec![],
            vec![bcs::to_bytes(&0u8).unwrap()],
        ));
    }

    // --- test_option_with_nocopy_type_none_allowed_some_rejected ---
    {
        // None succeeds
        assert_success!(h.run_entry_function(
            &acc,
            str::parse("0xcafe::option_private_type::accept_option_nocopy").unwrap(),
            vec![],
            vec![bcs::to_bytes(&Vec::<u8>::new()).unwrap()],
        ));

        // Some(NoCopyData{value: 42}) fails
        let some_nocopy = bcs::to_bytes(&vec![42u64]).unwrap();
        let status = h.run_entry_function(
            &acc,
            str::parse("0xcafe::option_private_type::accept_option_nocopy").unwrap(),
            vec![],
            vec![some_nocopy],
        );
        assert_vm_status!(status, StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE);
    }

    // --- test_option_with_nocopy_type_view_function ---
    {
        // None succeeds
        let res = h.execute_view_function(
            str::parse("0xcafe::option_private_type::is_option_nocopy_none").unwrap(),
            vec![],
            vec![bcs::to_bytes(&Vec::<u8>::new()).unwrap()],
        );
        assert!(res.values.is_ok());
        let is_none: bool = bcs::from_bytes(&res.values.unwrap()[0]).unwrap();
        assert!(is_none);

        // Some(NoCopyData{value: 42}) fails
        let some_nocopy = bcs::to_bytes(&vec![42u64]).unwrap();
        let res = h.execute_view_function(
            str::parse("0xcafe::option_private_type::is_option_nocopy_none").unwrap(),
            vec![],
            vec![some_nocopy],
        );
        assert!(res.values.is_err());
    }
}

// ========================================================================================
// Group 5: Negative phantom option tests
// ========================================================================================

/// Tests using the `negative_phantom_option` package. Stateless tests.
#[test]
fn test_negative_phantom_option_package() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/negative_phantom_option"),
        BuildOptions::move_2().set_latest_language(),
    ));

    // --- test_user_enum_with_private_type_empty_succeeds_value_fails ---
    {
        // Container<Hero>::Empty succeeds
        let empty = MoveValue::Struct(MoveStruct::RuntimeVariant(1, vec![]));
        assert_success!(h.run_entry_function(
            &acc,
            str::parse("0xcafe::negative_phantom_option::test_container_hero").unwrap(),
            vec![],
            vec![empty.simple_serialize().unwrap()],
        ));

        // Container<Hero>::Value{data: Hero{...}} fails
        let hero = MoveValue::Struct(MoveStruct::Runtime(vec![
            MoveValue::U64(1),
            MoveValue::U64(2),
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

    // --- test_container_with_nocopy_type_empty_succeeds_value_fails ---
    {
        // Empty variant succeeds
        let empty = MoveValue::Struct(MoveStruct::RuntimeVariant(1, vec![]));
        assert_success!(h.run_entry_function(
            &acc,
            str::parse("0xcafe::negative_phantom_option::test_container_nocopy").unwrap(),
            vec![],
            vec![empty.simple_serialize().unwrap()],
        ));

        // Value variant fails
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

    // --- test_container_with_nocopy_type_view_function ---
    {
        // Empty variant succeeds
        let empty = MoveValue::Struct(MoveStruct::RuntimeVariant(1, vec![]));
        let res = h.execute_view_function(
            str::parse("0xcafe::negative_phantom_option::check_container_nocopy").unwrap(),
            vec![],
            vec![empty.simple_serialize().unwrap()],
        );
        assert!(res.values.is_ok());

        // Value variant fails
        let nocopy = MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::U64(42)]));
        let value_variant = MoveValue::Struct(MoveStruct::RuntimeVariant(0, vec![nocopy]));
        let res = h.execute_view_function(
            str::parse("0xcafe::negative_phantom_option::check_container_nocopy").unwrap(),
            vec![],
            vec![value_variant.simple_serialize().unwrap()],
        );
        assert!(res.values.is_err());
    }

    // --- test_nested_no_copy_type_entry_function ---
    {
        // Option<CopyData<NoCopyData>>: None succeeds
        assert_success!(h.run_entry_function(
            &acc,
            str::parse("0xcafe::negative_phantom_option::test_option_copy_wrapper_nocopy").unwrap(),
            vec![],
            vec![bcs::to_bytes(&Vec::<u8>::new()).unwrap()],
        ));

        // Option<CopyData<NoCopyData>>: Some fails
        let some_copy_wrapper = bcs::to_bytes(&vec![42u64]).unwrap();
        let status = h.run_entry_function(
            &acc,
            str::parse("0xcafe::negative_phantom_option::test_option_copy_wrapper_nocopy").unwrap(),
            vec![],
            vec![some_copy_wrapper],
        );
        assert_vm_status!(status, StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE);

        // Container<CopyData<NoCopyData>>: Empty succeeds
        let empty = MoveValue::Struct(MoveStruct::RuntimeVariant(1, vec![]));
        assert_success!(h.run_entry_function(
            &acc,
            str::parse("0xcafe::negative_phantom_option::test_container_copy_wrapper_nocopy",)
                .unwrap(),
            vec![],
            vec![empty.simple_serialize().unwrap()],
        ));

        // Container<CopyData<NoCopyData>>: Value fails
        let nocopy = MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::U64(42)]));
        let copy_wrapper = MoveValue::Struct(MoveStruct::Runtime(vec![nocopy]));
        let value_variant = MoveValue::Struct(MoveStruct::RuntimeVariant(0, vec![copy_wrapper]));
        let status = h.run_entry_function(
            &acc,
            str::parse("0xcafe::negative_phantom_option::test_container_copy_wrapper_nocopy")
                .unwrap(),
            vec![],
            vec![value_variant.simple_serialize().unwrap()],
        );
        assert_vm_status!(status, StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE);
    }

    // --- test_triple_nested_nocopy_entry_function ---
    {
        // None succeeds
        assert_success!(h.run_entry_function(
            &acc,
            str::parse("0xcafe::negative_phantom_option::test_option_triple_nested_nocopy")
                .unwrap(),
            vec![],
            vec![bcs::to_bytes(&Vec::<u8>::new()).unwrap()],
        ));

        // Some(CopyData{CopyData{NoCopyData{7}}}) fails
        let some_triple = bcs::to_bytes(&vec![7u64]).unwrap();
        let status = h.run_entry_function(
            &acc,
            str::parse("0xcafe::negative_phantom_option::test_option_triple_nested_nocopy")
                .unwrap(),
            vec![],
            vec![some_triple],
        );
        assert_vm_status!(status, StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE);
    }

    // --- test_triple_nested_nocopy_view_function ---
    {
        // None succeeds
        let res = h.execute_view_function(
            str::parse("0xcafe::negative_phantom_option::check_option_triple_nested_nocopy")
                .unwrap(),
            vec![],
            vec![bcs::to_bytes(&Vec::<u8>::new()).unwrap()],
        );
        assert!(res.values.is_ok());

        // Some(CopyData{CopyData{NoCopyData{7}}}) fails
        let some_triple = bcs::to_bytes(&vec![7u64]).unwrap();
        let res = h.execute_view_function(
            str::parse("0xcafe::negative_phantom_option::check_option_triple_nested_nocopy")
                .unwrap(),
            vec![],
            vec![some_triple],
        );
        assert!(res.values.is_err());
    }
}

// ========================================================================================
// Group 6: Pair type params tests
// ========================================================================================

/// Tests using the `pair_type_params` package.
#[test]
fn test_pair_type_params_package() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/pair_type_params"),
        BuildOptions::move_2().set_latest_language(),
    ));

    // --- test_pair_both_type_params_valid ---
    {
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

    // --- test_pair_second_type_param_private ---
    {
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

    // --- test_pair_first_type_param_private ---
    {
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

    // --- test_pair_mixed_enum_and_struct_type_params ---
    {
        let simple_tag_y = MoveValue::Struct(MoveStruct::RuntimeVariant(1, vec![]));
        let public_point = MoveValue::Struct(MoveStruct::Runtime(vec![
            MoveValue::U64(3),
            MoveValue::U64(7),
        ]));
        let pair = MoveValue::Struct(MoveStruct::Runtime(vec![simple_tag_y, public_point]));

        assert_success!(h.run_entry_function(
            &acc,
            str::parse("0xcafe::pair_type_params::test_pair_enum_and_struct").unwrap(),
            vec![],
            vec![pair.simple_serialize().unwrap()],
        ));
    }
}

// ========================================================================================
// Standalone tests (each uses a unique package)
// ========================================================================================

/// Test that generic container with private type argument is rejected at construction time.
#[test]
fn test_generic_container_with_private_type_arg_rejected() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/negative_generic_private"),
        BuildOptions::move_2().set_latest_language(),
    ));

    let private_point = MoveValue::Struct(MoveStruct::Runtime(vec![
        MoveValue::U64(10),
        MoveValue::U64(20),
    ]));
    let container_value = MoveValue::Struct(MoveStruct::Runtime(vec![private_point]));

    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::negative_generic_private::test_generic_container").unwrap(),
        vec![str::parse("0xcafe::negative_generic_private::PrivatePoint").unwrap()],
        vec![container_value.simple_serialize().unwrap()],
    );
    assert_vm_status!(status, StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE);
}

/// Tests that a public copy struct with an `Option<PrivateT>` field is a valid transaction
/// argument type, illustrating the full flow from extended checker through execution.
#[test]
fn test_option_in_public_struct() {
    let mut h = setup_harness();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_struct_args.data/option_in_wrapper"),
        BuildOptions::move_2().set_latest_language(),
    ));

    let wrapper_none = MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::Vector(vec![])]));

    let hero = MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::U64(42)]));
    let wrapper_some = MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::Vector(vec![hero])]));

    // --- Entry function ---

    // None succeeds
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::option_in_wrapper::check_none").unwrap(),
        vec![],
        vec![wrapper_none.simple_serialize().unwrap()],
    ));

    // Some(Hero) fails
    let status = h.run_entry_function(
        &acc,
        str::parse("0xcafe::option_in_wrapper::check_none").unwrap(),
        vec![],
        vec![wrapper_some.simple_serialize().unwrap()],
    );
    assert_vm_status!(status, StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE);

    // --- View function ---

    // None succeeds
    let res = h.execute_view_function(
        str::parse("0xcafe::option_in_wrapper::check_none_view").unwrap(),
        vec![],
        vec![wrapper_none.simple_serialize().unwrap()],
    );
    assert!(res.values.is_ok());
    let is_none: bool = bcs::from_bytes(&res.values.unwrap()[0]).unwrap();
    assert!(is_none);

    // Some(Hero) fails
    let res = h.execute_view_function(
        str::parse("0xcafe::option_in_wrapper::check_none_view").unwrap(),
        vec![],
        vec![wrapper_some.simple_serialize().unwrap()],
    );
    assert!(res.values.is_err());
}

/// Test that invalid entry function parameter types are rejected at compile time.
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
