// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    core_mempool::{sender_bucket, CoreMempool, MempoolTransaction, SubmittedBy, TimelineState},
    network::BroadcastPeerPriority,
    tests::common::{
        add_signed_txn, add_txn, add_txns_to_mempool, setup_mempool,
        setup_mempool_with_broadcast_buckets, txn_bytes_len, TestTransaction,
    },
};
use aptos_config::config::{MempoolConfig, NodeConfig};
use aptos_consensus_types::common::{TransactionInProgress, TransactionSummary};
use aptos_crypto::HashValue;
use aptos_types::{
    account_address::AccountAddress,
    mempool_status::MempoolStatusCode,
    transaction::{ReplayProtector, SignedTransaction},
    vm_status::DiscardedVMStatus,
};
use itertools::Itertools;
use maplit::btreemap;
use std::time::{Duration, Instant, SystemTime};

#[test]
fn test_transaction_ordering_only_seqnos() {
    let (mut mempool, mut consensus) = setup_mempool();

    // Default ordering: gas price
    let mut transactions = add_txns_to_mempool(&mut mempool, vec![
        TestTransaction::new(0, ReplayProtector::SequenceNumber(0), 3),
        TestTransaction::new(1, ReplayProtector::SequenceNumber(0), 5),
    ]);
    assert_eq!(
        consensus.get_block(&mut mempool, 1, 1024),
        vec!(transactions[1].clone())
    );
    assert_eq!(
        consensus.get_block(&mut mempool, 1, 1024),
        vec!(transactions[0].clone())
    );

    // Second level ordering: expiration time
    let (mut mempool, mut consensus) = setup_mempool();
    transactions = add_txns_to_mempool(&mut mempool, vec![
        TestTransaction::new(0, ReplayProtector::SequenceNumber(0), 1),
        TestTransaction::new(1, ReplayProtector::SequenceNumber(0), 1),
    ]);
    for transaction in &transactions {
        assert_eq!(consensus.get_block(&mut mempool, 1, 1024), vec![
            transaction.clone()
        ]);
    }

    // Last level: for same account it should be by sequence number
    let (mut mempool, mut consensus) = setup_mempool();
    transactions = add_txns_to_mempool(&mut mempool, vec![
        TestTransaction::new(1, ReplayProtector::SequenceNumber(0), 7),
        TestTransaction::new(1, ReplayProtector::SequenceNumber(1), 5),
        TestTransaction::new(1, ReplayProtector::SequenceNumber(2), 1),
        TestTransaction::new(1, ReplayProtector::SequenceNumber(3), 6),
    ]);
    for transaction in &transactions {
        assert_eq!(consensus.get_block(&mut mempool, 1, 1024), vec![
            transaction.clone()
        ]);
    }
}

#[test]
fn test_transaction_ordering_seqnos_and_nonces() {
    let (mut mempool, mut consensus) = setup_mempool();

    // Default ordering: gas price
    add_txns_to_mempool(&mut mempool, vec![
        TestTransaction::new(0, ReplayProtector::Nonce(150), 3),
        TestTransaction::new(0, ReplayProtector::SequenceNumber(0), 3),
        TestTransaction::new(0, ReplayProtector::SequenceNumber(1), 5),
        TestTransaction::new(0, ReplayProtector::Nonce(100), 2),
        TestTransaction::new(0, ReplayProtector::Nonce(200), 7),
    ]);

    assert_eq!(mempool.transactions.priority_index.size(), 5);
    assert_eq!(
        mempool
            .transactions
            .transactions
            .values()
            .map(|account_txns| account_txns.len())
            .sum::<usize>(),
        5
    );

    // Expected transaction order in priority queue
    let ordered_transactions = vec![
        TestTransaction::new(0, ReplayProtector::Nonce(200), 7),
        TestTransaction::new(0, ReplayProtector::SequenceNumber(1), 5),
        TestTransaction::new(0, ReplayProtector::Nonce(150), 3),
        TestTransaction::new(0, ReplayProtector::SequenceNumber(0), 3),
        TestTransaction::new(0, ReplayProtector::Nonce(100), 2),
    ];

    for (i, ordered_key) in mempool.transactions.priority_index.iter().enumerate() {
        assert_eq!(
            ordered_transactions[i].replay_protector,
            ordered_key.replay_protector
        );
        assert_eq!(
            ordered_transactions[i].gas_price,
            ordered_key.gas_ranking_score
        );
    }

    // Expected order of retrieval in consensus
    let retrieved_transactions = vec![
        TestTransaction::new(0, ReplayProtector::Nonce(200), 7),
        TestTransaction::new(0, ReplayProtector::Nonce(150), 3),
        TestTransaction::new(0, ReplayProtector::SequenceNumber(0), 3),
        TestTransaction::new(0, ReplayProtector::SequenceNumber(1), 5),
        TestTransaction::new(0, ReplayProtector::Nonce(100), 2),
    ];

    for transaction in &retrieved_transactions {
        let txn = consensus.get_block(&mut mempool, 1, 1024);
        assert_eq!(txn[0].replay_protector(), transaction.replay_protector);
        assert_eq!(txn[0].gas_unit_price(), transaction.gas_price);
    }
}

#[test]
fn test_transaction_metrics() {
    let (mut mempool, _) = setup_mempool();

    let txn =
        TestTransaction::new(0, ReplayProtector::SequenceNumber(0), 1).make_signed_transaction();
    mempool.add_txn(
        txn.clone(),
        txn.gas_unit_price(),
        Some(0),
        TimelineState::NotReady,
        false,
        None,
        Some(BroadcastPeerPriority::Primary),
    );
    let txn =
        TestTransaction::new(1, ReplayProtector::SequenceNumber(0), 1).make_signed_transaction();
    mempool.add_txn(
        txn.clone(),
        txn.gas_unit_price(),
        Some(0),
        TimelineState::NonQualified,
        false,
        None,
        Some(BroadcastPeerPriority::Primary),
    );
    let txn =
        TestTransaction::new(2, ReplayProtector::SequenceNumber(0), 1).make_signed_transaction();
    mempool.add_txn(
        txn.clone(),
        txn.gas_unit_price(),
        Some(0),
        TimelineState::NotReady,
        true,
        None,
        Some(BroadcastPeerPriority::Primary),
    );

    // Check timestamp returned as end-to-end for broadcast-able transaction
    let (insertion_info, _bucket, _priority) = mempool
        .get_transaction_store()
        .get_insertion_info_and_bucket(
            &TestTransaction::get_address(0),
            ReplayProtector::SequenceNumber(0),
        )
        .unwrap();
    assert_eq!(insertion_info.submitted_by, SubmittedBy::Downstream);

    // Check timestamp returned as not end-to-end for non-broadcast-able transaction
    let (insertion_info, _bucket, _priority) = mempool
        .get_transaction_store()
        .get_insertion_info_and_bucket(
            &TestTransaction::get_address(1),
            ReplayProtector::SequenceNumber(0),
        )
        .unwrap();
    assert_eq!(insertion_info.submitted_by, SubmittedBy::PeerValidator);

    let (insertion_info, _bucket, _priority) = mempool
        .get_transaction_store()
        .get_insertion_info_and_bucket(
            &TestTransaction::get_address(2),
            ReplayProtector::SequenceNumber(0),
        )
        .unwrap();
    assert_eq!(insertion_info.submitted_by, SubmittedBy::Client);
}

