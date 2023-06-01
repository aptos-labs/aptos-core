// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_types::{
    account_address::{create_resource_address, AccountAddress},
};

// Note: this module uses parameterized tests via the
// [`rstest` crate](https://crates.io/crates/rstest)
// to test for multiple feature combinations.

#[test]
fn code_upgrading_using_resource_account() {
    let mut h = MoveHarness::new();

    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let resource_address = create_resource_address(*acc.address(), &[]);

    // add the named addresses for `owner` and `upgrade_resource_contract`
    let mut build_options = aptos_framework::BuildOptions::default();
    build_options
        .named_addresses
        .insert("owner".to_string(), *acc.address());
    build_options
        .named_addresses
        .insert("upgrade_resource_contract".to_string(), resource_address);

    // build the package from our example code
    let package = aptos_framework::BuiltPackage::build(
        common::test_dir_path("../../../move-examples/upgrade_resource_contract"),
        build_options,
    )
    .expect("building package must succeed");

    let code = package.extract_code();
    let metadata = package
        .extract_metadata()
        .expect("extracting package metadata must succeed");

    // create the resource account and publish the module under the resource account's address
    let result = h.run_transaction_payload(
        &acc,
        aptos_cached_packages::aptos_stdlib::resource_account_create_resource_account_and_publish_package(
            vec![],
            bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
            code.clone(),
        ),
    );

    assert_success!(result);

    // test upgrading the code
    assert_success!(h.run_entry_function(
        &acc,
        str::parse(&format!(
            "0x{}::upgrader::upgrade_contract",
            resource_address
        )).unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&bcs::to_bytes(&metadata).unwrap()).unwrap(),
            bcs::to_bytes(&code).unwrap(),
        ],
    ));
}
