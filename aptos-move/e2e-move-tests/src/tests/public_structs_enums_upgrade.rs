// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for upgrade compatibility of public/package structs/enums

use crate::{assert_abort, assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_language_e2e_tests::account::Account;
use aptos_package_builder::PackageBuilder;
use aptos_types::{account_address::AccountAddress, transaction::TransactionStatus};
use move_core_types::{parser::parse_struct_tag, vm_status::StatusCode};
use move_model::well_known::INCOMPLETE_MATCH_ABORT_CODE;
use serde::{Deserialize, Serialize};

#[test]
fn public_enum_upgrade() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x815").unwrap());

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m {
            enum Data {
               V1{x: u64}
            }
        }
    "#,
    );
    assert_success!(result);

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m {
            public enum Data {
               V1{x: u64},
            }
        }
    "#,
    );
    assert_success!(result);

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m {
            public enum Data {
               V1{x: u64},
               V2{x: u64, y: u8},
            }
        }
    "#,
    );
    assert_success!(result);

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m {
            enum Data {
               V1{x: u64},
               V2{x: u64, y: u8},
            }
        }
    "#,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m {
            package enum Data {
               V1{x: u64},
               V2{x: u64, y: u8},
            }
        }
    "#,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);
}

#[test]
fn friend_enum_struct_upgrade() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x815").unwrap());

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m {
            friend 0x815::m2;
            enum Data {
               V1{x: u64}
            }
        }
        module 0x815::m2 {
        }
    "#,
    );
    assert_success!(result);

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m2 {
            use 0x815::m::Data;
            fun get_data(): Data {
                Data::V2{x: 1, y: 2}
            }
        }
        module 0x815::m {
            friend 0x815::m2;
            friend enum Data {
               V1{x: u64},
               V2{x: u64, y: u8},
            }
        }
    "#,
    );
    assert_success!(result);

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m2 {
        }
        module 0x815::m {
            enum Data {
               V1{x: u64},
               V2{x: u64, y: u8},
            }
        }
    "#,
    );
    assert_success!(result);

    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m2 {
            use 0x815::m::Data;
            use 0x815::m::S;
            fun get_data(): Data {
                Data::V1{x: 1}
            }
            fun pack_struct(): S {
                S { x: 22 }
            }
        }
        module 0x815::m {
            package enum Data {
               V1{x: u64},
               V2{x: u64, y: u8},
            }

            package struct S {
                x: u64,
            }
        }
    "#,
    );
    assert_success!(result);

    let result = publish(
        &mut h,
        &acc,
        r#"
            module 0x815::m2 {
                use 0x815::m::Data;
                use 0x815::m::S;
                use 0x815::m::Predicate;
                use 0x815::m::Holder;
                fun get_data(): Data {
                    Data::V1{x: 1}
                }
                public fun pack_struct(): S {
                S { x: 22 }
                }

                public fun apply_pred(p: Predicate<u64>, val: u64): bool {
                    p(&val)
                }

                public fun apply_holder(h: Holder<u64>, val: u64): bool {
                    let Holder(p) = h;
                    p(&val)
                }
            }
            module 0x815::m {
                public enum Data {
                   V1{x: u64},
                   V2{x: u64, y: u8},
                }

                public struct S {
                    x: u64,
                }

                package struct Predicate<T>(|&T| bool) has copy, drop;

                package struct Holder<T>(Predicate<T>) has copy, drop;
            }
        "#,
    );
    assert_success!(result);
}

fn publish(h: &mut MoveHarness, account: &Account, source: &str) -> TransactionStatus {
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);

    builder.add_local_dep(
        "MoveStdlib",
        &common::framework_dir_path("move-stdlib").to_string_lossy(),
    );

    let path = builder.write_to_temp().unwrap();
    h.publish_package_with_options(
        account,
        path.path(),
        BuildOptions::move_2().set_latest_language(),
    )
}

/// Mimics `0x815::ops::Result` resource
#[derive(Serialize, Deserialize, Debug)]
struct Result {
    value: u64,
}

