// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    core_mempool::sender_bucket,
    mocks::MockSharedMempool,
    network::BroadcastPeerPriority,
    tests::common::{batch_add_signed_txn, TestTransaction},
    QuorumStoreRequest,
};
use velor_config::config::MempoolConfig;
use velor_consensus_types::common::RejectedTransactionSummary;
use velor_mempool_notifications::MempoolNotificationSender;
use velor_types::{
    transaction::{ReplayProtector, Transaction},
    vm_status::DiscardedVMStatus,
};
use futures::{channel::oneshot, sink::SinkExt};
use tokio::time::timeout;

#[tokio::test]
async fn test_consensus_events_rejected_txns() {
    let smp = MockSharedMempool::new();

    // Add txns 1, 2, 3
    // Txn 1: rejected during execution
    // Txn 2: not committed with different address
    // Txn 3: not committed with same address
    let rejected_txn =
        TestTransaction::new(0, ReplayProtector::SequenceNumber(0), 1).make_signed_transaction();
    let kept_txn_1 =
        TestTransaction::new(1, ReplayProtector::SequenceNumber(0), 1).make_signed_transaction();
    let kept_txn_2 =
        TestTransaction::new(0, ReplayProtector::SequenceNumber(1), 1).make_signed_transaction();
    let txns = vec![rejected_txn.clone(), kept_txn_1.clone(), kept_txn_2.clone()];
    let sender_bucket_1 = sender_bucket(
        &kept_txn_1.sender(),
        MempoolConfig::default().num_sender_buckets,
    );
    let sender_bucket_2 = sender_bucket(
        &kept_txn_2.sender(),
        MempoolConfig::default().num_sender_buckets,
    );
    // Add txns to mempool
    {
        let mut pool = smp.mempool.lock();
        assert!(batch_add_signed_txn(&mut pool, txns).is_ok());
    }

    let transactions = vec![RejectedTransactionSummary {
        sender: rejected_txn.sender(),
        replay_protector: rejected_txn.replay_protector(),
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
    if sender_bucket_1 != sender_bucket_2 {
        let (timeline, _) = pool.read_timeline(
            sender_bucket_1,
            &vec![0; 10].into(),
            10,
            None,
            BroadcastPeerPriority::Primary,
        );
        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline.first().unwrap().0, kept_txn_1);

        let (timeline, _) = pool.read_timeline(
            sender_bucket_2,
            &vec![0; 10].into(),
            10,
            None,
            BroadcastPeerPriority::Primary,
        );
        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline.first().unwrap().0, kept_txn_2);
    } else {
        let (timeline, _) = pool.read_timeline(
            sender_bucket_1,
            &vec![0; 10].into(),
            10,
            None,
            BroadcastPeerPriority::Primary,
        );
        assert_eq!(timeline.len(), 2);
        assert_eq!(timeline[0].0, kept_txn_1);
        assert_eq!(timeline[1].0, kept_txn_2);
    }
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
    let committed_txn = TestTransaction::new(0, ReplayProtector::SequenceNumber(0), 1)
        .make_signed_transaction_with_expiration_time(0);
    let kept_txn =
        TestTransaction::new(1, ReplayProtector::SequenceNumber(0), 1).make_signed_transaction(); // not committed or cleaned out by block timestamp gc
    let txns = vec![
        committed_txn.clone(),
        TestTransaction::new(0, ReplayProtector::SequenceNumber(1), 1)
            .make_signed_transaction_with_expiration_time(0),
        kept_txn.clone(),
    ];
    let sender_bucket = sender_bucket(
        &kept_txn.sender(),
        MempoolConfig::default().num_sender_buckets,
    );
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
        let (timeline, _) = pool.read_timeline(
            sender_bucket,
            &vec![0; 10].into(),
            10,
            None,
            BroadcastPeerPriority::Primary,
        );
        if timeline.len() == 10 && timeline.first().unwrap().0 == kept_txn {
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