#[test]
fn test_update_transaction_in_mempool() {
    let (mut mempool, mut consensus) = setup_mempool();
    let txns = add_txns_to_mempool(&mut mempool, vec![
        TestTransaction::new(0, ReplayProtector::SequenceNumber(0), 1),
        TestTransaction::new(1, ReplayProtector::SequenceNumber(0), 2),
        TestTransaction::new(2, ReplayProtector::Nonce(123), 3),
    ]);
    let fixed_txns = add_txns_to_mempool(&mut mempool, vec![
        TestTransaction::new(0, ReplayProtector::SequenceNumber(0), 5),
        TestTransaction::new(2, ReplayProtector::Nonce(123), 5),
    ]);

    // Check that higher gas price transactions removes lower gas price transactions.
    assert_eq!(
        mempool
            .transactions
            .transactions
            .values()
            .map(|account_txns| account_txns.len())
            .sum::<usize>(),
        3
    );
    assert_eq!(mempool.transactions.priority_index.size(), 3);
    assert_eq!(consensus.get_block(&mut mempool, 1, 1024), vec![fixed_txns
        [0]
    .clone()]);
    assert_eq!(consensus.get_block(&mut mempool, 1, 1024), vec![fixed_txns
        [1]
    .clone()]);
    assert_eq!(consensus.get_block(&mut mempool, 1, 1024), vec![
        txns[1].clone()
    ]);
}

#[test]
fn test_ignore_same_transaction_submitted_to_mempool() {
    let (mut mempool, _) = setup_mempool();
    let _ = add_txns_to_mempool(&mut mempool, vec![
        TestTransaction::new(0, ReplayProtector::SequenceNumber(0), 0),
        TestTransaction::new(0, ReplayProtector::Nonce(123), 1),
    ]);
    let ret = add_txn(
        &mut mempool,
        TestTransaction::new(0, ReplayProtector::SequenceNumber(0), 0),
    );
    assert!(ret.is_ok());
    let ret = add_txn(
        &mut mempool,
        TestTransaction::new(0, ReplayProtector::Nonce(123), 1),
    );
    assert!(ret.is_ok());
    assert_eq!(
        mempool
            .transactions
            .transactions
            .values()
            .map(|account_txns| account_txns.len())
            .sum::<usize>(),
        2
    );
}

#[test]
fn test_fail_for_same_gas_amount_and_not_same_expiration_time() {
    let (mut mempool, _) = setup_mempool();
    let _ = add_txns_to_mempool(&mut mempool, vec![
        TestTransaction::new(0, ReplayProtector::SequenceNumber(0), 0),
        TestTransaction::new(0, ReplayProtector::Nonce(123), 1),
    ]);
    let txn = TestTransaction::new(0, ReplayProtector::SequenceNumber(0), 0)
        .make_signed_transaction_with_expiration_time(u64::MAX - 1000);
    let ret = add_signed_txn(&mut mempool, txn);
    assert!(ret.is_err());

    let txn = TestTransaction::new(0, ReplayProtector::Nonce(123), 1)
        .make_signed_transaction_with_expiration_time(u64::MAX - 1000);
    let ret = add_signed_txn(&mut mempool, txn);
    assert!(ret.is_err());
}

#[test]
fn test_update_invalid_transaction_in_mempool() {
    let (mut mempool, mut consensus) = setup_mempool();
    let txns = add_txns_to_mempool(&mut mempool, vec![
        TestTransaction::new(0, ReplayProtector::SequenceNumber(0), 1),
        TestTransaction::new(1, ReplayProtector::SequenceNumber(0), 2),
    ]);
    let updated_txn = TestTransaction::make_signed_transaction_with_max_gas_amount(
        &TestTransaction::new(0, ReplayProtector::SequenceNumber(0), 5),
        200,
    );
    let _added_tnx = add_signed_txn(&mut mempool, updated_txn);

    // Since both gas price and mas gas amount were updated, the ordering should not have changed.
    // The second transaction with gas price 2 should come first.
    assert_eq!(consensus.get_block(&mut mempool, 1, 1024), vec![
        txns[1].clone()
    ]);
    let next_tnx = consensus.get_block(&mut mempool, 1, 1024);
    assert_eq!(next_tnx, vec![txns[0].clone()]);
    assert_eq!(next_tnx[0].gas_unit_price(), 1);
}

