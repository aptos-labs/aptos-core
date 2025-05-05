// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! # Overview
//! This module contains tests demonstrating how to use [`MoveHarness`]'s
//! network-forking feature, allowing simulations on top of the current state
//! of a remote Aptos network.
//!
//! **Note**: These simulations run entirely locally -- they do **not** affect the
//! actual remote network state in any way.
//!
//! This workflow is particularly useful for testing critical changes or bug fixes
//! in a realistic environment before deploying them to production.
//!
//! # Dummy Account Setup
//! The tests rely on a dummy account on testnet, pre-configured as follows:
//! - Some APT balance
//! - A module `test` published under its address
//!   - This contains a function `foo` that returns the `u64` value `100`
//! - The same module also published as a code object, owned by the same account
//!
//! # Running Tests Manually
//! These tests are marked `#[ignore]` to prevent them from running in CI.
//! since they depend on a specific transaction version that may be pruned over time.
//!
//! To run them manually, use `cargo test -- --ignored`.
//! If the tests fail due to the version being pruned, update [`TESTNET_TXN_VERSION`] accordingly.

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_rest_client::AptosBaseUrl;
use aptos_types::account_address::AccountAddress;
use move_core_types::{language_storage::TypeTag, value::MoveValue};
use std::str::FromStr;

const APTOS_COIN_STRUCT_STRING: &str = "0x1::aptos_coin::AptosCoin";

const TESTNET_TXN_VERSION: u64 = 6691904943;
const TESTNET_ACCOUNT_ADDR: &str =
    "0x3f9e0589ca0668a5273b86bfcb5f357164408a889bc733b309cf1901098c8ce5";
const TESTNET_CODE_OBJECT_ADDR: &str =
    "0x49dc2690339e3a7ad944d2eb6dde038f98b9ddece711530f4db2fbab67b741ed";
const TESTNET_ACCOUNT_APT_BALANCE: u64 = 91_8290_3550;

/// Helper function to fetch the APT balance of the specified account.
fn get_account_apt_balance(h: &mut MoveHarness, addr: AccountAddress) -> u64 {
    let bytes = h
        .execute_view_function(
            str::parse("0x1::coin::balance").unwrap(),
            vec![TypeTag::from_str(APTOS_COIN_STRUCT_STRING).unwrap()],
            vec![addr.to_vec()],
        )
        .values
        .unwrap()
        .pop()
        .unwrap();
    bcs::from_bytes::<u64>(bytes.as_slice()).unwrap()
}

/// Reads the APT balance of the test account and checks that it matches the expected value.
#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn view_existing_account_balance() {
    // Create a new `MoveHarness` connected to the testnet API endpoint.
    //
    // Simulations based on remote states rely heavily on API calls, which can easily run into
    // rate limits if executed repeatedly or in parallel.
    // Providing an API key raises these limits significantly.
    //
    // If you hit rate limits, you can create a free Aptos Build account and generate an API key:
    // - https://build.aptoslabs.com/docs/start#api-quick-start
    //
    // To use an API key here, switch to the alternative constructor:
    // `MoveHarness::new_with_remote_state_with_api_key`.
    let mut h = MoveHarness::new_with_remote_state(AptosBaseUrl::Testnet, TESTNET_TXN_VERSION);

    let existing_account_addr = AccountAddress::from_hex_literal(TESTNET_ACCOUNT_ADDR).unwrap();

    assert_eq!(
        get_account_apt_balance(&mut h, existing_account_addr),
        TESTNET_ACCOUNT_APT_BALANCE
    )
}

/// Transfers 1 APT from a newly created account to the existing test account
/// and verifies that the recipient's balance increases accordingly.
#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn transfer_to_existing_account() {
    let mut h = MoveHarness::new_with_remote_state(AptosBaseUrl::Testnet, TESTNET_TXN_VERSION);

    let existing_account_addr = AccountAddress::from_hex_literal(TESTNET_ACCOUNT_ADDR).unwrap();

    // Create a new account and fund it with 10 APT.
    let new_account =
        h.new_account_with_balance_and_sequence_number(10_0000_0000 /* 10 APT */, 0);

    // Transfer 1 APT to the existing account.
    let status = h.run_entry_function(
        &new_account,
        str::parse("0x1::coin::transfer").unwrap(),
        vec![TypeTag::from_str("0x1::aptos_coin::AptosCoin").unwrap()],
        vec![
            MoveValue::Address(existing_account_addr)
                .simple_serialize()
                .unwrap(),
            MoveValue::U64(1_0000_0000).simple_serialize().unwrap(),
        ],
    );
    assert_success!(status);

    // Verify that the recipient's balance has increased by 1 APT.
    assert_eq!(
        get_account_apt_balance(&mut h, existing_account_addr),
        TESTNET_ACCOUNT_APT_BALANCE + 1_0000_0000
    )
}

