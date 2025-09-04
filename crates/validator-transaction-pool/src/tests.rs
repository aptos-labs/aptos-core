// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{TransactionFilter, VTxnPoolState};
use velor_channels::{velor_channel, message_queues::QueueStyle};
use velor_crypto::hash::CryptoHash;
use velor_types::{
    dkg::DKGTranscript,
    jwks::{dummy_issuer, QuorumCertifiedUpdate},
    validator_txn::{Topic, ValidatorTransaction},
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
    let txn_0 = ValidatorTransaction::DKGResult(DKGTranscript::dummy());
    let txn_1 = ValidatorTransaction::ObservedJWKUpdate(QuorumCertifiedUpdate::dummy());
    let txn_2 = ValidatorTransaction::DKGResult(DKGTranscript::dummy());
    let _guard_0 = pool.put(
        Topic::JWK_CONSENSUS(dummy_issuer()),
        Arc::new(txn_0.clone()),
        None,
    );
    let _guard_1 = pool.put(Topic::DKG, Arc::new(txn_1.clone()), None);
    let _guard_2 = pool.put(
        Topic::JWK_CONSENSUS(dummy_issuer()),
        Arc::new(txn_2.clone()),
        None,
    ); // txn_0 is replaced.
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
    let txn_0 = ValidatorTransaction::ObservedJWKUpdate(QuorumCertifiedUpdate::dummy());
    let txn_1 = ValidatorTransaction::DKGResult(DKGTranscript::dummy());
    let guard_0 = pool.put(
        Topic::JWK_CONSENSUS(dummy_issuer()),
        Arc::new(txn_0.clone()),
        None,
    );
    let _guard_1 = pool.put(Topic::DKG, Arc::new(txn_1.clone()), None);
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
    let txn_0 = ValidatorTransaction::ObservedJWKUpdate(QuorumCertifiedUpdate::dummy());
    let txn_1 = ValidatorTransaction::DKGResult(DKGTranscript::dummy());
    let guard_0 = pool.put(
        Topic::JWK_CONSENSUS(dummy_issuer()),
        Arc::new(txn_0.clone()),
        None,
    );
    let guard_1 = pool.put(Topic::DKG, Arc::new(txn_1.clone()), None);
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
    let txn_0 = ValidatorTransaction::ObservedJWKUpdate(QuorumCertifiedUpdate::dummy());
    let txn_1 = ValidatorTransaction::DKGResult(DKGTranscript::dummy());
    let (tx, mut rx) = velor_channel::new(QueueStyle::KLAST, 1, None);
    let _guard_0 = pool.put(
        Topic::JWK_CONSENSUS(dummy_issuer()),
        Arc::new(txn_0.clone()),
        None,
    );
    let _guard_1 = pool.put(Topic::DKG, Arc::new(txn_1.clone()), Some(tx));
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
    let txn_0 = ValidatorTransaction::ObservedJWKUpdate(QuorumCertifiedUpdate::dummy());
    let txn_1 = ValidatorTransaction::DKGResult(DKGTranscript::dummy());
    let guard_0 = pool.put(
        Topic::JWK_CONSENSUS(dummy_issuer()),
        Arc::new(txn_0.clone()),
        None,
    );
    let _guard_1 = pool.put(Topic::DKG, Arc::new(txn_1.clone()), None);
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
    let txn_0 = ValidatorTransaction::dummy(vec![0xFF; 100]);
    let txn_1 = ValidatorTransaction::dummy(vec![0xFF; 100]);
    let guard_0 = pool.put(
        Topic::JWK_CONSENSUS(dummy_issuer()),
        Arc::new(txn_0.clone()),
        None,
    );
    let _guard_1 = pool.put(Topic::DKG, Arc::new(txn_1.clone()), None);
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
    let txn_0 = ValidatorTransaction::ObservedJWKUpdate(QuorumCertifiedUpdate::dummy());
    let txn_1 = ValidatorTransaction::dummy(vec![0xFF; 100]);
    let _guard_0 = pool.put(
        Topic::JWK_CONSENSUS(dummy_issuer()),
        Arc::new(txn_0.clone()),
        None,
    );
    let _guard_1 = pool.put(Topic::DKG, Arc::new(txn_1.clone()), None);
    let pulled = pool.pull(
        Instant::now().add(Duration::from_secs(10)),
        99,
        2048,
        TransactionFilter::PendingTxnHashSet(HashSet::from([txn_0.hash()])),
    );
    assert_eq!(vec![txn_1], pulled);
}