#[test]
fn test_commit_transaction() {
    let (mut pool, mut consensus) = setup_mempool();

    // Test normal flow.
    let txns = add_txns_to_mempool(&mut pool, vec![
        TestTransaction::new(0, ReplayProtector::Nonce(123), 1),
        TestTransaction::new(0, ReplayProtector::SequenceNumber(0), 1),
        TestTransaction::new(0, ReplayProtector::SequenceNumber(1), 2),
        TestTransaction::new(1, ReplayProtector::SequenceNumber(5), 12),
        TestTransaction::new(1, ReplayProtector::Nonce(123), 12),
        TestTransaction::new(2, ReplayProtector::Nonce(123), 2),
        TestTransaction::new(1, ReplayProtector::SequenceNumber(0), 4),
    ]);
    assert_eq!(
        pool.transactions
            .transactions
            .values()
            .map(|account_txns| account_txns.len())
            .sum::<usize>(),
        7
    );
    assert_eq!(pool.transactions.priority_index.size(), 6);
    // Transaction with sequence number 5 goes to parking lot
    assert_eq!(pool.get_parking_lot_size(), 1);
    // Nonce based transactions won't create an account_sequence_numbers entry
    assert_eq!(pool.transactions.account_sequence_numbers.len(), 2);
    // All account sequence numbers are initialized to 0 before transactions are committed
    assert_eq!(
        *pool
            .transactions
            .account_sequence_numbers
            .get(&TestTransaction::get_address(0))
            .unwrap(),
        0
    );
    assert_eq!(
        *pool
            .transactions
            .account_sequence_numbers
            .get(&TestTransaction::get_address(1))
            .unwrap(),
        0
    );

    for txn in txns {
        pool.commit_transaction(&txn.sender(), txn.replay_protector());
    }
    assert_eq!(pool.transactions.priority_index.size(), 0);
    assert_eq!(
        pool.transactions
            .transactions
            .values()
            .map(|account_txns| account_txns.len())
            .sum::<usize>(),
        0
    );
    // Committed transactions are removed parking lot as well
    assert_eq!(pool.get_parking_lot_size(), 0);
    // By the time (sender 1, seq number 0) is committed, the commitment of previous transaction (sender, seq number 1)
    // already removes (sender 1, seq number 0) from mempool. The commitment of (sender 1, seq number 0) adds account_sequence_numbers
    // entry for sender 1, but doesn't remove any data from indices.
    assert_eq!(pool.transactions.account_sequence_numbers.len(), 1);
    assert_eq!(
        *pool
            .transactions
            .account_sequence_numbers
            .get(&TestTransaction::get_address(1))
            .unwrap(),
        1
    );

    let new_txns = add_txns_to_mempool(&mut pool, vec![
        TestTransaction::new(1, ReplayProtector::SequenceNumber(0), 3),
        TestTransaction::new(1, ReplayProtector::SequenceNumber(1), 4),
        TestTransaction::new(1, ReplayProtector::SequenceNumber(5), 12),
        TestTransaction::new(2, ReplayProtector::Nonce(123), 3),
    ]);
    // (sender 1, seq number 0) is not inserted
    // (sender 1, seq number 1), (sender 2, nonce 123) are in priority index
    // (sender 1, seq number 5) is in parking lot
    assert_eq!(pool.transactions.priority_index.size(), 2);
    assert_eq!(pool.get_parking_lot_size(), 1);
    assert_eq!(
        pool.transactions
            .transactions
            .values()
            .map(|account_txns| account_txns.len())
            .sum::<usize>(),
        3
    );
    // Should return only txns from new_txns.
    assert_eq!(
        consensus.get_block(&mut pool, 1, 1024),
        vec!(new_txns[1].clone())
    );
    assert_eq!(
        consensus.get_block(&mut pool, 1, 1024),
        vec!(new_txns[3].clone())
    );

    // Consensus fetch doesn't remove transactions from parking lot or priority index.
    assert_eq!(pool.get_parking_lot_size(), 1);
    assert_eq!(pool.transactions.priority_index.size(), 2);
    assert_eq!(
        pool.transactions
            .transactions
            .values()
            .map(|account_txns| account_txns.len())
            .sum::<usize>(),
        3
    );

    // (sender 1, seq number 5) is parking lot. After committing sequence number 4, it should be moved to mempool.
    pool.commit_transaction(&new_txns[2].sender(), ReplayProtector::SequenceNumber(4));
    assert_eq!(pool.get_parking_lot_size(), 0);
    assert_eq!(pool.transactions.priority_index.size(), 2);
    assert_eq!(
        consensus.get_block(&mut pool, 1, 1024),
        vec!(new_txns[2].clone())
    );
}

#[test]
fn test_reject_transaction() {
    let (mut pool, _) = setup_mempool();

    let txns = add_txns_to_mempool(&mut pool, vec![
        TestTransaction::new(0, ReplayProtector::SequenceNumber(0), 1),
        TestTransaction::new(0, ReplayProtector::SequenceNumber(1), 2),
    ]);

    // reject with wrong hash should have no effect
    pool.reject_transaction(
        &TestTransaction::get_address(0),
        ReplayProtector::SequenceNumber(0),
        &txns[1].committed_hash(), // hash of other txn
        &DiscardedVMStatus::MALFORMED,
    );
    assert!(pool
        .get_transaction_store()
        .get(
            &TestTransaction::get_address(0),
            ReplayProtector::SequenceNumber(0)
        )
        .is_some());
    pool.reject_transaction(
        &TestTransaction::get_address(0),
        ReplayProtector::SequenceNumber(1),
        &txns[0].committed_hash(), // hash of other txn
        &DiscardedVMStatus::MALFORMED,
    );
    assert!(pool
        .get_transaction_store()
        .get(
            &TestTransaction::get_address(0),
            ReplayProtector::SequenceNumber(1)
        )
        .is_some());

    // reject with sequence number too new should have no effect
    // reject with wrong hash should have no effect
    pool.reject_transaction(
        &TestTransaction::get_address(0),
        ReplayProtector::SequenceNumber(0),
        &txns[0].committed_hash(),
        &DiscardedVMStatus::SEQUENCE_NUMBER_TOO_NEW,
    );
    assert!(pool
        .get_transaction_store()
        .get(
            &TestTransaction::get_address(0),
            ReplayProtector::SequenceNumber(0)
        )
        .is_some());
    pool.reject_transaction(
        &TestTransaction::get_address(0),
        ReplayProtector::SequenceNumber(1),
        &txns[1].committed_hash(),
        &DiscardedVMStatus::SEQUENCE_NUMBER_TOO_NEW,
    );
    assert!(pool
        .get_transaction_store()
        .get(
            &TestTransaction::get_address(0),
            ReplayProtector::SequenceNumber(1)
        )
        .is_some());

    // reject with correct hash should have effect
    pool.reject_transaction(
        &TestTransaction::get_address(0),
        ReplayProtector::SequenceNumber(0),
        &txns[0].committed_hash(),
        &DiscardedVMStatus::MALFORMED,
    );
    assert!(pool
        .get_transaction_store()
        .get(
            &TestTransaction::get_address(0),
            ReplayProtector::SequenceNumber(0)
        )
        .is_none());
    pool.reject_transaction(
        &TestTransaction::get_address(0),
        ReplayProtector::SequenceNumber(1),
        &txns[1].committed_hash(),
        &DiscardedVMStatus::MALFORMED,
    );
    assert!(pool
        .get_transaction_store()
        .get(
            &TestTransaction::get_address(0),
            ReplayProtector::SequenceNumber(1)
        )
        .is_none());
}

#[test]
fn test_system_ttl() {
    // Created mempool with system_transaction_timeout = 0.
    // All transactions are supposed to be evicted on next gc run.
    let mut config = NodeConfig::generate_random_config();
    config.mempool.system_transaction_timeout_secs = 0;
    let mut mempool = CoreMempool::new(&config);

    add_txn(
        &mut mempool,
        TestTransaction::new(0, ReplayProtector::SequenceNumber(0), 10),
    )
    .unwrap();

    // Reset system ttl timeout.
    mempool.system_transaction_timeout = Duration::from_secs(10);
    // Add new transaction. Should be valid for 10 seconds.
    let transaction = TestTransaction::new(1, ReplayProtector::SequenceNumber(0), 1);
    add_txn(&mut mempool, transaction.clone()).unwrap();

    // GC routine should clear transaction from first insert but keep last one.
    mempool.gc();
    let batch = mempool.get_batch(1, 1024, true, btreemap![]);
    assert_eq!(vec![transaction.make_signed_transaction()], batch);
}

#[test]
fn test_commit_callback() {
    // Consensus commit callback should unlock txns in parking lot.
    let mut pool = setup_mempool().0;
    // Insert transaction with sequence number 6 to pool (while last known executed transaction is 0).
    let txns = add_txns_to_mempool(&mut pool, vec![TestTransaction::new(
        1,
        ReplayProtector::SequenceNumber(6),
        1,
    )]);

    // Check that pool is empty.
    assert!(pool.get_batch(1, 1024, true, btreemap![]).is_empty());
    // Transaction 5 got back from consensus.
    pool.commit_transaction(
        &TestTransaction::get_address(1),
        ReplayProtector::SequenceNumber(5),
    );
    // Verify that we can execute transaction 6.
    assert_eq!(pool.get_batch(1, 1024, true, btreemap![])[0], txns[0]);
}

