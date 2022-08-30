// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use super::{balance_ap, encode_mint_transaction, encode_transfer_transaction, seqnum_ap, MockVM};
use anyhow::Result;
use aptos_state_view::StateView;
use aptos_types::state_store::state_storage_usage::StateStorageUsage;
use aptos_types::{
    account_address::AccountAddress, state_store::state_key::StateKey, write_set::WriteOp,
};
use aptos_vm::VMExecutor;

fn gen_address(index: u8) -> AccountAddress {
    AccountAddress::new([index; AccountAddress::LENGTH])
}

struct MockStateView;

impl StateView for MockStateView {
    fn get_state_value(&self, _state_key: &StateKey) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }

    fn is_genesis(&self) -> bool {
        false
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        Ok(StateStorageUsage::new_untracked())
    }
}

#[test]
fn test_mock_vm_different_senders() {
    let amount = 100;
    let mut txns = vec![];
    for i in 0..10 {
        txns.push(encode_mint_transaction(gen_address(i), amount));
    }

    let outputs = MockVM::execute_block(txns.clone(), &MockStateView)
        .expect("MockVM should not fail to start");

    for (output, txn) in itertools::zip_eq(outputs.iter(), txns.iter()) {
        let sender = txn.as_signed_user_txn().unwrap().sender();
        assert_eq!(
            output
                .write_set()
                .iter()
                .map(|(key, op)| (key.clone(), op.clone()))
                .collect::<BTreeMap<_, _>>(),
            [
                (
                    StateKey::AccessPath(balance_ap(sender)),
                    WriteOp::Modification(amount.to_le_bytes().to_vec())
                ),
                (
                    StateKey::AccessPath(seqnum_ap(sender)),
                    WriteOp::Modification(1u64.to_le_bytes().to_vec())
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

    let outputs =
        MockVM::execute_block(txns, &MockStateView).expect("MockVM should not fail to start");

    for (i, output) in outputs.iter().enumerate() {
        assert_eq!(
            output
                .write_set()
                .iter()
                .map(|(key, op)| (key.clone(), op.clone()))
                .collect::<BTreeMap<_, _>>(),
            [
                (
                    StateKey::AccessPath(balance_ap(sender)),
                    WriteOp::Modification((amount * (i as u64 + 1)).to_le_bytes().to_vec())
                ),
                (
                    StateKey::AccessPath(seqnum_ap(sender)),
                    WriteOp::Modification((i as u64 + 1).to_le_bytes().to_vec())
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

    let output =
        MockVM::execute_block(txns, &MockStateView).expect("MockVM should not fail to start");

    let mut output_iter = output.iter();
    output_iter.next();
    output_iter.next();
    assert_eq!(
        output_iter
            .next()
            .unwrap()
            .write_set()
            .iter()
            .map(|(key, op)| (key.clone(), op.clone()))
            .collect::<BTreeMap<_, _>>(),
        [
            (
                StateKey::AccessPath(balance_ap(gen_address(0))),
                WriteOp::Modification(50u64.to_le_bytes().to_vec())
            ),
            (
                StateKey::AccessPath(seqnum_ap(gen_address(0))),
                WriteOp::Modification(2u64.to_le_bytes().to_vec())
            ),
            (
                StateKey::AccessPath(balance_ap(gen_address(1))),
                WriteOp::Modification(150u64.to_le_bytes().to_vec())
            ),
        ]
        .into_iter()
        .collect()
    );
}
