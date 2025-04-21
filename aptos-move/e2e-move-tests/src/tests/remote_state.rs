// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! # Overview
//! This module contains tests demonstrating how to use [`MoveHarness`]'s
//! network-forking feature, allowing simulations on top of the current state
//! of a remote Aptos network.
//!
//! It should be emphasized that such simulations are done entirely locally --
//! they do not modify the states of the remote network in any ways.
//!
//! This workflow is particularly useful for testing critical changes or bug fixes
//! in a realistic environment before deploying them.
//!
//! # Dummy Account Setup
//! The tests rely on a dummy account on testnet, which has some APT balance.
//!
//! # Running Tests Manually
//! These tests are marked `#[ignore]` to prevent execution in CI.
//! This avoids CI failures when the referenced transaction version
//! falls outside the fullnode's pruning window, which will inevitably occur over time.
//!
//! To run these tests manually, append `-- --ignored` to the `cargo test` command.
//! Additionally, [`TEST_TXN_VERSION`] may need to be updated if it gets too old.

use crate::{assert_success, MoveHarness};
use aptos_rest_client::AptosBaseUrl;
use aptos_types::account_address::AccountAddress;
use move_core_types::{language_storage::TypeTag, value::MoveValue};
use std::str::FromStr;

const APTOS_COIN_STRUCT_STRING: &str = "0x1::aptos_coin::AptosCoin";

const TESTNET_TXN_VERSION: u64 = 6660862455;
const TESTNET_ACCOUNT_ADDR: &str =
    "0x3f9e0589ca0668a5273b86bfcb5f357164408a889bc733b309cf1901098c8ce5";
const TEST_ACCOUNT_APT_BALANCE: u64 = 91_8316_5250;

/// Helper that fetches the APT balance of a given account.
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

/// Simple test that reads the APT balance of the test account.
#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn view_existing_account_balance() {
    let mut h = MoveHarness::new_with_remote_state(AptosBaseUrl::Testnet, TESTNET_TXN_VERSION);

    let existing_account_addr = AccountAddress::from_hex_literal(TESTNET_ACCOUNT_ADDR).unwrap();

    assert_eq!(
        get_account_apt_balance(&mut h, existing_account_addr),
        TEST_ACCOUNT_APT_BALANCE
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
        TEST_ACCOUNT_APT_BALANCE + 1_0000_0000
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
            < TEST_ACCOUNT_APT_BALANCE - 1_0000_0000
    );
}