#[test]
fn test_reset_sequence_number_on_failure() {
    let mut pool = setup_mempool().0;
    let txns = [
        TestTransaction::new(1, ReplayProtector::SequenceNumber(0), 1),
        TestTransaction::new(1, ReplayProtector::SequenceNumber(1), 1),
    ];
    let hashes: Vec<_> = txns
        .iter()
        .cloned()
        .map(|txn| txn.make_signed_transaction().committed_hash())
        .collect();
    // Add two transactions for account.
    add_txns_to_mempool(&mut pool, vec![
        TestTransaction::new(1, ReplayProtector::SequenceNumber(0), 1),
        TestTransaction::new(1, ReplayProtector::SequenceNumber(1), 1),
    ]);

    // Notify mempool about failure in arbitrary order
    pool.reject_transaction(
        &TestTransaction::get_address(1),
        ReplayProtector::SequenceNumber(0),
        &hashes[0],
        &DiscardedVMStatus::MALFORMED,
    );
    pool.reject_transaction(
        &TestTransaction::get_address(1),
        ReplayProtector::SequenceNumber(1),
        &hashes[1],
        &DiscardedVMStatus::MALFORMED,
    );

    // Verify that new transaction for this account can be added.
    assert!(add_txn(
        &mut pool,
        TestTransaction::new(1, ReplayProtector::SequenceNumber(0), 1)
    )
    .is_ok());
}

fn view(txns: Vec<(SignedTransaction, u64)>) -> Vec<u64> {
    txns.iter()
        .map(|(txn, _)| txn.sequence_number())
        .sorted()
        .collect()
}

#[test]
fn test_timeline() {
    let mut pool = setup_mempool().0;
    let txns = add_txns_to_mempool(&mut pool, vec![
        TestTransaction::new(1, ReplayProtector::SequenceNumber(0), 1),
        TestTransaction::new(1, ReplayProtector::SequenceNumber(1), 1),
        TestTransaction::new(1, ReplayProtector::SequenceNumber(3), 1),
        TestTransaction::new(1, ReplayProtector::SequenceNumber(5), 1),
    ]);
    let sender_bucket = sender_bucket(
        &txns[0].sender(),
        MempoolConfig::default().num_sender_buckets,
    );

    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![0].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![0, 1]);
    // Txns 3 and 5 should be in parking lot.
    assert_eq!(2, pool.get_parking_lot_size());

    // Add txn 2 to unblock txn3.
    add_txns_to_mempool(&mut pool, vec![TestTransaction::new(
        1,
        ReplayProtector::SequenceNumber(2),
        1,
    )]);
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![0].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![0, 1, 2, 3]);
    // Txn 5 should be in parking lot.
    assert_eq!(1, pool.get_parking_lot_size());

    // Try different start read position.
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![2].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![2, 3]);

    // Simulate callback from consensus to unblock txn 5.
    pool.commit_transaction(
        &TestTransaction::get_address(1),
        ReplayProtector::SequenceNumber(4),
    );
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![0].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![5]);
    // check parking lot is empty
    assert_eq!(0, pool.get_parking_lot_size());
}

#[test]
fn test_timeline_before() {
    let mut pool = setup_mempool().0;
    let txns = add_txns_to_mempool(&mut pool, vec![
        TestTransaction::new(1, ReplayProtector::SequenceNumber(0), 1),
        TestTransaction::new(1, ReplayProtector::SequenceNumber(1), 1),
        TestTransaction::new(1, ReplayProtector::SequenceNumber(3), 1),
        TestTransaction::new(1, ReplayProtector::SequenceNumber(5), 1),
    ]);
    let sender_bucket = sender_bucket(
        &txns[0].sender(),
        MempoolConfig::default().num_sender_buckets,
    );
    let insertion_done_time = Instant::now();

    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![0].into(),
        10,
        Some(insertion_done_time - Duration::from_millis(200)),
        BroadcastPeerPriority::Primary,
    );
    assert!(timeline.is_empty());

    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![0].into(),
        10,
        Some(insertion_done_time),
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![0, 1]);

    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![0].into(),
        10,
        Some(insertion_done_time + Duration::from_millis(200)),
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![0, 1]);
}

#[test]
fn test_multi_bucket_timeline() {
    let mut pool = setup_mempool_with_broadcast_buckets(vec![0, 101, 201]).0;
    let txns = add_txns_to_mempool(&mut pool, vec![
        TestTransaction::new(1, ReplayProtector::SequenceNumber(0), 1), // bucket 0
        TestTransaction::new(1, ReplayProtector::SequenceNumber(1), 100), // bucket 0
        TestTransaction::new(1, ReplayProtector::SequenceNumber(3), 200), // bucket 1
        TestTransaction::new(1, ReplayProtector::SequenceNumber(5), 300), // bucket 2
    ]);
    let sender_bucket = sender_bucket(
        &txns[0].sender(),
        MempoolConfig::default().num_sender_buckets,
    );

    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![0, 0, 0].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![0, 1]);
    // Txns 3 and 5 should be in parking lot.
    assert_eq!(2, pool.get_parking_lot_size());

    // Add txn 2 to unblock txn3.
    add_txns_to_mempool(&mut pool, vec![TestTransaction::new(
        1,
        ReplayProtector::SequenceNumber(2),
        1,
    )]);
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![0, 0, 0].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![0, 1, 2, 3]);
    // Txn 5 should be in parking lot.
    assert_eq!(1, pool.get_parking_lot_size());

    // Try different start read positions. Expected buckets: [[0, 1, 2], [3], []]
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![1, 0, 0].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![1, 2, 3]);
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![2, 0, 0].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![2, 3]);
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![0, 1, 0].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![0, 1, 2]);
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![1, 1, 0].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![1, 2]);
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![2, 1, 0].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![2]);
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![3, 0, 0].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![3]);
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![3, 1, 0].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert!(view(timeline).is_empty());

    // Ensure high gas is prioritized.
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![0, 0, 0].into(),
        1,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![3]);

    // Simulate callback from consensus to unblock txn 5.
    pool.commit_transaction(
        &TestTransaction::get_address(1),
        ReplayProtector::SequenceNumber(4),
    );
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![0, 0, 0].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![5]);
    // check parking lot is empty
    assert_eq!(0, pool.get_parking_lot_size());
}

