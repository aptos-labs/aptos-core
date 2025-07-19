// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tests for enum type upgrade compatibility

use crate::{assert_success, assert_vm_status, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_language_e2e_tests::account::Account;
use aptos_package_builder::PackageBuilder;
use aptos_types::{account_address::AccountAddress, transaction::TransactionStatus};
use move_core_types::vm_status::StatusCode;

#[test]
fn public_enum_upgrade() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x815").unwrap());

    // Initial publish
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

    // Add a compatible variant
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

    // Incompatible because of removing `public`
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

fn publish(h: &mut MoveHarness, account: &Account, source: &str) -> TransactionStatus {
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    h.publish_package_with_options(
        account,
        path.path(),
        BuildOptions::move_2().set_latest_language(),
    )
}
