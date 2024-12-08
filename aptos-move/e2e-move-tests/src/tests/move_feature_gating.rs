// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tests for gating of Move language features

// Note: this module uses parameterized tests via the
// [`rstest` crate](https://crates.io/crates/rstest)
// to test for multiple feature combinations.

use crate::{assert_success, assert_vm_status, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_package_builder::PackageBuilder;
use aptos_types::{account_address::AccountAddress, on_chain_config::FeatureFlag};
use move_core_types::vm_status::StatusCode;
use rstest::rstest;

#[rstest(enabled, disabled,
    case(vec![], vec![FeatureFlag::ENABLE_ENUM_TYPES]),
    case(vec![FeatureFlag::ENABLE_ENUM_TYPES], vec![]),
)]
fn enum_types(enabled: Vec<FeatureFlag>, disabled: Vec<FeatureFlag>) {
    let positive_test = !enabled.is_empty();
    let mut h = MoveHarness::new_with_features(enabled, disabled);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x815").unwrap());

    let mut builder = PackageBuilder::new("Package");
    let source = r#"
        module 0x815::m {
            enum E { Black, White }
            fun dark(e: E): bool {
                match (e) {
                    Black => true,
                    White => false
                }
            }
        }
    "#;
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package_with_options(&acc, path.path(), BuildOptions::move_2());
    if positive_test {
        assert_success!(result);
    } else {
        assert_vm_status!(result, StatusCode::FEATURE_NOT_ENABLED)
    }
}

#[rstest(enabled, disabled,
    case(vec![], vec![FeatureFlag::ENABLE_RESOURCE_ACCESS_CONTROL]),
    case(vec![FeatureFlag::ENABLE_RESOURCE_ACCESS_CONTROL], vec![]),
)]
fn resource_access_control(enabled: Vec<FeatureFlag>, disabled: Vec<FeatureFlag>) {
    let positive_test = !enabled.is_empty();
    let mut h = MoveHarness::new_with_features(enabled, disabled);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x815").unwrap());

    let mut builder = PackageBuilder::new("Package");
    let source = r#"
        module 0x815::m {
            struct R has key, copy {}
            fun read(a: address): R reads R {
                *borrow_global<R>(a)
            }
        }
    "#;
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package_with_options(
        &acc,
        path.path(),
        BuildOptions::move_2().with_experiment("gen-access-specifiers"),
    );
    if positive_test {
        assert_success!(result);
    } else {
        assert_vm_status!(result, StatusCode::FEATURE_NOT_ENABLED);
    }
}

fn lambda_build_options() -> BuildOptions {
    BuildOptions::move_2_2()
        .with_experiment("lambda-fields")
        .with_experiment("lambda-in-params")
        .with_experiment("lambda-in-returns")
        .with_experiment("lambda-lifting")
        .with_experiment("lambda-values")
}

#[rstest(enabled, disabled,
         case(vec![], vec![FeatureFlag::ENABLE_FUNCTION_VALUES]),
         case(vec![FeatureFlag::ENABLE_FUNCTION_VALUES], vec![]),
)]
fn function_types_only(enabled: Vec<FeatureFlag>, disabled: Vec<FeatureFlag>) {
    let positive_test = !enabled.is_empty();
    let mut h = MoveHarness::new_with_features(enabled, disabled);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x815").unwrap());

    let mut builder = PackageBuilder::new("Package");
    let source = r#"
        module 0x815::m {
            public fun fn_id(f: |u64|u64 with copy): |u64|u64 with copy {
                f
            }
        }
    "#;
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package_with_options(&acc, path.path(), lambda_build_options());
    if positive_test {
        assert_success!(result);
    } else {
        assert_vm_status!(result, StatusCode::FEATURE_NOT_ENABLED)
    }
}