#[test]
fn test_multi_bucket_gas_ranking_update() {
    let mut pool = setup_mempool_with_broadcast_buckets(vec![0, 101, 201]).0;
    let txns = add_txns_to_mempool(&mut pool, vec![
        TestTransaction::new(1, ReplayProtector::SequenceNumber(0), 1), // bucket 0
        TestTransaction::new(1, ReplayProtector::SequenceNumber(1), 100), // bucket 0
        TestTransaction::new(1, ReplayProtector::SequenceNumber(2), 101), // bucket 1
        TestTransaction::new(1, ReplayProtector::SequenceNumber(3), 200), // bucket 1
    ]);
    let sender_bucket = sender_bucket(
        &txns[0].sender(),
        MempoolConfig::default().num_sender_buckets,
    );

    // txn 2 and 3 are prioritized
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![0, 0, 0].into(),
        2,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![2, 3]);
    // read only bucket 2
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![10, 10, 0].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert!(view(timeline).is_empty());

    // resubmit with higher gas: move txn 2 to bucket 2
    add_txns_to_mempool(&mut pool, vec![TestTransaction::new(
        1,
        ReplayProtector::SequenceNumber(2),
        400,
    )]);

    // txn 2 is now prioritized
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![0, 0, 0].into(),
        1,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![2]);
    // then txn 3 is prioritized
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![0, 0, 0].into(),
        2,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![2, 3]);
    // read only bucket 2
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![10, 10, 0].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![2]);
    // read only bucket 1
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![10, 0, 10].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![3]);
}

#[test]
fn test_multi_bucket_removal() {
    let mut pool = setup_mempool_with_broadcast_buckets(vec![0, 101, 201]).0;
    let txns = add_txns_to_mempool(&mut pool, vec![
        TestTransaction::new(1, ReplayProtector::SequenceNumber(0), 1), // bucket 0
        TestTransaction::new(1, ReplayProtector::SequenceNumber(1), 100), // bucket 0
        TestTransaction::new(1, ReplayProtector::SequenceNumber(2), 300), // bucket 2
        TestTransaction::new(1, ReplayProtector::SequenceNumber(3), 200), // bucket 1
    ]);
    let sender_bucket = sender_bucket(
        &txns[0].sender(),
        MempoolConfig::default().num_sender_buckets,
    );

    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![0, 0, 0].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![0, 1, 2, 3]);

    pool.commit_transaction(
        &TestTransaction::get_address(1),
        ReplayProtector::SequenceNumber(0),
    );
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![0, 0, 0].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![1, 2, 3]);

    pool.commit_transaction(
        &TestTransaction::get_address(1),
        ReplayProtector::SequenceNumber(1),
    );
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![0, 0, 0].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![2, 3]);

    pool.commit_transaction(
        &TestTransaction::get_address(1),
        ReplayProtector::SequenceNumber(2),
    );
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![0, 0, 0].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(view(timeline), vec![3]);

    pool.commit_transaction(
        &TestTransaction::get_address(1),
        ReplayProtector::SequenceNumber(3),
    );
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![0, 0, 0].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert!(view(timeline).is_empty());
}

#[test]
fn test_capacity() {
    let mut config = NodeConfig::generate_random_config();
    config.mempool.capacity = 1;
    config.mempool.system_transaction_timeout_secs = 0;
    let mut pool = CoreMempool::new(&config);

    // Error on exceeding limit.
    add_txn(
        &mut pool,
        TestTransaction::new(1, ReplayProtector::SequenceNumber(0), 1),
    )
    .unwrap();
    assert!(add_txn(
        &mut pool,
        TestTransaction::new(1, ReplayProtector::SequenceNumber(1), 1)
    )
    .is_err());

    // Commit transaction and free space.
    pool.commit_transaction(
        &TestTransaction::get_address(1),
        ReplayProtector::SequenceNumber(0),
    );
    assert!(add_txn(
        &mut pool,
        TestTransaction::new(1, ReplayProtector::SequenceNumber(1), 1)
    )
    .is_ok());

    // Fill it up and check that GC routine will clear space.
    assert!(add_txn(
        &mut pool,
        TestTransaction::new(1, ReplayProtector::SequenceNumber(2), 1)
    )
    .is_err());
    pool.gc();
    assert!(add_txn(
        &mut pool,
        TestTransaction::new(1, ReplayProtector::SequenceNumber(2), 1)
    )
    .is_ok());
}

#[test]
fn test_capacity_bytes() {
    let capacity_bytes = 2_048;

    // Get transactions to add.
    let address = 1;
    let mut size_bytes: usize = 0;
    let mut seq_no = 1_000;
    let mut txns = vec![];
    let last_txn;
    loop {
        let txn = signed_txn_to_mempool_transaction(
            TestTransaction::new(address, ReplayProtector::SequenceNumber(seq_no), 1)
                .make_signed_transaction(),
        );
        let txn_bytes = txn.get_estimated_bytes();

        if size_bytes <= capacity_bytes {
            txns.push(txn);
            seq_no -= 1;
            size_bytes += txn_bytes;
        } else {
            last_txn = Some(txn);
            break;
        }
    }
    assert!(!txns.is_empty());
    assert!(last_txn.is_some());

    // Set exact limit
    let capacity_bytes = size_bytes;

    let mut config = NodeConfig::generate_random_config();
    config.mempool.capacity = 1_000; // Won't hit this limit.
    config.mempool.capacity_bytes = capacity_bytes;
    config.mempool.system_transaction_timeout_secs = 0;
    let mut pool = CoreMempool::new(&config);

    for _i in 0..2 {
        txns.clone().into_iter().for_each(|txn| {
            let status = pool.add_txn(
                txn.txn,
                txn.ranking_score,
                Some(0),
                txn.timeline_state,
                false,
                None,
                Some(BroadcastPeerPriority::Primary),
            );
            assert_eq!(status.code, MempoolStatusCode::Accepted);
        });

        if let Some(txn) = last_txn.clone() {
            let status = pool.add_txn(
                txn.txn,
                txn.ranking_score,
                Some(0),
                txn.timeline_state,
                false,
                None,
                Some(BroadcastPeerPriority::Primary),
            );
            assert_eq!(status.code, MempoolStatusCode::MempoolIsFull);
        }
        // Check that GC returns size to zero.
        pool.gc();
    }
}

fn signed_txn_to_mempool_transaction(txn: SignedTransaction) -> MempoolTransaction {
    MempoolTransaction::new(
        txn,
        Duration::from_secs(1),
        1,
        TimelineState::NotReady,
        SystemTime::now(),
        false,
        Some(BroadcastPeerPriority::Primary),
    )
}

#[test]
fn test_parking_lot_eviction() {
    let mut config = NodeConfig::generate_random_config();
    config.mempool.capacity = 5;
    let mut pool = CoreMempool::new(&config);
    // Add transactions with the following sequence numbers to Mempool.
    for seq in &[0, 1, 2, 9, 10] {
        add_txn(
            &mut pool,
            TestTransaction::new(1, ReplayProtector::SequenceNumber(*seq), 1),
        )
        .unwrap();
    }
    // Mempool is full. Insert few txns for other account.
    for seq in &[0, 1] {
        add_txn(
            &mut pool,
            TestTransaction::new(0, ReplayProtector::SequenceNumber(*seq), 1),
        )
        .unwrap();
    }
    // Make sure that we have correct txns in Mempool.
    let mut txns: Vec<_> = pool
        .get_batch(5, 5120, true, btreemap![])
        .iter()
        .map(SignedTransaction::sequence_number)
        .collect();
    txns.sort_unstable();
    assert_eq!(txns, vec![0, 0, 1, 1, 2]);

    // Make sure we can't insert any new transactions, cause parking lot supposed to be empty by now.
    assert!(add_txn(
        &mut pool,
        TestTransaction::new(0, ReplayProtector::SequenceNumber(2), 1)
    )
    .is_err());
}

