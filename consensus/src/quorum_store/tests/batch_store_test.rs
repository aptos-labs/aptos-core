// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    quorum_store::{
        batch_requester::BatchRequester, batch_store::BatchStore, quorum_store_db::QuorumStoreDB,
        types::PersistedValue,
    },
    test_utils::mock_quorum_store_sender::MockQuorumStoreSender,
};
use aptos_consensus_types::proof_of_store::{BatchId, LogicalTime};
use aptos_crypto::HashValue;
use aptos_temppath::TempPath;
use aptos_types::{account_address::AccountAddress, validator_verifier::random_validator_verifier};
use claims::assert_ok_eq;
use futures::executor::block_on;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::{sync::mpsc::channel, task::spawn_blocking};

fn batch_store_for_test_no_db(memory_quota: usize) -> Arc<BatchStore<MockQuorumStoreSender>> {
    let tmp_dir = TempPath::new();
    let db = Arc::new(QuorumStoreDB::new(&tmp_dir));
    let (tx, _rx) = channel(10);
    let requester = BatchRequester::new(
        10,
        AccountAddress::random(),
        1,
        1,
        MockQuorumStoreSender::new(tx),
    );
    let (signers, validator_verifier) = random_validator_verifier(4, None, false);

    Arc::new(BatchStore::new(
        10, // epoch
        10, // last committed round
        db,
        0,
        0,
        2100,
        0,            // grace period rounds
        memory_quota, // memory_quota
        1000,         // db quota
        requester,
        signers[0].clone(),
        validator_verifier,
    ))
}

#[test]
fn test_insert_expire() {
    let batch_store = batch_store_for_test_no_db(30);

    let digest = HashValue::random();
    assert_ok_eq!(
        batch_store.insert_to_cache(
            digest,
            PersistedValue::new(
                Some(Vec::new()),
                LogicalTime::new(10, 15), // Expiration
                AccountAddress::random(),
                BatchId::new_for_test(1),
                10,
            ),
        ),
        true
    );

    assert_ok_eq!(
        batch_store.insert_to_cache(
            digest,
            PersistedValue::new(
                Some(Vec::new()),
                LogicalTime::new(10, 30), // Expiration
                AccountAddress::random(),
                BatchId::new_for_test(1),
                10,
            ),
        ),
        true
    );
    assert_ok_eq!(
        batch_store.insert_to_cache(
            digest,
            PersistedValue::new(
                Some(Vec::new()),
                LogicalTime::new(10, 25), // Expiration
                AccountAddress::random(),
                BatchId::new_for_test(1),
                10,
            ),
        ),
        false
    );
    let expired = batch_store.clear_expired_payload(LogicalTime::new(10, 27));
    assert!(expired.is_empty());
    let expired = batch_store.clear_expired_payload(LogicalTime::new(10, 29));
    assert!(expired.is_empty());
    assert_eq!(
        batch_store.clear_expired_payload(LogicalTime::new(10, 30)),
        vec![digest]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_extend_expiration_vs_save() {
    let num_experiments = 2000;
    let batch_store = batch_store_for_test_no_db(0);

    let batch_store_clone1 = batch_store.clone();
    let batch_store_clone2 = batch_store.clone();

    let digests: Vec<HashValue> = (0..num_experiments).map(|_| HashValue::random()).collect();
    let later_exp_values: Vec<(HashValue, PersistedValue)> = (0..num_experiments)
        .map(|i| {
            // Pre-insert some of them.
            if i % 2 == 0 {
                batch_store
                    .save(
                        digests[i],
                        PersistedValue::new(
                            Some(Vec::new()),
                            LogicalTime::new(10, i as u64 + 30),
                            AccountAddress::random(),
                            BatchId::new_for_test(1),
                            10,
                        ),
                    )
                    .unwrap();
            }

            (
                digests[i],
                PersistedValue::new(
                    Some(Vec::new()),
                    LogicalTime::new(10, i as u64 + 40),
                    AccountAddress::random(),
                    BatchId::new_for_test(1),
                    10,
                ),
            )
        })
        .collect();

    // Marshal threads to start at the same time.
    let start_flag = Arc::new(AtomicUsize::new(0));
    let start_clone1 = start_flag.clone();
    let start_clone2 = start_flag.clone();

    // Thread that extends expiration by saving.
    spawn_blocking(move || {
        for (i, (digest, later_exp_value)) in later_exp_values.into_iter().enumerate() {
            // Wait until both threads are ready for next experiment.
            loop {
                let flag_val = start_clone1.load(Ordering::Acquire);
                if flag_val == 3 * i + 1 || flag_val == 3 * i + 2 {
                    break;
                }
            }

            batch_store_clone1.save(digest, later_exp_value).unwrap();
            start_clone1.fetch_add(1, Ordering::Relaxed);
        }
    });

    // Thread that expires.
    spawn_blocking(move || {
        for i in 0..num_experiments {
            // Wait until both threads are ready for next experiment.
            loop {
                let flag_val = start_clone2.load(Ordering::Acquire);
                if flag_val == 3 * i + 1 || flag_val == 3 * i + 2 {
                    break;
                }
            }

            block_on(
                batch_store_clone2.update_certified_round(LogicalTime::new(10, i as u64 + 30)),
            );
            start_clone2.fetch_add(1, Ordering::Relaxed);
        }
    });

    for (i, &digest) in digests.iter().enumerate().take(num_experiments) {
        // Set the conditions for experiment (both threads waiting).
        while start_flag.load(Ordering::Acquire) % 3 != 0 {}

        if i % 2 == 1 {
            batch_store
                .save(
                    digest,
                    PersistedValue::new(
                        Some(Vec::new()),
                        LogicalTime::new(10, i as u64 + 30),
                        AccountAddress::random(),
                        BatchId::new_for_test(1),
                        10,
                    ),
                )
                .unwrap();
        }

        // Unleash the threads.
        start_flag.fetch_add(1, Ordering::Relaxed);
    }
    // Finish the experiment
    while start_flag.load(Ordering::Acquire) % 3 != 0 {}

    // Expire everything, call for higher times as well.
    for i in 35..50 {
        batch_store
            .update_certified_round(LogicalTime::new(10, (i + num_experiments) as u64))
            .await;
    }
}

// TODO: last certified round.
// TODO: check correct digests are returned.
// TODO: check grace period.
// TODO: check quota (cache vs persisted).
// TODO: check the channels.
