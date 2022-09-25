// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_types::account_address::AccountAddress;
use cached_packages::aptos_stdlib;
use framework::{BuildOptions, BuiltPackage};
use move_deps::move_core_types::parser::parse_struct_tag;
use serde::{Deserialize, Serialize};

/// Mimics `0xcafe::test::ModuleData`
#[derive(Serialize, Deserialize)]
struct ModuleData {
    u64_value: u64,
    u8_value: u8,
    address_value: AccountAddress,
    signer_cap: AccountAddress,
}

#[test]
fn resource_account_with_data() {
    let mut h = MoveHarness::new();
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let resource_address = aptos_types::account_address::create_resource_address(
        *account.address(),
        vec![].as_slice(),
    );

    let mut build_options = BuildOptions::default();
    build_options
        .named_addresses
        .insert("resource_account".to_string(), resource_address);
    let package = BuiltPackage::build(
        common::test_dir_path("resource_account.data/with_data"),
        build_options,
    )
    .expect("building package must succeed");
    let code = package.extract_code();
    let metadata = package
        .extract_metadata()
        .expect("extracting package metadata must succeed");

    let u64_value = 12345u64;
    let u8_value = 123u8;

    let blob = bcs::to_bytes(&(u64_value, u8_value, account.address())).unwrap();
    let txn = h.create_transaction_payload(
        &account,
        aptos_stdlib::resource_account_create_resource_account_and_publish_package_with_data(
            vec![],
            bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
            code,
            blob,
        ),
    );
    assert_success!(h.run(txn));

    // Verify that init_module was called.
    let module_data =
        parse_struct_tag(&format!("0x{}::core::ModuleData", resource_address)).unwrap();
    let resource = h
        .read_resource::<ModuleData>(&resource_address, module_data.clone())
        .unwrap();
    assert_eq!(resource.u64_value, u64_value);
    assert_eq!(resource.u8_value, u8_value);
    assert_eq!(resource.address_value, *account.address());
    assert_eq!(resource.signer_cap, resource_address);
}