#[test]
fn test_parking_lot_eviction_bytes() {
    // Get the small transaction size
    let small_txn_size = signed_txn_to_mempool_transaction(
        TestTransaction::new(1, ReplayProtector::SequenceNumber(1), 1).make_signed_transaction(),
    )
    .get_estimated_bytes();

    let mut config = NodeConfig::generate_random_config();
    config.mempool.capacity = 100;
    // Fit 2 small transactions + one additional transaction (by overflowing the capacity bytes)
    config.mempool.capacity_bytes = 3 * small_txn_size + 1;
    let mut pool = CoreMempool::new(&config);
    // Add 2 small transactions to parking lot
    for address in 0..2 {
        add_txn(
            &mut pool,
            TestTransaction::new(address, ReplayProtector::SequenceNumber(1), 1),
        )
        .unwrap();
    }
    // Add one large transaction that will top off the capacity bytes
    add_txn(
        &mut pool,
        TestTransaction::new_with_large_script(2, ReplayProtector::SequenceNumber(1), 1),
    )
    .unwrap();
    // Mempool is full. Insert a small txn for other account.
    add_txn(
        &mut pool,
        TestTransaction::new(3, ReplayProtector::SequenceNumber(0), 1),
    )
    .unwrap();
}

#[test]
fn test_parking_lot_eviction_benchmark() {
    // Get the small transaction size
    let small_txn_size = signed_txn_to_mempool_transaction(
        TestTransaction::new(1, ReplayProtector::SequenceNumber(1), 1).make_signed_transaction(),
    )
    .get_estimated_bytes();
    let huge_txn_size = signed_txn_to_mempool_transaction(
        TestTransaction::new_with_huge_script(1, ReplayProtector::SequenceNumber(1), 1)
            .make_signed_transaction(),
    )
    .get_estimated_bytes();
    let num_small_txns = (huge_txn_size / small_txn_size) * 2;

    let mut config = NodeConfig::generate_random_config();
    config.mempool.capacity_per_user = 200;
    config.mempool.capacity = 4_000_000;
    // ~5 MB
    config.mempool.capacity_bytes = num_small_txns * small_txn_size + 1;
    let mut pool = CoreMempool::new(&config);

    // // Add one huge transaction that will evict all transactions from parking lot
    // let huge_signed_txn = TestTransaction::new_with_huge_script(0, 1, 1).make_signed_transaction();
    // // Pre-compute these values, as shared mempool would do
    // huge_signed_txn.committed_hash();
    // huge_signed_txn.txn_bytes_len();
    //
    // let now = Instant::now();
    // add_signed_txn(&mut pool, huge_signed_txn.clone()).unwrap();
    // // Flush the huge transaction
    // add_txn(&mut pool, TestTransaction::new(1, 0, 1)).unwrap();

    let accounts: Vec<_> = (0..num_small_txns)
        .map(|_| AccountAddress::random())
        .collect();
    // Fill up parking lot to capacity
    for account in accounts {
        for seq_num in 1..2 {
            add_txn(
                &mut pool,
                TestTransaction::new_with_address(
                    account,
                    ReplayProtector::SequenceNumber(seq_num),
                    1,
                ),
            )
            .unwrap();
        }
    }
    // Add one huge transaction that will cause mempool to be (beyond) full
    let huge_signed_txn =
        TestTransaction::new_with_huge_script(0, ReplayProtector::SequenceNumber(0), 1)
            .make_signed_transaction();
    add_signed_txn(&mut pool, huge_signed_txn).unwrap();
    assert_eq!(pool.get_parking_lot_size(), num_small_txns);

    // Add one huge transaction that will evict many transactions from parking lot
    let huge_signed_txn =
        TestTransaction::new_with_huge_script(1, ReplayProtector::SequenceNumber(0), 1)
            .make_signed_transaction();
    // Pre-compute these values, as shared mempool would do
    huge_signed_txn.committed_hash();
    huge_signed_txn.txn_bytes_len();
    let now = Instant::now();
    add_signed_txn(&mut pool, huge_signed_txn).unwrap();
    let time_to_evict_ms = now.elapsed().as_millis();

    let has_remainder = huge_txn_size % small_txn_size != 0;
    let num_expected_evicted = num_small_txns / 2 + has_remainder as usize;
    assert_eq!(
        pool.get_parking_lot_size(),
        num_small_txns - num_expected_evicted
    );
    assert!(
        time_to_evict_ms < 300,
        "Parking lot eviction of {} should take less than 300 ms on a reasonable machine. Took {} ms",
        num_expected_evicted, time_to_evict_ms
    );
}

#[test]
fn test_parking_lot_evict_only_for_ready_txn_insertion() {
    let mut config = NodeConfig::generate_random_config();
    config.mempool.capacity = 6;
    let mut pool = CoreMempool::new(&config);
    // Add transactions with the following sequence numbers to Mempool.
    for seq in &[0, 1, 2, 9, 10, 11] {
        add_txn(
            &mut pool,
            TestTransaction::new(1, ReplayProtector::SequenceNumber(*seq), 1),
        )
        .unwrap();
    }

    // Try inserting for ready txs.
    let ready_seq_nums = vec![3, 4];
    for seq in ready_seq_nums {
        add_txn(
            &mut pool,
            TestTransaction::new(1, ReplayProtector::SequenceNumber(seq), 1),
        )
        .unwrap();
    }

    // Make sure that we have correct txns in Mempool.
    let mut txns: Vec<_> = pool
        .get_batch(5, 5120, true, btreemap![])
        .iter()
        .map(SignedTransaction::sequence_number)
        .collect();
    txns.sort_unstable();
    assert_eq!(txns, vec![0, 1, 2, 3, 4]);

    // Trying to insert a tx that would not be ready after inserting should fail.
    let not_ready_seq_nums = vec![6, 8, 12, 14];
    for seq in not_ready_seq_nums {
        assert!(add_txn(
            &mut pool,
            TestTransaction::new(1, ReplayProtector::SequenceNumber(seq), 1)
        )
        .is_err());
    }
}

