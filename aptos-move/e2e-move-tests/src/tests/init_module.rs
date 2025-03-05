// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{assert_abort, assert_success, tests::common, MoveHarness};
use aptos_package_builder::PackageBuilder;
use aptos_types::account_address::AccountAddress;
use move_core_types::parser::parse_struct_tag;
use serde::{Deserialize, Serialize};
use rstest::rstest;

/// Mimics `0xcafe::test::ModuleData`
#[derive(Serialize, Deserialize)]
struct ModuleData {
    global_counter: u64,
}

#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions, 
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true),
)]
fn init_module(stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap(), if stateless_account { None } else { Some(0)});
    
    assert_success!(
        h.publish_package_cache_building(&acc, &common::test_dir_path("init_module.data/pack"))
    );

    // Verify that init_module was called.
    let module_data = parse_struct_tag("0xcafe::test::ModuleData").unwrap();
    assert_eq!(
        h.read_resource::<ModuleData>(acc.address(), module_data.clone())
            .unwrap()
            .global_counter,
        42
    );

    // Republish to show that init_module is not called again. If init_module would be called again,
    // we would get an abort here because the first time, it used move_to for initialization.
    assert_success!(
        h.publish_package_cache_building(&acc, &common::test_dir_path("init_module.data/pack"))
    );
    assert_eq!(
        h.read_resource::<ModuleData>(acc.address(), module_data)
            .unwrap()
            .global_counter,
        42
    );
}


#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions, 
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true),
)]
fn init_module_when_republishing_package(stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap(), if stateless_account { None } else { Some(0)});

    // Deploy a package that initially does not have the module that has the init_module function.
    assert_success!(h.publish_package_cache_building(
        &acc,
        &common::test_dir_path("init_module.data/pack_initial")
    ));

    // Now republish the package with the new module that has init_module.
    assert_success!(
        h.publish_package_cache_building(&acc, &common::test_dir_path("init_module.data/pack"))
    );

    // Verify that init_module was called.
    let module_data = parse_struct_tag("0xcafe::test::ModuleData").unwrap();
    assert_eq!(
        h.read_resource::<ModuleData>(acc.address(), module_data)
            .unwrap()
            .global_counter,
        42
    );
}

#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions, 
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true),
)]
fn init_module_with_abort_and_republish(stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x12").unwrap(), if stateless_account { None } else { Some(0) });

    let mut p1 = PackageBuilder::new("Pack");
    p1.add_source(
        "m.move",
        "module 0x12::M { fun init_module(_s: &signer) { abort 1 } }",
    );
    let path1 = p1.write_to_temp().unwrap();

    let mut p2 = PackageBuilder::new("Pack");
    p2.add_source(
        "m.move",
        "module 0x12::M { fun init_module(_s: &signer) {} }",
    );
    let path2 = p2.write_to_temp().unwrap();

    let txn1 = h.create_publish_package(&acc, path1.path(), None, |_| {});
    let txn2 = h.create_publish_package(&acc, path2.path(), None, |_| {});
    let res = h.run_block(vec![txn1, txn2]);

    // First publish aborts, package should not count as published.
    assert_abort!(res[0], 1);

    // 2nd publish succeeds, not the old but the new init_module is called.
    assert_success!(res[1]);
}
