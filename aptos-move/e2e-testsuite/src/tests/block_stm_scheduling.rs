// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_language_e2e_tests::{common_transactions::peer_to_peer_txn, executor::FakeExecutor};
use aptos_types::transaction::{ExecutionStatus, TransactionStatus};

/// One sender's transfers in a block form a chain through its account and
/// store. The scheduling flags must leave the parallel output equal to the
/// sequential one.
#[test]
fn same_sender_chain_with_scheduling_flags() {
    // SAFETY: nothing else runs in this test process before the flags are read.
    unsafe {
        std::env::set_var("BLOCK_STM_PREDICTIVE_STALL", "1");
        std::env::set_var("BLOCK_STM_SENDER_HINTS", "1");
        std::env::set_var("BLOCK_STM_IDLE_BACKOFF", "1");
    }
    let mut executor = FakeExecutor::from_head_genesis().set_parallel();
    let accounts = executor.create_accounts(65, 100_000_000, 10);
    let sender = &accounts[0];
    let txns = accounts[1..]
        .iter()
        .enumerate()
        .map(|(i, receiver)| peer_to_peer_txn(sender, receiver, 10 + i as u64, 1_000, 0))
        .collect::<Vec<_>>();

    let output = executor.execute_block(txns).unwrap();
    assert_eq!(output.len(), 64);
    for txn_output in &output {
        assert_eq!(
            txn_output.status(),
            &TransactionStatus::Keep(ExecutionStatus::Success)
        );
    }
}
