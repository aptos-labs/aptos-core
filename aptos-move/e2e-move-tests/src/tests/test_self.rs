// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use rstest::rstest;

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_self(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    // Load the code
    let acc = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    let mut build_options = BuildOptions::move_2();
    build_options
        .named_addresses
        .insert("publisher".to_string(), *acc.address());
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("smart_data_structures_self.data"),
        build_options
    ));
}