#[test]
fn test_gc_ready_transaction() {
    let mut pool = setup_mempool().0;
    add_txn(
        &mut pool,
        TestTransaction::new(1, ReplayProtector::SequenceNumber(0), 1),
    )
    .unwrap();

    // Insert in the middle transaction that's going to be expired.
    let txn = TestTransaction::new(1, ReplayProtector::SequenceNumber(1), 1)
        .make_signed_transaction_with_expiration_time(0);
    let sender_bucket = sender_bucket(&txn.sender(), MempoolConfig::default().num_sender_buckets);

    pool.add_txn(
        txn,
        1,
        Some(0),
        TimelineState::NotReady,
        false,
        None,
        Some(BroadcastPeerPriority::Primary),
    );

    // Insert few transactions after it.
    // They are supposed to be ready because there's a sequential path from 0 to them.
    add_txn(
        &mut pool,
        TestTransaction::new(1, ReplayProtector::SequenceNumber(2), 1),
    )
    .unwrap();
    add_txn(
        &mut pool,
        TestTransaction::new(1, ReplayProtector::SequenceNumber(3), 1),
    )
    .unwrap();

    // Check that all txns are ready.
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![0].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(timeline.len(), 4);

    // GC expired transaction.
    pool.gc_by_expiration_time(Duration::from_secs(1));

    // Make sure txns 2 and 3 became not ready and we can't read them from any API.
    let block = pool.get_batch(1, 1024, true, btreemap![]);
    assert_eq!(block.len(), 1);
    assert_eq!(block[0].sequence_number(), 0);

    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![0].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(timeline.len(), 1);
    assert_eq!(timeline[0].0.sequence_number(), 0);

    // Resubmit txn 1
    add_txn(
        &mut pool,
        TestTransaction::new(1, ReplayProtector::SequenceNumber(1), 1),
    )
    .unwrap();

    // Make sure txns 2 and 3 can be broadcast after txn 1 is resubmitted
    let (timeline, _) = pool.read_timeline(
        sender_bucket,
        &vec![0].into(),
        10,
        None,
        BroadcastPeerPriority::Primary,
    );
    assert_eq!(timeline.len(), 4);
}

#[test]
fn test_clean_stuck_transactions() {
    let mut pool = setup_mempool().0;
    for seq in 0..5 {
        add_txn(
            &mut pool,
            TestTransaction::new(0, ReplayProtector::SequenceNumber(seq), 1),
        )
        .unwrap();
    }
    let db_sequence_number = 10;
    let txn = TestTransaction::new(0, ReplayProtector::SequenceNumber(db_sequence_number), 1)
        .make_signed_transaction();
    pool.add_txn(
        txn,
        1,
        Some(db_sequence_number),
        TimelineState::NotReady,
        false,
        None,
        Some(BroadcastPeerPriority::Primary),
    );
    let block = pool.get_batch(1, 1024, true, btreemap![]);
    assert_eq!(block.len(), 1);
    assert_eq!(block[0].sequence_number(), 10);
}

#[test]
fn test_get_transaction_by_hash() {
    let mut pool = setup_mempool().0;
    let db_sequence_number = 10;
    let txn = TestTransaction::new(0, ReplayProtector::SequenceNumber(db_sequence_number), 1)
        .make_signed_transaction();
    pool.add_txn(
        txn.clone(),
        1,
        Some(db_sequence_number),
        TimelineState::NotReady,
        false,
        None,
        Some(BroadcastPeerPriority::Primary),
    );
    let hash = txn.committed_hash();
    let ret = pool.get_by_hash(hash);
    assert_eq!(ret, Some(txn));

    let ret = pool.get_by_hash(HashValue::random());
    assert!(ret.is_none());
}

#[test]
fn test_get_transaction_by_hash_after_the_txn_is_updated() {
    let mut pool = setup_mempool().0;
    let db_sequence_number = 10;

    let txn = TestTransaction::new(0, ReplayProtector::SequenceNumber(db_sequence_number), 1)
        .make_signed_transaction();
    pool.add_txn(
        txn.clone(),
        1,
        Some(db_sequence_number),
        TimelineState::NotReady,
        false,
        None,
        Some(BroadcastPeerPriority::Primary),
    );
    let hash = txn.committed_hash();

    // new txn with higher gas price
    let new_txn = TestTransaction::new(0, ReplayProtector::SequenceNumber(db_sequence_number), 100)
        .make_signed_transaction();
    pool.add_txn(
        new_txn.clone(),
        1,
        Some(db_sequence_number),
        TimelineState::NotReady,
        false,
        None,
        Some(BroadcastPeerPriority::Primary),
    );
    let new_txn_hash = new_txn.committed_hash();

    let txn_by_old_hash = pool.get_by_hash(hash);
    assert!(txn_by_old_hash.is_none());

    let txn_by_new_hash = pool.get_by_hash(new_txn_hash);
    assert_eq!(txn_by_new_hash, Some(new_txn));
}

#[test]
fn test_bytes_limit() {
    let mut config = NodeConfig::generate_random_config();
    config.mempool.capacity = 100;
    let mut pool = CoreMempool::new(&config);
    // add 100 transacionts
    for seq in 0..100 {
        add_txn(
            &mut pool,
            TestTransaction::new(1, ReplayProtector::SequenceNumber(seq), 1),
        )
        .unwrap();
    }
    let get_all = pool.get_batch(100, 100 * 1024, true, btreemap![]);
    assert_eq!(get_all.len(), 100);
    let txn_size = get_all[0].txn_bytes_len() as u64;
    let limit = 10;
    let hit_limit = pool.get_batch(100, txn_size * limit, true, btreemap![]);
    assert_eq!(hit_limit.len(), limit as usize);
    let hit_limit = pool.get_batch(100, txn_size * limit + 1, true, btreemap![]);
    assert_eq!(hit_limit.len(), limit as usize);
    let hit_limit = pool.get_batch(100, txn_size * limit - 1, true, btreemap![]);
    assert_eq!(hit_limit.len(), limit as usize - 1);
}

#[test]
fn test_transaction_store_remove_account_if_empty() {
    let mut config = NodeConfig::generate_random_config();
    config.mempool.capacity = 100;
    let mut pool = CoreMempool::new(&config);

    assert_eq!(pool.get_transaction_store().get_transactions().len(), 0);

    add_txn(
        &mut pool,
        TestTransaction::new(1, ReplayProtector::SequenceNumber(0), 1),
    )
    .unwrap();
    add_txn(
        &mut pool,
        TestTransaction::new(1, ReplayProtector::SequenceNumber(1), 1),
    )
    .unwrap();
    add_txn(
        &mut pool,
        TestTransaction::new(2, ReplayProtector::SequenceNumber(0), 1),
    )
    .unwrap();
    assert_eq!(pool.get_transaction_store().get_transactions().len(), 2);

    pool.commit_transaction(
        &TestTransaction::get_address(1),
        ReplayProtector::SequenceNumber(0),
    );
    pool.commit_transaction(
        &TestTransaction::get_address(1),
        ReplayProtector::SequenceNumber(1),
    );
    pool.commit_transaction(
        &TestTransaction::get_address(2),
        ReplayProtector::SequenceNumber(0),
    );
    assert_eq!(pool.get_transaction_store().get_transactions().len(), 0);

    let txn =
        TestTransaction::new(2, ReplayProtector::SequenceNumber(2), 1).make_signed_transaction();
    let hash = txn.committed_hash();
    add_signed_txn(&mut pool, txn).unwrap();
    assert_eq!(pool.get_transaction_store().get_transactions().len(), 1);

    pool.reject_transaction(
        &TestTransaction::get_address(2),
        ReplayProtector::SequenceNumber(2),
        &hash,
        &DiscardedVMStatus::MALFORMED,
    );
    assert_eq!(pool.get_transaction_store().get_transactions().len(), 0);
}

