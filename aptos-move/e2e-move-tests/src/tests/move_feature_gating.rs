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
        BuildOptions::move_2().set_latest_language(),
    );
    if positive_test {
        assert_success!(result);
    } else {
        assert_vm_status!(result, StatusCode::FEATURE_NOT_ENABLED);
    }
}

#[test]
fn function_values() {
    let sources = &[
        r#"
            module 0x815::m {
                fun test(_f: |u64| has drop) {
                }
            }
        "#,
        r#"
            module 0x815::m {
                struct S { f: |u64| }
            }
        "#,
        r#"
            module 0x815::m {
                fun test(): u64 {
                    let f = |x| x + 1;
                    f(2)
                }
            }
        "#,
    ];
    for source in sources {
        let mut builder = PackageBuilder::new("Package");
        builder.add_source("m.move", source);
        let path = builder.write_to_temp().unwrap();
        let mut h = MoveHarness::new();
        let acc = h.new_account_at(AccountAddress::from_hex_literal("0x815").unwrap());
        let result = h.publish_package_with_options(
            &acc,
            path.path(),
            BuildOptions::move_2().set_latest_language(),
        );
        assert_success!(result);
    }
}
