// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    mocks::MockSharedMempool,
    tests::common::{batch_add_signed_txn, TestTransaction},
    QuorumStoreRequest,
};
use aptos_consensus_types::common::RejectedTransactionSummary;
use aptos_mempool_notifications::MempoolNotificationSender;
use aptos_types::{transaction::Transaction, vm_status::DiscardedVMStatus};
use futures::{channel::oneshot, sink::SinkExt};
use tokio::time::timeout;

#[tokio::test]
async fn test_consensus_events_rejected_txns() {
    let smp = MockSharedMempool::new();

    // Add txns 1, 2, 3
    // Txn 1: rejected during execution
    // Txn 2: not committed with different address
    // Txn 3: not committed with same address
    let rejected_txn = TestTransaction::new(0, 0, 1).make_signed_transaction();
    let kept_txn = TestTransaction::new(1, 0, 1).make_signed_transaction();
    let txns = vec![
        rejected_txn.clone(),
        kept_txn.clone(),
        TestTransaction::new(0, 1, 1).make_signed_transaction(),
    ];
    // Add txns to mempool
    {
        let mut pool = smp.mempool.lock();
        assert!(batch_add_signed_txn(&mut pool, txns).is_ok());
    }

    let transactions = vec![RejectedTransactionSummary {
        sender: rejected_txn.sender(),
        sequence_number: rejected_txn.sequence_number(),
        hash: rejected_txn.committed_hash(),
        reason: DiscardedVMStatus::MALFORMED,
    }];
    let (callback, callback_rcv) = oneshot::channel();
    let req = QuorumStoreRequest::RejectNotification(transactions, callback);
    let mut consensus_sender = smp.consensus_to_mempool_sender.clone();
    assert!(consensus_sender.send(req).await.is_ok());
    assert!(callback_rcv.await.is_ok());

    let pool = smp.mempool.lock();
    // TODO: make less brittle to broadcast buckets changes
    let (timeline, _) = pool.read_timeline(&vec![0; 10].into(), 10);
    assert_eq!(timeline.len(), 2);
    assert_eq!(timeline.first().unwrap(), &kept_txn);
}

#[allow(clippy::await_holding_lock)] // This appears to be a false positive!
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_notify_committed_txns() {
    // Create a new mempool notifier, listener and shared mempool
    let smp = MockSharedMempool::new();

    // Add txns 1, 2, 3
    // Txn 1: committed successfully
    // Txn 2: not committed but older than gc block timestamp
    // Txn 3: not committed and newer than block timestamp
    let committed_txn =
        TestTransaction::new(0, 0, 1).make_signed_transaction_with_expiration_time(0);
    let kept_txn = TestTransaction::new(1, 0, 1).make_signed_transaction(); // not committed or cleaned out by block timestamp gc
    let txns = vec![
        committed_txn.clone(),
        TestTransaction::new(0, 1, 1).make_signed_transaction_with_expiration_time(0),
        kept_txn.clone(),
    ];
    // Add txns to mempool
    {
        let mut pool = smp.mempool.lock();
        assert!(batch_add_signed_txn(&mut pool, txns).is_ok());
    }

    // Notify mempool of the new commit
    let committed_txns = vec![Transaction::UserTransaction(committed_txn)];
    assert!(smp
        .mempool_notifier
        .notify_new_commit(committed_txns, 1)
        .await
        .is_ok());

    // Wait until mempool handles the commit notification
    let wait_for_commit = async {
        let pool = smp.mempool.lock();
        // TODO: make less brittle to broadcast buckets changes
        let (timeline, _) = pool.read_timeline(&vec![0; 10].into(), 10);
        if timeline.len() == 10 && timeline.first().unwrap() == &kept_txn {
            return; // Mempool handled the commit notification
        }
        drop(pool);

        // Sleep for a while
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    };
    if let Err(elasped) = timeout(std::time::Duration::from_secs(5), wait_for_commit).await {
        panic!(
            "Mempool did not receive the commit notification! {:?}",
            elasped
        );
    }
}
