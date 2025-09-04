// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tests for enum type upgrade compatibility

use crate::{assert_success, assert_vm_status, MoveHarness};
use velor_framework::BuildOptions;
use velor_language_e2e_tests::account::Account;
use velor_package_builder::PackageBuilder;
use velor_types::{account_address::AccountAddress, transaction::TransactionStatus};
use move_core_types::vm_status::StatusCode;

#[test]
fn enum_upgrade() {
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
            enum Data {
               V1{x: u64},
               V2{x: u64, y: u8},
            }
        }
    "#,
    );
    assert_success!(result);

    // Upgrade identity
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
    assert_success!(result);

    // Incompatible because of modification
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m {
            enum Data {
               V1{x: u64, z: u32},
               V2{x: u64, y: u8},
            }
        }
    "#,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    // Incompatible because of removal
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m {
            enum Data {
               V2{x: u64, y: u8},
            }
        }
    "#,
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);

    // Incompatible because of renaming
    let result = publish(
        &mut h,
        &acc,
        r#"
        module 0x815::m {
           enum Data {
               V1{x: u64},
               V2a{x: u64, y: u8},
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
    h.publish_package_with_options(account, path.path(), BuildOptions::move_2())
}
