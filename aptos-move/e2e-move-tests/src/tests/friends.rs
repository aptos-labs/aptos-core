// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{ExecutionStatus, TransactionStatus},
};
use move_core_types::vm_status::StatusCode;
use test_case::test_case;

#[test_case(true)]
#[test_case(false)]
fn cannot_publish_cross_package_friends(enable_lazy_loading: bool) {
    let mut h = MoveHarness::new_with_lazy_loading(enable_lazy_loading);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(
        h.publish_package_cache_building(&acc, &common::test_dir_path("friends.data/p1"))
    );

    // Module in p2 declares a module in p1 a friend. With lazy loading this is not allowed.
    let res = h.publish_package_cache_building(&acc, &common::test_dir_path("friends.data/p2"));
    if enable_lazy_loading {
        assert!(matches!(
            res,
            TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
                StatusCode::FRIEND_NOT_FOUND_IN_MODULE_BUNDLE
            )))
        ));
    } else {
        assert_success!(res);
    }
}
