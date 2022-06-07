// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use aptos_types::{
    account_address::AccountAddress, block_metadata::BlockMetadata, transaction::Transaction,
};
use language_e2e_tests::executor::FakeExecutor;

#[test]
fn block_prologue_abort() {
    let mut executor = FakeExecutor::from_fresh_genesis();
    // ensure normal metadata can succeed
    executor.new_block();
    // this transaction will fail
    let new_block_txn = BlockMetadata::new(
        HashValue::zero(),
        0,
        0,
        vec![false; 1],
        AccountAddress::random(),
        executor.get_block_time(),
    );
    // the result should aborted transaction but not aborted vm.
    let mut outputs = executor
        .execute_transaction_block(vec![Transaction::BlockMetadata(new_block_txn)])
        .unwrap();
    assert_eq!(outputs.len(), 1);
    let output = outputs.pop().unwrap();
    assert!(output.write_set().is_empty());
    assert!(output.events().is_empty());
    assert_eq!(output.gas_used(), 0);
}
