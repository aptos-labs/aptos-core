// Copyright © Aptos Foundation

use crate::{
    assert_success,
    tests::common,
    transaction_context::{create_many_uuids, initialize},
    MoveHarness,
};
use aptos_language_e2e_tests::account::Account;

fn setup() -> (MoveHarness, Account) {
    initialize(common::test_dir_path("transaction_context.data"))
}

#[test]
fn test_many_unique_uuids() {
    let (mut h, acc) = setup();

    let txn1 = create_many_uuids(&mut h, &acc, 50);

    assert_success!(h.run(txn1));
}
