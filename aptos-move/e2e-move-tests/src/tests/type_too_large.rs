// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{assert_success, tests::common, MoveHarness};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{ExecutionStatus, TransactionStatus},
};
use move_core_types::vm_status::StatusCode;

#[test]
fn type_too_large() {
    let mut h = MoveHarness::new();

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
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

    assert!(matches!(
        result,
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::TOO_MANY_TYPE_NODES
        )))
    ));
}
