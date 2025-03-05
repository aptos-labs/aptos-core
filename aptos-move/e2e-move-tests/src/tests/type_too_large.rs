// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{assert_abort, assert_success, tests::common, MoveHarness};
use aptos_types::account_address::AccountAddress;
use rstest::rstest;

#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true),
)]
fn type_too_large(stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap(), if stateless_account { None } else { Some(0) });
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("type_too_large.data/type_too_large"),
    ));

    let result = h.run_entry_function(
        &acc,
        str::parse("0xbeef::test::run").unwrap(),
        vec![],
        vec![],
    );

    // The abort code is NFE_BCS_SERIALIZATION_FAILURE = 0x1c5, since the actual VM error
    // for TOO_MANY_TYPE_NODES is hidden by the bcs serializer and turned into this generic error.
    assert_abort!(result, 0x1C5);
}
