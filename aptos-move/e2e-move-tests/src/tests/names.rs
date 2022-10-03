// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
//

use crate::{assert_success, MoveHarness};
use aptos_types::account_address::AccountAddress;
use cached_packages::aptos_names_sdk_builder;
use cached_packages::aptos_stdlib;
use move_deps::move_core_types::language_storage::CORE_CODE_ADDRESS;

#[test]
fn test_names_end_to_end() {
    let mut harness = MoveHarness::new();

    let user = harness.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());

    let aptos_framework_account = harness.new_account_at(CORE_CODE_ADDRESS);
    harness.run_transaction_payload(
        &aptos_framework_account,
        aptos_stdlib::aptos_coin_mint(CORE_CODE_ADDRESS, 1000),
    );

    assert_success!(harness.run_transaction_payload(
        &user,
        aptos_names_sdk_builder::domains_register_domain("max".to_string().into_bytes(), 2),
    ));
}