#[test]
fn execute_cross_module_public_struct() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x815").unwrap());

    // Publish two modules: types (defines structs) and ops (uses them)
    let result = publish(
        &mut h,
        &acc,
        r#"
        // Module 1: Define public struct
        module 0x815::types {
            public struct Point has copy, drop {
                x: u64,
                y: u64,
            }
        }

        // Module 2: Use public struct from another module
        module 0x815::ops {
            use std::signer;
            use 0x815::types::Point;

            struct Result has key {
                value: u64,
            }

            // Pack Point in a different module than where it's defined
            public entry fun test_cross_module_pack(account: &signer, x: u64, y: u64) {
                let p = Point { x, y };
                let Point { x, y } = p;
                move_to(account, Result { value: x + y });
            }

            // Borrow fields across modules
            public entry fun test_cross_module_borrow(account: &signer, x: u64, y: u64) {
                let p = Point { x, y };
                let x_ref = &p.x;
                let y_ref = &p.y;
                move_to(account, Result { value: *x_ref + *y_ref });
            }

            // Mutable borrow across modules - we unpack, modify, and repack
            public entry fun test_cross_module_mut_borrow(account: &signer, x: u64, y: u64, delta: u64) {
                let p = Point { x, y };
                let Point { x: old_x, y } = p;
                let new_x = old_x + delta;
                move_to(account, Result { value: new_x + y });
            }

            // Multiple struct operations across modules
            public entry fun test_cross_module_multiple(account: &signer, x1: u64, y1: u64, x2: u64, y2: u64) {
                let p1 = Point { x: x1, y: y1 };
                let p2 = Point { x: x2, y: y2 };
                let Point { x: a, y: b } = p1;
                let Point { x: c, y: d } = p2;
                move_to(account, Result { value: a + b + c + d });
            }
        }
        "#,
    );
    assert_success!(result);

    let result_tag = parse_struct_tag("0x815::ops::Result").unwrap();

    // Test 1: Cross-module pack/unpack
    let acc1 = h.new_account_at(AccountAddress::from_hex_literal("0x816").unwrap());
    let result = h.run_entry_function(
        &acc1,
        str::parse("0x815::ops::test_cross_module_pack").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&10u64).unwrap(),
            bcs::to_bytes(&20u64).unwrap(),
        ],
    );
    assert_success!(result);

    let stored_result = h
        .read_resource::<Result>(acc1.address(), result_tag.clone())
        .unwrap();
    assert_eq!(stored_result.value, 30); // 10 + 20

    // Test 2: Cross-module borrow
    let acc2 = h.new_account_at(AccountAddress::from_hex_literal("0x817").unwrap());
    let result = h.run_entry_function(
        &acc2,
        str::parse("0x815::ops::test_cross_module_borrow").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&100u64).unwrap(),
            bcs::to_bytes(&200u64).unwrap(),
        ],
    );
    assert_success!(result);

    let stored_result = h
        .read_resource::<Result>(acc2.address(), result_tag.clone())
        .unwrap();
    assert_eq!(stored_result.value, 300); // 100 + 200

    // Test 3: Cross-module mutable borrow
    let acc3 = h.new_account_at(AccountAddress::from_hex_literal("0x818").unwrap());
    let result = h.run_entry_function(
        &acc3,
        str::parse("0x815::ops::test_cross_module_mut_borrow").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&50u64).unwrap(),
            bcs::to_bytes(&30u64).unwrap(),
            bcs::to_bytes(&20u64).unwrap(),
        ],
    );
    assert_success!(result);

    let stored_result = h
        .read_resource::<Result>(acc3.address(), result_tag.clone())
        .unwrap();
    assert_eq!(stored_result.value, 100); // (50 + 20) + 30

    // Test 4: Multiple cross-module operations
    let acc4 = h.new_account_at(AccountAddress::from_hex_literal("0x819").unwrap());
    let result = h.run_entry_function(
        &acc4,
        str::parse("0x815::ops::test_cross_module_multiple").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&5u64).unwrap(),
            bcs::to_bytes(&10u64).unwrap(),
            bcs::to_bytes(&15u64).unwrap(),
            bcs::to_bytes(&20u64).unwrap(),
        ],
    );
    assert_success!(result);

    let stored_result = h
        .read_resource::<Result>(acc4.address(), result_tag)
        .unwrap();
    assert_eq!(stored_result.value, 50); // 5 + 10 + 15 + 20
}

