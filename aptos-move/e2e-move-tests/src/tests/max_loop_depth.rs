// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_types::{account_address::AccountAddress, vm_status::StatusCode};
use serde::{Deserialize, Serialize};

/// Mimics `0xcafe::test::ModuleData`
#[derive(Serialize, Deserialize)]
struct ModuleData {
    state: Vec<u8>,
}

#[test]
fn module_loop_depth_at_limit_test_with_stateful_sender() {
    let mut h = MoveHarness::new();
    let stateless_acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap(), Some(0));
    module_loop_depth_at_limit(&mut h, acc);
}

#[test]
fn module_loop_depth_at_limit_test_with_stateless_sender() {
    let mut h = MoveHarness::new();
    let stateless_acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap(), None);
    module_loop_depth_at_limit(&mut h, acc);
}

fn module_loop_depth_at_limit(h: &mut MoveHarness, acc: Account) {
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("max_loop_depth.data/pack-good"),
    ));
}

#[test]
fn module_loop_depth_at_limit_test_with_stateful_sender() {
    let mut h = MoveHarness::new();
    let stateless_acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap(), Some(0));
    module_loop_depth_just_above_limit(&mut h, acc);
}

#[test]
fn module_loop_depth_at_limit_test_with_stateless_sender() {
    let mut h = MoveHarness::new();
    let stateless_acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap(), None);
    module_loop_depth_just_above_limit(&mut h, acc);
}

fn module_loop_depth_just_above_limit(h: &mut MoveHarness, acc: Account) {
    assert_vm_status!(
        h.publish_package(&acc, &common::test_dir_path("max_loop_depth.data/pack-bad"),),
        StatusCode::LOOP_MAX_DEPTH_REACHED
    );
}
