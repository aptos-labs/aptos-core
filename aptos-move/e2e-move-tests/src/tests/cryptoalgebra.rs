// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{assert_success, tests::common, MoveHarness};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{ExecutionStatus, TransactionStatus},
};

fn crypto_algebra_type_tag_limit_exceeded(harness: &mut MoveHarness) -> TransactionStatus {
    let acc = harness.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    assert_success!(harness.publish_package(&acc, &common::test_dir_path("cryptoalgebra.data/p"),));

    harness.run_entry_function(
        &acc,
        str::parse("0xbeef::test::run").unwrap(),
        vec![],
        vec![],
    )
}

#[test]
fn crypto_algebra_type_tag_limit_exceeded_handled() {
    let mut h = MoveHarness::new();
    h.new_epoch();
    let result = crypto_algebra_type_tag_limit_exceeded(&mut h);

    assert!(
        matches!(
            result,
            TransactionStatus::Keep(ExecutionStatus::MoveAbort { .. })
        ),
        "result: {:?}",
        result
    );
}
