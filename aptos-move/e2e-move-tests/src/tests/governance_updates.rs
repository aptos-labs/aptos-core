// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::MoveHarness;
use aptos_crypto::HashValue;
use aptos_language_e2e_tests::account::{Account, TransactionBuilder};
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::{ApprovedExecutionHashes, OnChainConfig},
    transaction::{Script, TransactionArgument, TransactionStatus},
};

#[test]
fn large_transactions() {
    // This test validates that only small txns (less than the maximum txn size) can be kept. It
    // then evaluates the limits of the ApprovedExecutionHashes. Specifically, the hash is the code
    // is the only portion that can exceed the size limits. There's a further restriction on the
    // maximum transaction size of 1 MB even for governance, because the governance transaction can
    // be submitted by any one and that can result in a large amount of large transactions making their
    // way into consensus.
    let mut h = MoveHarness::new();

    let alice = h.new_account_at(AccountAddress::from_hex_literal("0xa11ce").unwrap());
    let root = h.aptos_framework_account();
    let entries = ApprovedExecutionHashes { entries: vec![] };
    h.set_resource(
        *root.address(),
        ApprovedExecutionHashes::struct_tag(),
        &entries,
    );

    let small = vec![0; 1024];
    // Max size is 1024 * 1024
    let large = vec![0; 1000 * 1024];
    let very_large = vec![0; 1024 * 1024];

    let status = run(&mut h, &alice, small.clone(), small.clone());
    assert!(!status.is_discarded());
    let status = run(&mut h, &alice, large.clone(), small.clone());
    assert!(status.is_discarded());
    let status = run(&mut h, &alice, small.clone(), large.clone());
    assert!(status.is_discarded());
    let status = run(&mut h, &alice, large.clone(), large.clone());
    assert!(status.is_discarded());
    let status = run(&mut h, &alice, very_large.clone(), small.clone());
    assert!(status.is_discarded());

    let entries = ApprovedExecutionHashes {
        entries: vec![
            (0, HashValue::sha3_256_of(&large).to_vec()),
            (1, HashValue::sha3_256_of(&very_large).to_vec()),
        ],
    };
    h.set_resource(
        *root.address(),
        ApprovedExecutionHashes::struct_tag(),
        &entries,
    );

    let status = run(&mut h, &alice, small.clone(), small.clone());
    assert!(!status.is_discarded());
    let status = run(&mut h, &alice, large.clone(), small.clone());
    assert!(!status.is_discarded());
    let status = run(&mut h, &alice, small.clone(), large.clone());
    assert!(status.is_discarded());
    let status = run(&mut h, &alice, large.clone(), large);
    assert!(status.is_discarded());
    let status = run(&mut h, &alice, very_large, small);
    assert!(status.is_discarded());
}

fn run(
    h: &mut MoveHarness,
    account: &Account,
    code: Vec<u8>,
    txn_arg: Vec<u8>,
) -> TransactionStatus {
    let script = Script::new(code, vec![], vec![TransactionArgument::U8Vector(txn_arg)]);

    let txn = TransactionBuilder::new(account.clone())
        .script(script)
        .sequence_number(h.sequence_number(account.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign();

    h.run(txn)
}
