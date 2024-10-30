// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tests for enum type upgrade compatibility

// Note[Orderless]: Done
use crate::{assert_success, assert_vm_status, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_language_e2e_tests::account::Account;
use aptos_package_builder::PackageBuilder;
use aptos_types::transaction::TransactionStatus;
use move_core_types::vm_status::StatusCode;
use rstest::rstest;

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn enum_upgrade(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let acc = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    // Initial publish
    let result = publish(
        &mut h,
        &acc,
        format!(
            r#"
            module {}::m {{
                enum Data {{
                V1{{x: u64}}
            }}
        }}
    "#,
            acc.address()
        )
        .as_str(),
    );
    assert_success!(result);

    // Add a compatible variant
    let result = publish(
        &mut h,
        &acc,
        format!(
            r#"
        module {}::m {{
            enum Data {{
               V1{{x: u64}},
               V2{{x: u64, y: u8}},
            }}
        }}
    "#,
            acc.address()
        )
        .as_str(),
    );
    assert_success!(result);

    // Upgrade identity
    let result = publish(
        &mut h,
        &acc,
        format!(
            r#"
        module {}::m {{
            enum Data {{
               V1{{x: u64}},
               V2{{x: u64, y: u8}},
            }}
        }}
    "#,
            acc.address()
        )
        .as_str(),
    );
    assert_success!(result);

    // Incompatible because of modification
    let result = publish(
        &mut h,
        &acc,
        format!(
            r#"
        module {}::m {{
            enum Data {{
               V1{{x: u64, z: u32}},
               V2{{x: u64, y: u8}},
            }}
        }}
    "#,
            acc.address()
        )
        .as_str(),
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    // Incompatible because of removal
    let result = publish(
        &mut h,
        &acc,
        format!(
            r#"
        module {}::m {{
            enum Data {{
               V2{{x: u64, y: u8}},
            }}
        }}
    "#,
            acc.address()
        )
        .as_str(),
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    // Incompatible because of renaming
    let result = publish(
        &mut h,
        &acc,
        format!(
            r#"
        module {}::m {{
           enum Data {{
               V1{{x: u64}},
               V2a{{x: u64, y: u8}},
            }}
        }}
    "#,
            acc.address()
        )
        .as_str(),
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);
}

fn publish(h: &mut MoveHarness, account: &Account, source: &str) -> TransactionStatus {
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    h.publish_package_with_options(account, path.path(), BuildOptions::move_2())
}
