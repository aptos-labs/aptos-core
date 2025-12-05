// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_language_e2e_tests::account::Account;
use aptos_types::move_utils::MemberId;
use aptos_vm_environment::prod_configs;
use bcs::to_bytes;
use once_cell::sync::OnceCell;
use std::str::FromStr;

fn ensure_paranoid_ref_checks() {
    static FLAG: OnceCell<()> = OnceCell::new();
    FLAG.get_or_init(|| {
        prod_configs::set_paranoid_ref_checks(true);
    });
}

fn publish_package(h: &mut MoveHarness, account: &Account, relative_path: &str) {
    let path = common::test_dir_path(relative_path);
    assert_success!(h.publish_package_cache_building(account, path.as_path()));
}

#[test]
fn signer_borrow_address_and_move() {
    ensure_paranoid_ref_checks();

    let mut h = MoveHarness::new();
    let aptos_framework_account = h.aptos_framework_account();
    publish_package(
        &mut h,
        &aptos_framework_account,
        "runtime_ref_checks.data/signer",
    );

    let status = h.run_entry_function(
        &aptos_framework_account,
        MemberId::from_str("0x1::cases::borrow_then_move").unwrap(),
        vec![],
        vec![],
    );
    assert_success!(status);
}

#[test]
fn vector_borrow_and_mutate_succeeds() {
    ensure_paranoid_ref_checks();

    let mut h = MoveHarness::new();
    let aptos_framework_account = h.aptos_framework_account();
    publish_package(
        &mut h,
        &aptos_framework_account,
        "runtime_ref_checks.data/vector",
    );

    let status = h.run_entry_function(
        &aptos_framework_account,
        MemberId::from_str("0x1::cases::borrow_read_and_mutate").unwrap(),
        vec![],
        vec![],
    );
    assert_success!(status);
}

#[test]
fn table_borrow_operations_succeed() {
    ensure_paranoid_ref_checks();

    let mut h = MoveHarness::new();
    let aptos_framework_account = h.aptos_framework_account();
    publish_package(
        &mut h,
        &aptos_framework_account,
        "runtime_ref_checks.data/table",
    );

    assert_success!(h.run_entry_function(
        &aptos_framework_account,
        MemberId::from_str("0x1::cases::init").unwrap(),
        vec![],
        vec![],
    ));

    assert_success!(h.run_entry_function(
        &aptos_framework_account,
        MemberId::from_str("0x1::cases::borrow_read").unwrap(),
        vec![],
        vec![to_bytes(&0u64).unwrap(), to_bytes(&41u64).unwrap()],
    ));

    assert_success!(h.run_entry_function(
        &aptos_framework_account,
        MemberId::from_str("0x1::cases::borrow_with_default").unwrap(),
        vec![],
        vec![
            to_bytes(&2u64).unwrap(),
            to_bytes(&999u64).unwrap(),
            to_bytes(&999u64).unwrap(),
        ],
    ));

    assert_success!(h.run_entry_function(
        &aptos_framework_account,
        MemberId::from_str("0x1::cases::borrow_mut_update").unwrap(),
        vec![],
        vec![
            to_bytes(&0u64).unwrap(),
            to_bytes(&1u64).unwrap(),
            to_bytes(&42u64).unwrap(),
        ],
    ));

    assert_success!(h.run_entry_function(
        &aptos_framework_account,
        MemberId::from_str("0x1::cases::upsert_value").unwrap(),
        vec![],
        vec![to_bytes(&2u64).unwrap(), to_bytes(&55u64).unwrap()],
    ));
}