#[test]
fn execute_cross_module_public_enum() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x815").unwrap());

    // Publish two modules: types (defines enum) and ops (uses it)
    let result = publish(
        &mut h,
        &acc,
        r#"
        // Module 1: Define public enum
        module 0x815::types {
            public enum Color has copy, drop {
                Red { r: u8 },
                Green { g: u16 },
                Blue { b: u8 },
            }
        }

        // Module 2: Use public enum from another module
        module 0x815::ops {
            use std::signer;
            use 0x815::types::Color;

            struct Result has key {
                value: u64,
            }

            // Pack Red variant in a different module
            public entry fun test_cross_module_red(account: &signer, r: u8) {
                let c = Color::Red { r };
                let val = match (c) {
                    Color::Red { r } => (r as u64),
                    Color::Green { g } => (g as u64),
                    Color::Blue { b } => (b as u64),
                };
                move_to(account, Result { value: val });
            }

            // Pack Green variant across modules
            public entry fun test_cross_module_green(account: &signer, g: u16) {
                let c = Color::Green { g };
                let val = match (c) {
                    Color::Red { r } => (r as u64),
                    Color::Green { g } => (g as u64),
                    Color::Blue { b } => (b as u64),
                };
                move_to(account, Result { value: val });
            }

            // Pack Blue variant across modules
            public entry fun test_cross_module_blue(account: &signer, b: u8) {
                let c = Color::Blue { b };
                let val = match (c) {
                    Color::Red { r } => (r as u64),
                    Color::Green { g } => (g as u64),
                    Color::Blue { b } => (b as u64),
                };
                move_to(account, Result { value: val });
            }

            // Mix multiple variants across modules
            public entry fun test_cross_module_mix(account: &signer, r: u8, g: u16, b: u8) {
                let c1 = Color::Red { r };
                let c2 = Color::Green { g };
                let c3 = Color::Blue { b };

                let v1 = match (c1) {
                    Color::Red { r } => (r as u64),
                    Color::Green { g } => (g as u64),
                    Color::Blue { b } => (b as u64),
                };
                let v2 = match (c2) {
                    Color::Red { r } => (r as u64),
                    Color::Green { g } => (g as u64),
                    Color::Blue { b } => (b as u64),
                };
                let v3 = match (c3) {
                    Color::Red { r } => (r as u64),
                    Color::Green { g } => (g as u64),
                    Color::Blue { b } => (b as u64),
                };

                move_to(account, Result { value: v1 + v2 + v3 });
            }

            // Test variant across modules using pattern matching
            public entry fun test_cross_module_variant_match(account: &signer, r: u8, delta: u8) {
                let c = Color::Red { r };
                // Extract value through pattern matching and transform it
                let val = match (c) {
                    Color::Red { r } => ((r + delta) as u64),
                    Color::Green { g } => (g as u64),
                    Color::Blue { b } => (b as u64),
                };
                move_to(account, Result { value: val });
            }
        }
        "#,
    );
    assert_success!(result);

    let result_tag = parse_struct_tag("0x815::ops::Result").unwrap();

    // Test 1: Cross-module Red variant
    let acc1 = h.new_account_at(AccountAddress::from_hex_literal("0x816").unwrap());
    let result = h.run_entry_function(
        &acc1,
        str::parse("0x815::ops::test_cross_module_red").unwrap(),
        vec![],
        vec![bcs::to_bytes(&255u8).unwrap()],
    );
    assert_success!(result);

    let stored_result = h
        .read_resource::<Result>(acc1.address(), result_tag.clone())
        .unwrap();
    assert_eq!(stored_result.value, 255);

    // Test 2: Cross-module Green variant
    let acc2 = h.new_account_at(AccountAddress::from_hex_literal("0x817").unwrap());
    let result = h.run_entry_function(
        &acc2,
        str::parse("0x815::ops::test_cross_module_green").unwrap(),
        vec![],
        vec![bcs::to_bytes(&1000u16).unwrap()],
    );
    assert_success!(result);

    let stored_result = h
        .read_resource::<Result>(acc2.address(), result_tag.clone())
        .unwrap();
    assert_eq!(stored_result.value, 1000);

    // Test 3: Cross-module Blue variant
    let acc3 = h.new_account_at(AccountAddress::from_hex_literal("0x818").unwrap());
    let result = h.run_entry_function(
        &acc3,
        str::parse("0x815::ops::test_cross_module_blue").unwrap(),
        vec![],
        vec![bcs::to_bytes(&128u8).unwrap()],
    );
    assert_success!(result);

    let stored_result = h
        .read_resource::<Result>(acc3.address(), result_tag.clone())
        .unwrap();
    assert_eq!(stored_result.value, 128);

    // Test 4: Cross-module mix variants
    let acc4 = h.new_account_at(AccountAddress::from_hex_literal("0x819").unwrap());
    let result = h.run_entry_function(
        &acc4,
        str::parse("0x815::ops::test_cross_module_mix").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&50u8).unwrap(),
            bcs::to_bytes(&100u16).unwrap(),
            bcs::to_bytes(&25u8).unwrap(),
        ],
    );
    assert_success!(result);

    let stored_result = h
        .read_resource::<Result>(acc4.address(), result_tag.clone())
        .unwrap();
    assert_eq!(stored_result.value, 175); // 50 + 100 + 25

    // Test 5: Cross-module variant pattern matching and transformation
    let acc5 = h.new_account_at(AccountAddress::from_hex_literal("0x81a").unwrap());
    let result = h.run_entry_function(
        &acc5,
        str::parse("0x815::ops::test_cross_module_variant_match").unwrap(),
        vec![],
        vec![bcs::to_bytes(&10u8).unwrap(), bcs::to_bytes(&15u8).unwrap()],
    );
    assert_success!(result);

    let stored_result = h
        .read_resource::<Result>(acc5.address(), result_tag)
        .unwrap();
    assert_eq!(stored_result.value, 25); // 10 + 15
}

