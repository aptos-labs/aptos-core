// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tests for gating of Move language features

// Note: this module uses parameterized tests via the
// [`rstest` crate](https://crates.io/crates/rstest)
// to test for multiple feature combinations.

// Note[Orderless]: Done
use crate::{assert_success, assert_vm_status, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_package_builder::PackageBuilder;
use aptos_types::on_chain_config::FeatureFlag;
use move_core_types::vm_status::StatusCode;
use rstest::rstest;

#[rstest(enabled, disabled, stateless_account, use_txn_payload_v2_format, use_orderless_transactions,
    case(vec![FeatureFlag::ENABLE_ENUM_TYPES], vec![], true, false, false),
    case(vec![FeatureFlag::ENABLE_ENUM_TYPES], vec![], true, true, false),
    case(vec![FeatureFlag::ENABLE_ENUM_TYPES], vec![], true, true, true),
    case(vec![FeatureFlag::ENABLE_ENUM_TYPES], vec![], false, false, false),
    case(vec![FeatureFlag::ENABLE_ENUM_TYPES], vec![], false, true, false),
    case(vec![FeatureFlag::ENABLE_ENUM_TYPES], vec![], false, true, true),
)]
fn enum_types(
    enabled: Vec<FeatureFlag>,
    disabled: Vec<FeatureFlag>,
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let positive_test = !enabled.is_empty();
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    h.enable_features(enabled, disabled);
    let acc = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });

    let mut builder = PackageBuilder::new("Package");
    let source = format!(
        r#"
        module {}::m {{
            enum E {{ Black, White }}
            fun dark(e: E): bool {{
                match (e) {{
                    Black => true,
                    White => false
                }}
            }}
        }}
    "#,
        acc.address()
    );
    builder.add_source("m.move", source.as_str());
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package_with_options(&acc, path.path(), BuildOptions::move_2());
    if positive_test {
        assert_success!(result);
    } else {
        assert_vm_status!(result, StatusCode::FEATURE_NOT_ENABLED)
    }
}

#[rstest(enabled, disabled, stateless_account, use_txn_payload_v2_format, use_orderless_transactions,
    case(vec![FeatureFlag::ENABLE_RESOURCE_ACCESS_CONTROL], vec![], true, false, false),
    case(vec![FeatureFlag::ENABLE_RESOURCE_ACCESS_CONTROL], vec![], true, true, false),
    case(vec![FeatureFlag::ENABLE_RESOURCE_ACCESS_CONTROL], vec![], true, true, true),
    case(vec![FeatureFlag::ENABLE_RESOURCE_ACCESS_CONTROL], vec![], false, false, false),
    case(vec![FeatureFlag::ENABLE_RESOURCE_ACCESS_CONTROL], vec![], false, true, false),
    case(vec![FeatureFlag::ENABLE_RESOURCE_ACCESS_CONTROL], vec![], false, true, true),

    case(vec![], vec![FeatureFlag::ENABLE_RESOURCE_ACCESS_CONTROL], true, false, false),
    case(vec![], vec![FeatureFlag::ENABLE_RESOURCE_ACCESS_CONTROL], true, true, false),
    case(vec![], vec![FeatureFlag::ENABLE_RESOURCE_ACCESS_CONTROL], true, true, true),
    case(vec![], vec![FeatureFlag::ENABLE_RESOURCE_ACCESS_CONTROL], false, false, false),
    case(vec![], vec![FeatureFlag::ENABLE_RESOURCE_ACCESS_CONTROL], false, true, false),
    case(vec![], vec![FeatureFlag::ENABLE_RESOURCE_ACCESS_CONTROL], false, true, true),
)]
fn resource_access_control(
    enabled: Vec<FeatureFlag>,
    disabled: Vec<FeatureFlag>,
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let positive_test = !enabled.is_empty();
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    h.enable_features(enabled, disabled);
    let acc = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });

    let mut builder = PackageBuilder::new("Package");
    let source = format!(
        r#"
        module {}::m {{
            struct R has key, copy {{ }}
            fun read(a: address): R reads R {{
                *borrow_global<R>(a)
            }}
        }}
    "#,
        acc.address()
    );
    builder.add_source("m.move", source.as_str());
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

#[rstest(enabled, disabled, stateless_account, use_txn_payload_v2_format, use_orderless_transactions,
    case(vec![FeatureFlag::ENABLE_FUNCTION_VALUES], vec![], true, false, false),
    case(vec![FeatureFlag::ENABLE_FUNCTION_VALUES], vec![], true, true, false),
    case(vec![FeatureFlag::ENABLE_FUNCTION_VALUES], vec![], true, true, true),
    case(vec![FeatureFlag::ENABLE_FUNCTION_VALUES], vec![], false, false, false),
    case(vec![FeatureFlag::ENABLE_FUNCTION_VALUES], vec![], false, true, false),
    case(vec![FeatureFlag::ENABLE_FUNCTION_VALUES], vec![], false, true, true),

    case(vec![], vec![FeatureFlag::ENABLE_FUNCTION_VALUES], true, false, false),
    case(vec![], vec![FeatureFlag::ENABLE_FUNCTION_VALUES], true, true, false),
    case(vec![], vec![FeatureFlag::ENABLE_FUNCTION_VALUES], true, true, true),
    case(vec![], vec![FeatureFlag::ENABLE_FUNCTION_VALUES], false, false, false),
    case(vec![], vec![FeatureFlag::ENABLE_FUNCTION_VALUES], false, true, false),
    case(vec![], vec![FeatureFlag::ENABLE_FUNCTION_VALUES], false, true, true),
)]
fn function_values(
    enabled: Vec<FeatureFlag>,
    disabled: Vec<FeatureFlag>,
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let positive_test = !enabled.is_empty();
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    h.enable_features(enabled.clone(), disabled.clone());
    let accounts: Vec<_> = (0..3)
        .map(|_| h.new_account_with_key_pair(if stateless_account { None } else { Some(0) }))
        .collect();

    let sources = &[
        format!(
            r#"
            module {}::m {{
                fun test(_f: |u64| has drop) {{
                }}
            }}
        "#,
            accounts[0].address()
        ),
        format!(
            r#"
            module {}::m {{
                struct S {{ f: |u64| }}
            }}
        "#,
            accounts[1].address()
        ),
        format!(
            r#"
            module {}::m {{
                fun test(): u64 {{
                    let f = |x| x + 1;
                    f(2)
                }}
            }}
        "#,
            accounts[2].address()
        ),
    ];
    for (i, source) in sources.iter().enumerate() {
        let mut builder = PackageBuilder::new("Package");
        builder.add_source("m.move", source);
        let path = builder.write_to_temp().unwrap();
        let result = h.publish_package_with_options(
            &accounts[i],
            path.path(),
            BuildOptions::move_2().set_latest_language(),
        );
        if positive_test {
            assert_success!(result);
        } else {
            assert_vm_status!(result, StatusCode::FEATURE_NOT_ENABLED);
        }
    }
}
