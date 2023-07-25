// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_package_builder::PackageBuilder;
use aptos_types::account_address::{create_resource_address, AccountAddress};
use aptos_framework::natives::code::{PackageMetadata, UpgradePolicy};
use move_core_types::parser::parse_struct_tag;
use serde::Deserialize;

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct SomeResource {
    value: u64,
}

fn custom_build_helper(
    deployer_address: AccountAddress,
    resource_address: AccountAddress,
    package_manager_code: &String,
    basic_contract_code: &String,
) -> (PackageMetadata, Vec<Vec<u8>>) {
    // add the named addresses for `deployer` and `upgradeable_resource_account_package`
    let mut build_options = aptos_framework::BuildOptions::default();
    build_options
        .named_addresses
        .insert("deployer".to_string(), deployer_address);
    build_options
        .named_addresses
        .insert("upgradeable_resource_account_package".to_string(), resource_address);

    let mut package_builder =
        PackageBuilder::new("Upgradeable Module With Resource Account")
            .with_policy(UpgradePolicy::compat());
    package_builder.add_source("package_manager", &package_manager_code);
    package_builder.add_source("basic_contract", &basic_contract_code);
    package_builder.add_local_dep("AptosFramework", &common::framework_dir_path("aptos-framework").to_string_lossy());
    let pack_dir = package_builder.write_to_temp().unwrap();
    let package = aptos_framework::BuiltPackage::build(
        pack_dir.path().to_owned(),
        build_options,
    )
    .expect("building package must succeed");

    let code = package.extract_code();
    let metadata = package
        .extract_metadata()
        .expect("extracting package metadata must succeed");

    (metadata, code)
}

#[test]
fn code_upgrading_using_resource_account() {
    let mut h = MoveHarness::new();

    let deployer = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let resource_address = create_resource_address(*deployer.address(), &[]);

    // get contract code from file
    let package_manager_code =
        std::fs::read_to_string(
        &common::test_dir_path("../../../move-examples/upgradeable_resource_account_package/sources/package_manager.move")
        ).unwrap();
    let basic_contract_code =
        std::fs::read_to_string(
        &common::test_dir_path("../../../move-examples/upgradeable_resource_account_package/sources/basic_contract.move")
        ).unwrap();

    let (metadata, code) = custom_build_helper(
        *deployer.address(),
        resource_address,
        &package_manager_code,
        &basic_contract_code
    );

    // create the resource account and publish the module under the resource account's address
    assert_success!(h.run_transaction_payload(
        &deployer,
        aptos_cached_packages::aptos_stdlib::resource_account_create_resource_account_and_publish_package(
            vec![],
            bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
            code.clone(),
        ),
    ));

    // run the view function and check the result
    let bcs_result = h.execute_view_function(
        str::parse(&format!(
            "0x{}::basic_contract::upgradeable_function",
            resource_address
        )).unwrap(),
        vec![],
        vec![],
    ).unwrap().pop().unwrap();
    let result = bcs::from_bytes::<u64>(&bcs_result).unwrap();

    const BEFORE_VALUE: u64 = 9000;
    const AFTER_VALUE: u64 = 9001;
    // run the view function and check the result
    assert_eq!(BEFORE_VALUE, result, "assert view function result {} == {}", result, BEFORE_VALUE);

    let (metadata, code) = custom_build_helper(
        *deployer.address(),
        resource_address,
        &package_manager_code,
        &basic_contract_code.replace(&BEFORE_VALUE.to_string(), &AFTER_VALUE.to_string())
    );
    // test upgrading the code
    assert_success!(h.run_entry_function(
        &deployer,
        str::parse(&format!(
            "0x{}::package_manager::publish_package",
            resource_address
        )).unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&bcs::to_bytes(&metadata).unwrap()).unwrap(),
            bcs::to_bytes(&code).unwrap(),
        ],
    ));

    // run the view function and check the result
    let bcs_result = h.execute_view_function(
        str::parse(&format!(
            "0x{}::basic_contract::upgradeable_function",
            resource_address
        )).unwrap(),
        vec![],
        vec![],
    ).unwrap().pop().unwrap();
    let result = bcs::from_bytes::<u64>(&bcs_result).unwrap();
    assert_eq!(AFTER_VALUE, result, "assert view function result {} == {}", result, AFTER_VALUE);

    // test the `move_to_rseource_account(...)` function by moving SomeResource into the resource
    // account
    assert_success!(h.run_entry_function(
        &deployer,
        str::parse(&format!(
            "0x{}::basic_contract::move_to_resource_account",
            resource_address
        )).unwrap(),
        vec![],
        vec![],
    ));

    let some_resource = parse_struct_tag(&format!(
        "0x{}::basic_contract::SomeResource",
        resource_address
    )).unwrap();
    let some_resource_value = h
        .read_resource::<SomeResource>(&resource_address, some_resource)
        .unwrap();
    assert_eq!(some_resource_value.value, 42, "assert SomeResource.value == 42");
}
