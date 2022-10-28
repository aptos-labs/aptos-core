// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_types::account_config::CoinStoreResource;
use aptos_types::{
    account_address::{create_resource_address, AccountAddress},
    transaction::{EntryFunction, TransactionPayload},
};
use cached_packages::aptos_stdlib;
use framework::{BuildOptions, BuiltPackage};
use language_e2e_tests::account::Account;
use move_core_types::{ident_str, language_storage::ModuleId, parser::parse_struct_tag};

// :!:>section_1c
static NODE_URL: Lazy<Url> = Lazy::new(|| {
    Url::from_str(
        std::env::var("APTOS_NODE_URL")
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("https://fullnode.devnet.aptoslabs.com"),
    )
        .unwrap()
});

static FAUCET_URL: Lazy<Url> = Lazy::new(|| {
    Url::from_str(
        std::env::var("APTOS_FAUCET_URL")
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("https://faucet.devnet.aptoslabs.com"),
    )
        .unwrap()
});
// <:!:section_1c

#[tokio::main]
async fn main() {
    // :!:>section_1a
    let rest_client = Client::new(NODE_URL.clone());
    let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone()); // <:!:section_1a

    // create an origin account and create a resource address from it
    let mut origin_account = LocalAccount::generate(&mut rand::rngs::OsRng);
    let resource_address = create_resource_address(*origin_account.address(), vec![].as_slice());

    let mut build_options = BuildOptions::default();
    build_options
        .named_addresses
        .insert("mint_nft".to_string(), resource_address);
    let package = BuiltPackage::build(
        common::test_dir_path("../mint_nft"),
        build_options,
    )
        .expect("building package must succeed");
    let code = package.extract_code();
    let metadata = package
        .extract_metadata()
        .expect("extracting package metadata must succeed");


    // create the resource account and publish the code under the resource account's address
    let result = h.run_transaction_payload(
        &origin_account,
        aptos_stdlib::resource_account_create_resource_account_and_publish_package(
            vec![],
            bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
            code,
        ),
    );
    assert_success!(result);
}