#[rstest(enabled, disabled,
    case(vec![], vec![FeatureFlag::ENABLE_FUNCTION_VALUES]),
         case(vec![FeatureFlag::ENABLE_FUNCTION_VALUES], vec![]),
)]
fn function_values_apply_only(enabled: Vec<FeatureFlag>, disabled: Vec<FeatureFlag>) {
    let positive_test = !enabled.is_empty();
    let mut h = MoveHarness::new_with_features(enabled, disabled);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x815").unwrap());

    let mut builder = PackageBuilder::new("Package");
    let source = r#"
        module 0x815::m {
            public fun map(f: |u64|u64 with copy, x: u64): u64 {
                f(x)
            }
        }
    "#;
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package_with_options(&acc, path.path(), lambda_build_options());
    if positive_test {
        assert_success!(result);
    } else {
        assert_vm_status!(result, StatusCode::FEATURE_NOT_ENABLED)
    }
}

#[rstest(enabled, disabled,
    case(vec![], vec![FeatureFlag::ENABLE_FUNCTION_VALUES]),
          case(vec![FeatureFlag::ENABLE_FUNCTION_VALUES], vec![]),
)]
fn function_values_create_only(enabled: Vec<FeatureFlag>, disabled: Vec<FeatureFlag>) {
    let positive_test = !enabled.is_empty();
    let mut h = MoveHarness::new_with_features(enabled, disabled);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x815").unwrap());

    let mut builder = PackageBuilder::new("Package");
    let source = r#"
        module 0x815::m {
            public fun add_func(x: u64, y: u64): u64 {
                x + y
            }
            public fun build_function(): |u64, u64|u64 with copy+store {
                add_func
            }
        }
    "#;
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package_with_options(&acc, path.path(), lambda_build_options());
    if positive_test {
        assert_success!(result);
    } else {
        assert_vm_status!(result, StatusCode::FEATURE_NOT_ENABLED)
    }
}

#[rstest(enabled, disabled,
         case(vec![], vec![FeatureFlag::ENABLE_FUNCTION_VALUES]),
          case(vec![FeatureFlag::ENABLE_FUNCTION_VALUES], vec![]),
)]
fn function_values_early_bind_only(enabled: Vec<FeatureFlag>, disabled: Vec<FeatureFlag>) {
    let positive_test = !enabled.is_empty();
    let mut h = MoveHarness::new_with_features(enabled, disabled);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x815").unwrap());

    let mut builder = PackageBuilder::new("Package");
    let source = r#"
        module 0x815::m {
            public fun add_func(x: u64, y: u64): u64 {
                x + y
            }
            public fun build_function(x: u64): |u64|u64 with copy+store {
                let f = move |y| add_func(x, y);
                f
            }
        }
    "#;
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package_with_options(&acc, path.path(), lambda_build_options());
    if positive_test {
        assert_success!(result);
    } else {
        assert_vm_status!(result, StatusCode::FEATURE_NOT_ENABLED)
    }
}

#[rstest(enabled, disabled,
    case(vec![], vec![FeatureFlag::ENABLE_FUNCTION_VALUES]),
         case(vec![FeatureFlag::ENABLE_FUNCTION_VALUES], vec![]),
)]
fn function_values(enabled: Vec<FeatureFlag>, disabled: Vec<FeatureFlag>) {
    let positive_test = !enabled.is_empty();
    let mut h = MoveHarness::new_with_features(enabled, disabled);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x815").unwrap());

    let mut builder = PackageBuilder::new("Package");
    let source = r#"
        module 0x815::m {
            fun map(f: |u64|u64 with copy, x: u64): u64 {
                f(x)
            }
            public fun add_func(x: u64, y: u64): u64 {
                x + y
            }
            fun build_function(x: u64): |u64|u64 with copy+store {
                let f = move |y| add_func(x, y);
                f
            }
            public fun main(x: u64): u64 {
                let g = build_function(x);
                map(g, 3)
            }
        }
    "#;
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package_with_options(&acc, path.path(), lambda_build_options());
    if positive_test {
        assert_success!(result);
    } else {
        assert_vm_status!(result, StatusCode::FEATURE_NOT_ENABLED)
    }
}
