// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{balance_ap, encode_mint_transaction, encode_transfer_transaction, seqnum_ap, MockVM};
use velor_block_executor::txn_provider::default::DefaultTxnProvider;
use velor_types::{
    account_address::AccountAddress,
    bytes::NumToBytes,
    state_store::{state_key::StateKey, MockStateView},
    transaction::signature_verified_transaction::into_signature_verified_block,
    write_set::WriteOp,
};
use velor_vm::VMBlockExecutor;
use std::collections::BTreeMap;

fn gen_address(index: u8) -> AccountAddress {
    AccountAddress::new([index; AccountAddress::LENGTH])
}

#[test]
fn test_mock_vm_different_senders() {
    let amount = 100;
    let mut txns = vec![];
    for i in 0..10 {
        txns.push(encode_mint_transaction(gen_address(i), amount));
    }

    let txn_provider =
        DefaultTxnProvider::new_without_info(into_signature_verified_block(txns.clone()));
    let outputs = MockVM::new()
        .execute_block_no_limit(&txn_provider, &MockStateView::empty())
        .expect("MockVM should not fail to start");

    for (output, txn) in itertools::zip_eq(outputs.iter(), txns.iter()) {
        let sender = txn.try_as_signed_user_txn().unwrap().sender();
        assert_eq!(
            output
                .write_set()
                .write_op_iter()
                .map(|(key, op)| (key.clone(), op.clone()))
                .collect::<BTreeMap<_, _>>(),
            [
                (
                    StateKey::raw(&balance_ap(sender)),
                    WriteOp::legacy_modification(amount.le_bytes()),
                ),
                (
                    StateKey::raw(&seqnum_ap(sender)),
                    WriteOp::legacy_modification(1u64.le_bytes()),
                ),
            ]
            .into_iter()
            .collect()
        );
    }
}

#[test]
fn test_mock_vm_same_sender() {
    let amount = 100;
    let sender = gen_address(1);
    let mut txns = vec![];
    for _i in 0..10 {
        txns.push(encode_mint_transaction(sender, amount));
    }

    let txn_provider = DefaultTxnProvider::new_without_info(into_signature_verified_block(txns));
    let outputs = MockVM::new()
        .execute_block_no_limit(&txn_provider, &MockStateView::empty())
        .expect("MockVM should not fail to start");

    for (i, output) in outputs.iter().enumerate() {
        assert_eq!(
            output
                .write_set()
                .write_op_iter()
                .map(|(key, op)| (key.clone(), op.clone()))
                .collect::<BTreeMap<_, _>>(),
            [
                (
                    StateKey::raw(&balance_ap(sender)),
                    WriteOp::legacy_modification((amount * (i as u64 + 1)).le_bytes()),
                ),
                (
                    StateKey::raw(&seqnum_ap(sender)),
                    WriteOp::legacy_modification((i as u64 + 1).le_bytes()),
                ),
            ]
            .into_iter()
            .collect()
        );
    }
}

#[test]
fn test_mock_vm_payment() {
    let txns = vec![
        encode_mint_transaction(gen_address(0), 100),
        encode_mint_transaction(gen_address(1), 100),
        encode_transfer_transaction(gen_address(0), gen_address(1), 50),
    ];

    let txn_provider = DefaultTxnProvider::new_without_info(into_signature_verified_block(txns));
    let output = MockVM::new()
        .execute_block_no_limit(&txn_provider, &MockStateView::empty())
        .expect("MockVM should not fail to start");

    let mut output_iter = output.iter();
    output_iter.next();
    output_iter.next();
    assert_eq!(
        output_iter
            .next()
            .unwrap()
            .write_set()
            .write_op_iter()
            .map(|(key, op)| (key.clone(), op.clone()))
            .collect::<BTreeMap<_, _>>(),
        [
            (
                StateKey::raw(&balance_ap(gen_address(0))),
                WriteOp::legacy_modification(50u64.le_bytes())
            ),
            (
                StateKey::raw(&seqnum_ap(gen_address(0))),
                WriteOp::legacy_modification(2u64.le_bytes())
            ),
            (
                StateKey::raw(&balance_ap(gen_address(1))),
                WriteOp::legacy_modification(150u64.le_bytes())
            ),
        ]
        .into_iter()
        .collect()
    );
}
