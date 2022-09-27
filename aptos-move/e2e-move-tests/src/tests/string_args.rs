// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_types::account_address::AccountAddress;
use move_deps::move_core_types::parser::parse_struct_tag;
use serde::{Deserialize, Serialize};

/// Mimics `0xcafe::test::ModuleData`
#[derive(Serialize, Deserialize)]
struct ModuleData {
    state: Vec<u8>,
}

#[test]
fn string_args() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(&acc, &common::test_dir_path("string_args.data/pack")));

    let module_data = parse_struct_tag("0xCAFE::test::ModuleData").unwrap();

    // Check in initial state, resource does not exist.
    assert!(!h.exists_resource(acc.address(), module_data.clone()));

    // Now send hi transaction, after that resource should exist and carry value
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::hi").unwrap(),
        vec![],
        vec![bcs::to_bytes("hi there!".as_bytes()).unwrap()],
    ));
    assert_eq!(
        String::from_utf8(
            h.read_resource::<ModuleData>(acc.address(), module_data)
                .unwrap()
                .state
        )
        .unwrap(),
        "hi there!"
    );
}
