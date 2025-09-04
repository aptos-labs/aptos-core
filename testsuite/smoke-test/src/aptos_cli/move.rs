// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::SwarmBuilder;
use velor::test::CliTestFramework;
use velor_framework::{BuildOptions, BuiltPackage};
use velor_logger::info;
use velor_types::move_utils::MemberId;
use move_core_types::account_address::AccountAddress;
use move_package::source_package::manifest_parser::parse_move_manifest_from_file;
use std::{collections::BTreeMap, path::PathBuf, str::FromStr};

const PACKAGE_NAME: &str = "AwesomePackage";
const HELLO_BLOCKCHAIN: &str = "hello_blockchain";

fn velor_framework_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("velor-move")
        .join("framework")
        .join("velor-framework")
}

#[tokio::test]
async fn test_move_compile_flow() {
    let mut cli = CliTestFramework::local_new(1);

    cli.init_move_dir();
    let move_dir = cli.move_dir();
    let account = cli.account_id(0).to_hex_literal();

    let mut package_addresses = BTreeMap::new();
    package_addresses.insert(HELLO_BLOCKCHAIN, "_");

    cli.init_package(
        PACKAGE_NAME.to_string(),
        package_addresses,
        Some(velor_framework_dir()),
    )
    .await
    .expect("Should succeed");

    // The manifest should work to compile
    let mut named_addresses = BTreeMap::new();
    named_addresses.insert(HELLO_BLOCKCHAIN, account.as_str());
    match cli.compile_package(named_addresses.clone(), None).await {
        Ok(modules) => assert!(modules.is_empty()),
        Err(err) => panic!("Error compiling: {:?}", err),
    }

    // Let's check that it's setup correctly
    let manifest = parse_move_manifest_from_file(move_dir.join("Move.toml").as_path())
        .expect("Expect a Move.toml file");
    assert_eq!(manifest.package.name.as_str(), PACKAGE_NAME);
    // Expect "1.0.0"
    assert_eq!(manifest.package.version.0, 1);
    assert_eq!(manifest.package.version.1, 0);
    assert_eq!(manifest.package.version.2, 0);

    let addresses = manifest.addresses.expect("Expect some addresses");
    assert_eq!(addresses.len(), 1);
    let (key, value) = addresses.iter().next().expect("Expect an address");
    assert_eq!(key.as_str(), HELLO_BLOCKCHAIN);
    assert!(value.is_none());

    assert_eq!(manifest.dependencies.len(), 1);

    let dependency = manifest.dependencies.iter().next().unwrap();
    assert_eq!("VelorFramework", dependency.0.to_string());

    // Now try to compile real code
    cli.add_move_files();

    match cli.compile_package(named_addresses.clone(), None).await {
        Ok(modules) => assert!(!modules.is_empty()),
        Err(err) => panic!("Error compiling: {:?}", err),
    }

    // Run tests to ensure they work too
    match cli.test_package(named_addresses.clone(), None).await {
        Ok(result) => assert_eq!("Success", result),
        Err(err) => panic!("Error testing: {:?}", err),
    }
}

#[tokio::test]
async fn test_move_publish_flow() {
    let (_swarm, mut cli, _faucet) = SwarmBuilder::new_local(1)
        .with_velor()
        .build_with_cli(2)
        .await;

    let account = cli.account_id(0).to_hex_literal();
    // Setup move package
    cli.init_move_dir();
    let mut package_addresses = BTreeMap::new();
    package_addresses.insert(HELLO_BLOCKCHAIN, "_");
    cli.init_package(
        PACKAGE_NAME.to_string(),
        package_addresses,
        Some(velor_framework_dir()),
    )
    .await
    .expect("Should succeed");
    cli.add_move_files();

    cli.wait_for_account(0)
        .await
        .expect("Should create account");
    info!("Move package dir: {}", cli.move_dir().display());

    // Let's publish it
    let mut named_addresses = BTreeMap::new();
    named_addresses.insert(HELLO_BLOCKCHAIN, account.as_str());
    let _ = match cli.publish_package(0, None, named_addresses, None).await {
        Ok(response) => response,
        Err(err) => panic!("Should not have failed to publish package {:?}", err),
    };

    // TODO: Verify transaction summary

    // Wrong number of args will definitely fail
    let function_id = MemberId::from_str(&format!("{}::message::set_message", account)).unwrap();

    assert!(cli
        .run_function(0, None, function_id.clone(), vec![], vec![])
        .await
        .is_err());

    assert!(cli
        .run_function(0, None, function_id, vec!["string:hello_world"], vec![])
        .await
        .is_ok());

    // Now download the package. It will be stored in a directory PACKAGE_NAME inside move_dir.
    let _ = match cli
        .download_package(0, PACKAGE_NAME.to_owned(), cli.move_dir())
        .await
    {
        Ok(response) => response,
        Err(err) => panic!("Should not have failed to download package {:?}", err),
    };

    // Ensure the downloaded package can build. This is a test that the information is correctly
    // roundtripped.
    let _ = match BuiltPackage::build(cli.move_dir().join(PACKAGE_NAME), BuildOptions {
        named_addresses: std::iter::once((
            HELLO_BLOCKCHAIN.to_owned(),
            AccountAddress::from_hex_literal(&account).expect("account address parsable"),
        ))
        .collect(),
        ..BuildOptions::default()
    }) {
        Ok(response) => response,
        Err(err) => panic!(
            "Should not have failed to build downloaded package {:?}",
            err
        ),
    };
}
