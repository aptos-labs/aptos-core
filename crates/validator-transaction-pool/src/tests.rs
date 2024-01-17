// Copyright © Aptos Foundation

use crate::{TransactionFilter, VTxnPoolState};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_crypto::hash::CryptoHash;
use aptos_types::validator_txn::{
    Topic::{DUMMY1, DUMMY2},
    ValidatorTransaction,
};
use futures_util::StreamExt;
use std::{
    collections::HashSet,
    ops::Add,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::time::timeout;

#[test]
fn txn_pull_order_should_be_fifo_except_in_topic_overwriting() {
    let pool = VTxnPoolState::default();
    let txn_0 = ValidatorTransaction::dummy2(b"txn0".to_vec());
    let txn_1 = ValidatorTransaction::dummy1(b"txn1".to_vec());
    let txn_2 = ValidatorTransaction::dummy2(b"txn2".to_vec());
    let _guard_0 = pool.put(DUMMY2, Arc::new(txn_0.clone()), None);
    let _guard_1 = pool.put(DUMMY1, Arc::new(txn_1.clone()), None);
    let _guard_2 = pool.put(DUMMY2, Arc::new(txn_2.clone()), None); // txn_0 is replaced.
    let pulled = pool.pull(
        Instant::now().add(Duration::from_secs(10)),
        99,
        2048,
        TransactionFilter::default(),
    );
    assert_eq!(vec![txn_1, txn_2], pulled);
}

#[test]
fn delete_by_seq_num() {
    let pool = VTxnPoolState::default();
    let txn_0 = ValidatorTransaction::dummy2(b"txn0".to_vec());
    let txn_1 = ValidatorTransaction::dummy1(b"txn1".to_vec());
    let guard_0 = pool.put(DUMMY2, Arc::new(txn_0.clone()), None);
    let _guard_1 = pool.put(DUMMY1, Arc::new(txn_1.clone()), None);
    drop(guard_0);
    let pulled = pool.pull(
        Instant::now().add(Duration::from_secs(10)),
        99,
        2048,
        TransactionFilter::default(),
    );
    assert_eq!(vec![txn_1], pulled);
}

#[test]
fn txn_should_be_dropped_if_guard_is_dropped() {
    let pool = VTxnPoolState::default();
    let txn_0 = ValidatorTransaction::dummy2(b"txn0".to_vec());
    let txn_1 = ValidatorTransaction::dummy1(b"txn1".to_vec());
    let guard_0 = pool.put(DUMMY2, Arc::new(txn_0.clone()), None);
    let guard_1 = pool.put(DUMMY1, Arc::new(txn_1.clone()), None);
    drop(guard_0);
    drop(guard_1);
    let pulled = pool.pull(
        Instant::now().add(Duration::from_secs(10)),
        99,
        2048,
        TransactionFilter::default(),
    );
    assert!(pulled.is_empty());
}

#[tokio::test]
async fn per_txn_pull_notification() {
    let pool = VTxnPoolState::default();
    let txn_0 = ValidatorTransaction::dummy2(b"txn0".to_vec());
    let txn_1 = ValidatorTransaction::dummy1(b"txn1".to_vec());
    let (tx, mut rx) = aptos_channel::new(QueueStyle::KLAST, 1, None);
    let _guard_0 = pool.put(DUMMY2, Arc::new(txn_0.clone()), None);
    let _guard_1 = pool.put(DUMMY1, Arc::new(txn_1.clone()), Some(tx));
    let notification_received = timeout(Duration::from_millis(100), rx.select_next_some()).await;
    assert!(notification_received.is_err());
    let pulled = pool.pull(
        Instant::now().add(Duration::from_secs(10)),
        99,
        2048,
        TransactionFilter::default(),
    );
    let notification_received = timeout(Duration::from_millis(100), rx.select_next_some()).await;
    assert_eq!(&txn_1, notification_received.unwrap().as_ref());
    assert_eq!(vec![txn_0, txn_1], pulled);
}

#[test]
fn pull_item_limit_should_be_respected() {
    let pool = VTxnPoolState::default();
    let txn_0 = ValidatorTransaction::dummy2(b"txn0".to_vec());
    let txn_1 = ValidatorTransaction::dummy1(b"txn1".to_vec());
    let guard_0 = pool.put(DUMMY2, Arc::new(txn_0.clone()), None);
    let _guard_1 = pool.put(DUMMY1, Arc::new(txn_1.clone()), None);
    let pulled = pool.pull(
        Instant::now().add(Duration::from_secs(10)),
        1,
        2048,
        TransactionFilter::default(),
    );
    assert_eq!(vec![txn_0], pulled);
    drop(guard_0);
    let pulled = pool.pull(
        Instant::now().add(Duration::from_secs(10)),
        1,
        2048,
        TransactionFilter::default(),
    );
    assert_eq!(vec![txn_1], pulled);
}

#[test]
fn pull_size_limit_should_be_respected() {
    let pool = VTxnPoolState::default();
    let txn_0 = ValidatorTransaction::dummy2(vec![0xFF; 100]);
    let txn_1 = ValidatorTransaction::dummy1(vec![0xFF; 100]);
    let guard_0 = pool.put(DUMMY2, Arc::new(txn_0.clone()), None);
    let _guard_1 = pool.put(DUMMY1, Arc::new(txn_1.clone()), None);
    let pulled = pool.pull(
        Instant::now().add(Duration::from_secs(10)),
        99,
        150,
        TransactionFilter::default(),
    );
    assert_eq!(vec![txn_0], pulled);
    drop(guard_0);
    let pulled = pool.pull(
        Instant::now().add(Duration::from_secs(10)),
        99,
        150,
        TransactionFilter::default(),
    );
    assert_eq!(vec![txn_1], pulled);
}

#[test]
fn pull_filter_should_be_respected() {
    let pool = VTxnPoolState::default();
    let txn_0 = ValidatorTransaction::dummy2(vec![0xFF; 100]);
    let txn_1 = ValidatorTransaction::dummy1(vec![0xFF; 100]);
    let _guard_0 = pool.put(DUMMY2, Arc::new(txn_0.clone()), None);
    let _guard_1 = pool.put(DUMMY1, Arc::new(txn_1.clone()), None);
    let pulled = pool.pull(
        Instant::now().add(Duration::from_secs(10)),
        99,
        2048,
        TransactionFilter::PendingTxnHashSet(HashSet::from([txn_0.hash()])),
    );
    assert_eq!(vec![txn_1], pulled);
}
