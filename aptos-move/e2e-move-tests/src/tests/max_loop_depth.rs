// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_types::vm_status::StatusCode;
use rstest::rstest;
use serde::{Deserialize, Serialize};

/// Mimics `0xcafe::test::ModuleData`
#[derive(Serialize, Deserialize)]
struct ModuleData {
    state: Vec<u8>,
}

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
fn module_loop_depth_at_limit(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let acc = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    let mut build_options = BuildOptions::default();
    build_options
        .named_addresses
        .insert("publisher".to_string(), *acc.address());
    assert_success!(h.publish_package_with_options(
        &acc,
        &common::test_dir_path("max_loop_depth.data/pack-good"),
        build_options,
    ));
}

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
fn module_loop_depth_just_above_limit(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let acc = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    let mut build_options = BuildOptions::default();
    build_options
        .named_addresses
        .insert("publisher".to_string(), *acc.address());
    assert_vm_status!(
        h.publish_package_with_options(
            &acc,
            &common::test_dir_path("max_loop_depth.data/pack-bad"),
            build_options,
        ),
        StatusCode::LOOP_MAX_DEPTH_REACHED,
    );
}