#[test]
fn test_sequence_number_behavior_at_capacity() {
    let mut config = NodeConfig::generate_random_config();
    config.mempool.capacity = 2;
    let mut pool = CoreMempool::new(&config);

    add_txn(
        &mut pool,
        TestTransaction::new(0, ReplayProtector::SequenceNumber(0), 1),
    )
    .unwrap();
    add_txn(
        &mut pool,
        TestTransaction::new(1, ReplayProtector::SequenceNumber(0), 1),
    )
    .unwrap();
    pool.commit_transaction(
        &TestTransaction::get_address(1),
        ReplayProtector::SequenceNumber(0),
    );
    add_txn(
        &mut pool,
        TestTransaction::new(2, ReplayProtector::SequenceNumber(0), 1),
    )
    .unwrap();
    pool.commit_transaction(
        &TestTransaction::get_address(2),
        ReplayProtector::SequenceNumber(0),
    );

    let batch = pool.get_batch(10, 10240, true, btreemap![]);
    assert_eq!(batch.len(), 1);
}

#[test]
fn test_sequence_number_stale_account_sequence_number() {
    let mut config = NodeConfig::generate_random_config();
    config.mempool.capacity = 2;
    let mut pool = CoreMempool::new(&config);
    pool.commit_transaction(
        &TestTransaction::get_address(0),
        ReplayProtector::SequenceNumber(1),
    );
    // This has a stale account sequence number of 0
    add_txn(
        &mut pool,
        TestTransaction::new(0, ReplayProtector::SequenceNumber(2), 1),
    )
    .unwrap();

    let batch = pool.get_batch(10, 10240, true, btreemap![]);
    assert_eq!(batch.len(), 1);
}

#[test]
fn test_not_return_non_full() {
    let mut config = NodeConfig::generate_random_config();
    config.mempool.capacity = 2;
    let mut pool = CoreMempool::new(&config);
    let txn_0 = TestTransaction::new(0, ReplayProtector::SequenceNumber(0), 1);
    let txn_1 = TestTransaction::new(0, ReplayProtector::SequenceNumber(1), 1);
    let txn_num = 2;
    let txn_bytes = txn_bytes_len(txn_0.clone()) + txn_bytes_len(txn_1.clone());
    add_txn(&mut pool, txn_0).unwrap();
    add_txn(&mut pool, txn_1).unwrap();

    // doesn't hit any limits
    let batch = pool.get_batch(10, 10240, true, btreemap![]);
    assert_eq!(batch.len(), 2);

    let batch = pool.get_batch(10, 10240, false, btreemap![]);
    assert_eq!(batch.len(), 0);

    // reaches or close to max_txns
    let batch = pool.get_batch(txn_num + 1, 10240, false, btreemap![]);
    assert_eq!(batch.len(), 0);

    let batch = pool.get_batch(txn_num, 10240, false, btreemap![]);
    assert_eq!(batch.len(), 2);

    let batch = pool.get_batch(txn_num - 1, 10240, false, btreemap![]);
    assert_eq!(batch.len(), 1);

    let batch = pool.get_batch(txn_num + 1, 10240, true, btreemap![]);
    assert_eq!(batch.len(), 2);

    let batch = pool.get_batch(txn_num, 10240, true, btreemap![]);
    assert_eq!(batch.len(), 2);

    let batch = pool.get_batch(txn_num - 1, 10240, true, btreemap![]);
    assert_eq!(batch.len(), 1);

    // reaches or close to max_bytes
    let batch = pool.get_batch(10, txn_bytes + 1, false, btreemap![]);
    assert_eq!(batch.len(), 0);

    let batch = pool.get_batch(10, txn_bytes, false, btreemap![]);
    assert_eq!(batch.len(), 2);

    let batch = pool.get_batch(10, txn_bytes - 1, false, btreemap![]);
    assert_eq!(batch.len(), 1);

    let batch = pool.get_batch(10, txn_bytes + 1, true, btreemap![]);
    assert_eq!(batch.len(), 2);

    let batch = pool.get_batch(10, txn_bytes, true, btreemap![]);
    assert_eq!(batch.len(), 2);

    let batch = pool.get_batch(10, txn_bytes - 1, true, btreemap![]);
    assert_eq!(batch.len(), 1);
}

#[test]
fn test_include_gas_upgraded() {
    let mut config = NodeConfig::generate_random_config();
    config.mempool.capacity = 100;
    let mut pool = CoreMempool::new(&config);

    let sequence_number = 0;
    let address_index = 0;

    let low_gas_price = 1;
    let low_gas_signed_txn = add_txn(
        &mut pool,
        TestTransaction::new(
            address_index,
            ReplayProtector::SequenceNumber(sequence_number),
            low_gas_price,
        ),
    )
    .unwrap();

    let low_gas_txn = TransactionSummary::new(
        low_gas_signed_txn.sender(),
        ReplayProtector::SequenceNumber(low_gas_signed_txn.sequence_number()),
        low_gas_signed_txn.committed_hash(),
    );
    let batch = pool.get_batch(10, 10240, true, btreemap! {
        low_gas_txn => TransactionInProgress::new(low_gas_price)
    });
    assert_eq!(batch.len(), 0);

    let high_gas_price = 100;
    let high_gas_signed_txn = add_txn(
        &mut pool,
        TestTransaction::new(
            address_index,
            ReplayProtector::SequenceNumber(sequence_number),
            high_gas_price,
        ),
    )
    .unwrap();
    let high_gas_txn = TransactionSummary::new(
        high_gas_signed_txn.sender(),
        ReplayProtector::SequenceNumber(high_gas_signed_txn.sequence_number()),
        high_gas_signed_txn.committed_hash(),
    );

    // When the low gas txn (but not the high gas txn) is excluded, will the high gas txn be included.
    let batch = pool.get_batch(10, 10240, true, btreemap! {
        low_gas_txn => TransactionInProgress::new(low_gas_price)
    });
    assert_eq!(batch.len(), 1);
    assert_eq!(
        batch[0].sender(),
        TestTransaction::get_address(address_index)
    );
    assert_eq!(batch[0].sequence_number(), sequence_number);
    assert_eq!(batch[0].gas_unit_price(), high_gas_price);

    let batch = pool.get_batch(10, 10240, true, btreemap! {
        high_gas_txn => TransactionInProgress::new(high_gas_price)
    });
    assert_eq!(batch.len(), 0);

    let batch = pool.get_batch(10, 10240, true, btreemap! {
        low_gas_txn => TransactionInProgress::new(low_gas_price),
        high_gas_txn => TransactionInProgress::new(high_gas_price)
    });
    assert_eq!(batch.len(), 0);
}
