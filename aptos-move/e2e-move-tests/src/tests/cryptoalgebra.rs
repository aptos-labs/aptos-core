// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{assert_success, tests::common, MoveHarness};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{ExecutionStatus, TransactionStatus},
};

fn deep_type_tag(harness: &mut MoveHarness) -> TransactionStatus {
    let account = harness.new_account_at(AccountAddress::from_hex_literal("0x42").unwrap());
    assert_success!(harness.publish_package(&account, &common::test_dir_path("cryptoalgebra.data/large_type_tag"),));
    harness.run_entry_function(
        &account,
        str::parse("0x42::test::main").unwrap(),
        vec![],
        vec![],
    )
}

#[test]
#[should_panic]
fn deep_type_tag_panic_regression() {
    let mut h = MoveHarness::new();
    deep_type_tag(&mut h);
}

#[test]
fn test_deep_type_tag() {
    let mut h = MoveHarness::new();
    h.new_epoch();
    let result = deep_type_tag(&mut h);

    assert!(
        matches!(
            result,
            TransactionStatus::Keep(ExecutionStatus::MoveAbort { .. })
        ),
    );
}
