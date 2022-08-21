// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{account_address::AccountAddress, vm_status::StatusCode};
use e2e_move_tests::{assert_success, assert_vm_status, MoveHarness};
use serde::{Deserialize, Serialize};

mod common;

/// Mimics `0xcafe::test::ModuleData`
#[derive(Serialize, Deserialize)]
struct ModuleData {
    state: Vec<u8>,
}

#[test]
fn module_loop_depth_at_limit() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("max_loop_depth.data/pack-good"),
    ));
}

#[test]
fn module_loop_depth_just_above_limit() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    assert_vm_status!(
        h.publish_package(&acc, &common::test_dir_path("max_loop_depth.data/pack-bad"),),
        StatusCode::LOOP_MAX_DEPTH_REACHED
    );
}