/// Attempts to transfer 1 APT from the existing test account to a newly created account,
/// verifying that the sender's balance decreases appropriately.
#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn transfer_from_existing_account() {
    let mut h = MoveHarness::new_with_remote_state(AptosBaseUrl::Testnet, TESTNET_TXN_VERSION);

    let existing_account_addr = AccountAddress::from_hex_literal(TESTNET_ACCOUNT_ADDR).unwrap();

    // Create a new account.
    let new_account =
        h.new_account_with_balance_and_sequence_number(10_0000_0000 /* 10 APT */, 0);

    // Rotate the authentication key of the existing account so that we can authenticate transactions
    // without using or exposing the real private key.
    //
    // This is a recommended security practice to prevent accidental leakage of the original private key,
    // such as pushing to github accidentally.
    let existing_account = h
        .executor
        .rotate_account_authentication_key(existing_account_addr);

    // Transfer 1 APT from the existing account to the new account.
    let status = h.run_entry_function(
        &existing_account,
        str::parse("0x1::coin::transfer").unwrap(),
        vec![TypeTag::from_str("0x1::aptos_coin::AptosCoin").unwrap()],
        vec![
            MoveValue::Address(*new_account.address())
                .simple_serialize()
                .unwrap(),
            MoveValue::U64(1_0000_0000).simple_serialize().unwrap(),
        ],
    );
    assert_success!(status);

    // Assert that the sender's balance has decreased
    // (but due to gas fees, the exact value will be slightly less).
    assert!(
        get_account_apt_balance(&mut h, existing_account_addr)
            < TESTNET_ACCOUNT_APT_BALANCE - 1_0000_0000
    );
}

/// Upgrades a Move package that is published under the existing account.
#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn upgrade_package() {
    let mut h = MoveHarness::new_with_remote_state(AptosBaseUrl::Testnet, TESTNET_TXN_VERSION);

    let existing_account_addr = AccountAddress::from_hex_literal(TESTNET_ACCOUNT_ADDR).unwrap();

    // A module named `test` is already published under this account.
    //
    // Calling `test::foo()` should return 100 before the upgrade.
    let bytes = h
        .execute_view_function(
            str::parse(&format!("0x{}::test::foo", existing_account_addr.to_hex())).unwrap(),
            vec![],
            vec![],
        )
        .values
        .unwrap()
        .pop()
        .unwrap();
    let val = bcs::from_bytes::<u64>(&bytes).unwrap();
    assert_eq!(val, 100);

    // Rotate the authentication key of the existing account so that we can authenticate transactions
    // without using or exposing the real private key.
    //
    // This is a recommended security practice to prevent accidental leakage of the original private key,
    // such as pushing to github accidentally.
    let existing_account = h
        .executor
        .rotate_account_authentication_key(existing_account_addr);

    // Attempt to upgrade the `test` module to a newer version.
    let mut build_options = BuildOptions::default();
    build_options
        .named_addresses
        .insert("my_addr".to_string(), existing_account_addr);
    let status = h.publish_package_with_options(
        &existing_account,
        &common::test_dir_path("remote_state.data/test_package"),
        build_options,
    );
    assert_success!(status);

    // Verify that `test::foo()` now returns 300 after the upgrade.
    let bytes = h
        .execute_view_function(
            str::parse(&format!("0x{}::test::foo", existing_account_addr.to_hex())).unwrap(),
            vec![],
            vec![],
        )
        .values
        .unwrap()
        .pop()
        .unwrap();
    let val = bcs::from_bytes::<u64>(&bytes).unwrap();
    assert_eq!(val, 300);
}

/// Attempts to upgrade a package managed by a code object.
#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn upgrade_package_via_object() {
    let mut h = MoveHarness::new_with_remote_state(AptosBaseUrl::Testnet, TESTNET_TXN_VERSION);

    // A module named `test` is published under this object address.
    // Calling `test::foo()` should return 200 before the upgrade.
    let code_object_addr = AccountAddress::from_hex_literal(TESTNET_CODE_OBJECT_ADDR).unwrap();

    let bytes = h
        .execute_view_function(
            str::parse(&format!("0x{}::test::foo", code_object_addr.to_hex())).unwrap(),
            vec![],
            vec![],
        )
        .values
        .unwrap()
        .pop()
        .unwrap();
    let val = bcs::from_bytes::<u64>(&bytes).unwrap();
    assert_eq!(val, 200);

    // Rotate the authentication key of the owner account so that we can authenticate transactions
    // without using or exposing the real private key.
    //
    // This is a recommended security practice to prevent accidental leakage of the original private key,
    // such as pushing to github accidentally.
    let existing_account_addr: AccountAddress =
        AccountAddress::from_hex_literal(TESTNET_ACCOUNT_ADDR).unwrap();

    let existing_account = h
        .executor
        .rotate_account_authentication_key(existing_account_addr);

    // Now upgrade the `test` module to a newer version using the object code upgrade flow.
    let mut build_options = BuildOptions::default();
    build_options
        .named_addresses
        .insert("my_addr".to_string(), code_object_addr);
    let status = h.object_code_upgrade_package(
        &existing_account,
        &common::test_dir_path("remote_state.data/test_package"),
        build_options,
        code_object_addr,
    );
    assert_success!(status);

    // Verify that `test::foo()` now returns 300 after the upgrade.
    let bytes = h
        .execute_view_function(
            str::parse(&format!("0x{}::test::foo", code_object_addr.to_hex())).unwrap(),
            vec![],
            vec![],
        )
        .values
        .unwrap()
        .pop()
        .unwrap();
    let val = bcs::from_bytes::<u64>(&bytes).unwrap();
    assert_eq!(val, 300);
}