#[test]
fn execute_cross_module_nested_structs() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x815").unwrap());

    // Publish modules with nested public structs
    let result = publish(
        &mut h,
        &acc,
        r#"
        // Module 1: Define public structs
        module 0x815::types {
            public struct Point has copy, drop {
                x: u64,
                y: u64,
            }

            public struct Line has copy, drop {
                start: Point,
                end: Point,
            }
        }

        // Module 2: Use nested public structs from another module
        module 0x815::ops {
            use std::signer;
            use 0x815::types::{Point, Line};

            struct Result has key {
                value: u64,
            }

            // Create and unpack nested structs across modules
            public entry fun test_cross_module_nested(account: &signer, x1: u64, y1: u64, x2: u64, y2: u64) {
                let p1 = Point { x: x1, y: y1 };
                let p2 = Point { x: x2, y: y2 };
                let line = Line { start: p1, end: p2 };

                let Line { start, end } = line;
                let Point { x: a, y: b } = start;
                let Point { x: c, y: d } = end;

                move_to(account, Result { value: a + b + c + d });
            }
        }
        "#,
    );
    assert_success!(result);

    let result_tag = parse_struct_tag("0x815::ops::Result").unwrap();

    let result = h.run_entry_function(
        &acc,
        str::parse("0x815::ops::test_cross_module_nested").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&10u64).unwrap(),
            bcs::to_bytes(&20u64).unwrap(),
            bcs::to_bytes(&30u64).unwrap(),
            bcs::to_bytes(&40u64).unwrap(),
        ],
    );
    assert_success!(result);

    let stored_result = h
        .read_resource::<Result>(acc.address(), result_tag)
        .unwrap();
    assert_eq!(stored_result.value, 100); // 10 + 20 + 30 + 40
}

#[test]
fn execute_cross_module_generic_struct() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x815").unwrap());

    // Publish modules with generic public struct
    let result = publish(
        &mut h,
        &acc,
        r#"
        // Module 1: Define generic public struct
        module 0x815::types {
            public struct Box<T> has copy, drop {
                value: T,
            }
        }

        // Module 2: Use generic public struct from another module
        module 0x815::ops {
            use std::signer;
            use 0x815::types::Box;

            struct Result has key {
                value: u64,
            }

            // Create and unpack generic Box<u64> across modules
            public entry fun test_cross_module_generic_u64(account: &signer, val: u64) {
                let b = Box<u64> { value: val };
                let Box { value } = b;
                move_to(account, Result { value });
            }

            // Create and unpack generic Box<u128> across modules
            public entry fun test_cross_module_generic_u128(account: &signer, val: u128) {
                let b = Box<u128> { value: val };
                let Box { value } = b;
                move_to(account, Result { value: (value as u64) });
            }
        }
        "#,
    );
    assert_success!(result);

    let result_tag = parse_struct_tag("0x815::ops::Result").unwrap();

    // Test 1: Box<u64> across modules
    let acc1 = h.new_account_at(AccountAddress::from_hex_literal("0x816").unwrap());
    let result = h.run_entry_function(
        &acc1,
        str::parse("0x815::ops::test_cross_module_generic_u64").unwrap(),
        vec![],
        vec![bcs::to_bytes(&12345u64).unwrap()],
    );
    assert_success!(result);

    let stored_result = h
        .read_resource::<Result>(acc1.address(), result_tag.clone())
        .unwrap();
    assert_eq!(stored_result.value, 12345);

    // Test 2: Box<u128> across modules
    let acc2 = h.new_account_at(AccountAddress::from_hex_literal("0x817").unwrap());
    let result = h.run_entry_function(
        &acc2,
        str::parse("0x815::ops::test_cross_module_generic_u128").unwrap(),
        vec![],
        vec![bcs::to_bytes(&99999u128).unwrap()],
    );
    assert_success!(result);

    let stored_result = h
        .read_resource::<Result>(acc2.address(), result_tag)
        .unwrap();
    assert_eq!(stored_result.value, 99999);
}

