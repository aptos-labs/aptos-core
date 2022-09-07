use crate::{assert_success, tests::common, MoveHarness};
use aptos_types::account_address::AccountAddress;
use cached_packages::aptos_stdlib;
use framework::{BuildOptions, BuiltPackage};
use move_deps::move_core_types::parser::parse_struct_tag;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct ModuleData {
    resource_signer_cap: AccountAddress,
}

#[test]
fn resource_account_init_module_and_store_signer_capability() {
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
        common::test_dir_path("../../../move-examples/resource_account"),
        build_options,
    )
        .expect("building package must succeed");
    let code = package.extract_code();
    let metadata = package
        .extract_metadata()
        .expect("extracting package metadata must succeed");

    let result = h.run_transaction_payload(
        &account,
        aptos_stdlib::resource_account_create_resource_account_and_publish_package(
            vec![],
            bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
            code,
        ),
    );
    assert_success!(result);

    // verify that we store the signer cap within the module
    let module_data = parse_struct_tag(&format!("0x{}::resource_account::ModuleData", resource_address)).unwrap();

    assert_eq!(
        h.read_resource::<ModuleData>(&resource_address, module_data.clone()).unwrap().resource_signer_cap,
        resource_address
    );
}
