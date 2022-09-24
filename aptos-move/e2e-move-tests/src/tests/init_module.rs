// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_types::account_address::AccountAddress;
use move_deps::move_core_types::parser::parse_struct_tag;
use serde::{Deserialize, Serialize};

/// Mimics `0xcafe::test::ModuleData`
#[derive(Serialize, Deserialize)]
struct ModuleData {
    global_counter: u64,
}

#[test]
fn init_module() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("init_module.data/pack")));

    // Verify that init_module was called.
    let module_data = parse_struct_tag("0xCAFE::test::ModuleData").unwrap();
    assert_eq!(
        h.read_resource::<ModuleData>(acc.address(), module_data.clone())
            .unwrap()
            .global_counter,
        42
    );

    // Republish to show that init_module is not called again. If init_module would be called again,
    // we would get an abort here because the first time, it used move_to for initialization.
    assert_success!(h.publish_package(&acc, &common::test_dir_path("init_module.data/pack")));
    assert_eq!(
        h.read_resource::<ModuleData>(acc.address(), module_data)
            .unwrap()
            .global_counter,
        42
    );
}