#[test]
fn execute_cross_module_vector_of_structs() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x815").unwrap());

    // Publish modules that use vector of public structs
    let result = publish(
        &mut h,
        &acc,
        r#"
        // Module 1: Define public struct
        module 0x815::types {
            public struct Point has copy, drop {
                x: u64,
                y: u64,
            }
        }

        // Module 2: Create and process vector of public structs from another module
        module 0x815::ops {
            use std::signer;
            use std::vector;
            use 0x815::types::Point;

            struct Result has key {
                value: u64,
            }

            // Create vector of Point structs from another module
            public entry fun test_cross_module_vector(account: &signer, count: u64) {
                let points = vector::empty<Point>();
                let i = 0;
                while (i < count) {
                    vector::push_back(&mut points, Point { x: i, y: i + 1 });
                    i = i + 1;
                };

                // Sum all points
                let sum = 0u64;
                let j = 0;
                let len = vector::length(&points);
                while (j < len) {
                    let p = vector::borrow(&points, j);
                    sum = sum + p.x + p.y;
                    j = j + 1;
                };

                move_to(account, Result { value: sum });
            }
        }
        "#,
    );
    assert_success!(result);

    let result_tag = parse_struct_tag("0x815::ops::Result").unwrap();

    // Create 3 points: (0,1), (1,2), (2,3)
    // Expected sum: 0 + 1 + 1 + 2 + 2 + 3 = 9
    let result = h.run_entry_function(
        &acc,
        str::parse("0x815::ops::test_cross_module_vector").unwrap(),
        vec![],
        vec![bcs::to_bytes(&3u64).unwrap()],
    );
    assert_success!(result);

    let stored_result = h
        .read_resource::<Result>(acc.address(), result_tag)
        .unwrap();
    assert_eq!(stored_result.value, 9); // 0+1 + 1+2 + 2+3
}

#[test]
fn execute_cross_module_enum_new_variant_handling() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x815").unwrap());

    // Enum with 2 variants, ops module uses exhaustive match
    let status = h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_enum.data/test_package"),
        BuildOptions::move_2().set_latest_language(),
    );
    assert_success!(status);

    // Publish ops, which uses the enum from the test_package
    let acc2 = h.new_account_at(AccountAddress::from_hex_literal("0x816").unwrap());
    let status = h.publish_package_with_options(
        &acc2,
        &common::test_dir_path("public_enum.data/ops"),
        BuildOptions::move_2().set_latest_language(),
    );
    assert_success!(status);

    // Expect success when running the entry function `check`
    let result = h.run_entry_function(
        &acc2,
        str::parse("0x816::ops::check").unwrap(),
        vec![],
        vec![bcs::to_bytes(&2u8).unwrap(), bcs::to_bytes(&2u64).unwrap()],
    );
    assert_success!(result);

    // Upgrade enum with new Blue variant
    let status = h.publish_package_with_options(
        &acc,
        &common::test_dir_path("public_enum.data/upgraded_test_package"),
        BuildOptions::move_2().set_latest_language(),
    );
    assert_success!(status);

    // ops module will abort because it doesn't know about the new Blue variant
    let result = h.run_entry_function(
        &acc2,
        str::parse("0x816::ops::check").unwrap(),
        vec![],
        vec![bcs::to_bytes(&2u8).unwrap(), bcs::to_bytes(&2u64).unwrap()],
    );
    assert_abort!(result, INCOMPLETE_MATCH_ABORT_CODE);
}
