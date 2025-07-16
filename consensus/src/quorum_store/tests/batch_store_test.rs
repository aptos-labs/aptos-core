// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::{
    batch_store::{BatchStore, BatchWriter, QuotaManager},
    quorum_store_db::QuorumStoreDB,
    types::{PersistedValue, StorageMode},
};
use aptos_consensus_types::proof_of_store::BatchInfo;
use aptos_crypto::HashValue;
use aptos_temppath::TempPath;
use aptos_types::{
    account_address::AccountAddress, quorum_store::BatchId, transaction::SignedTransaction,
    validator_verifier::random_validator_verifier,
};
use claims::{assert_err, assert_ok, assert_ok_eq};
use once_cell::sync::Lazy;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
use tokio::task::spawn_blocking;

static TEST_REQUEST_ACCOUNT: Lazy<AccountAddress> = Lazy::new(AccountAddress::random);

pub fn batch_store_for_test(memory_quota: usize) -> Arc<BatchStore> {
    let tmp_dir = TempPath::new();
    let db = Arc::new(QuorumStoreDB::new(&tmp_dir));
    let (signers, _validator_verifier) = random_validator_verifier(4, None, false);

    Arc::new(BatchStore::new(
        10, // epoch
        false,
        10, // last committed round
        db,
        memory_quota, // memory_quota
        2001,         // db quota
        2001,         // batch quota
        signers[0].clone(),
        0,
    ))
}

fn request_for_test(
    digest: &HashValue,
    round: u64,
    num_bytes: u64,
    maybe_payload: Option<Vec<SignedTransaction>>,
) -> PersistedValue {
    PersistedValue::new(
        BatchInfo::new(
            *TEST_REQUEST_ACCOUNT, // make sure all request come from the same account
            BatchId::new_for_test(1),
            10,
            round,
            *digest,
            10,
            num_bytes,
            0,
        ),
        maybe_payload,
    )
}

#[tokio::test]
async fn test_insert_expire() {
    let batch_store = batch_store_for_test(30);

    let digest = HashValue::random();

    assert_ok_eq!(
        batch_store.insert_to_cache(&request_for_test(&digest, 15, 10, None)),
        true
    );
    assert_ok_eq!(
        batch_store.insert_to_cache(&request_for_test(&digest, 30, 10, None)),
        true
    );
    assert_ok_eq!(
        batch_store.insert_to_cache(&request_for_test(&digest, 25, 10, None)),
        false
    );
    let expired = batch_store.clear_expired_payload(27);
    assert!(expired.is_empty());
    let expired = batch_store.clear_expired_payload(29);
    assert!(expired.is_empty());
    assert_eq!(batch_store.clear_expired_payload(30), vec![digest]);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_extend_expiration_vs_save() {
    let num_experiments = 2000;
    let batch_store = batch_store_for_test(2001);

    let batch_store_clone1 = batch_store.clone();
    let batch_store_clone2 = batch_store.clone();

    let digests: Vec<HashValue> = (0..num_experiments).map(|_| HashValue::random()).collect();
    let later_exp_values: Vec<PersistedValue> = (0..num_experiments)
        .map(|i| {
            // Pre-insert some of them.
            if i % 2 == 0 {
                assert_ok!(batch_store.save(&request_for_test(
                    &digests[i],
                    i as u64 + 30,
                    1,
                    None
                )));
            }

            request_for_test(&digests[i], i as u64 + 40, 1, None)
        })
        .collect();

    // Marshal threads to start at the same time.
    let start_flag = Arc::new(AtomicUsize::new(0));
    let start_clone1 = start_flag.clone();
    let start_clone2 = start_flag.clone();

    let save_error = Arc::new(AtomicBool::new(false));
    let save_error_clone1 = save_error.clone();
    let save_error_clone2 = save_error.clone();

    // Thread that extends expiration by saving.
    spawn_blocking(move || {
        for (i, later_exp_value) in later_exp_values.into_iter().enumerate() {
            // Wait until both threads are ready for next experiment.
            loop {
                let flag_val = start_clone1.load(Ordering::Acquire);
                if flag_val == 3 * i + 1 || flag_val == 3 * i + 2 {
                    break;
                }
            }

            if batch_store_clone1.save(&later_exp_value).is_err() {
                // Save in a separate flag and break so test doesn't hang.
                save_error_clone1.store(true, Ordering::Release);
                break;
            }
            start_clone1.fetch_add(1, Ordering::Relaxed);
        }
    });

    // Thread that expires.
    spawn_blocking(move || {
        for i in 0..num_experiments {
            // Wait until both threads are ready for next experiment.
            loop {
                let flag_val = start_clone2.load(Ordering::Acquire);
                if flag_val == 3 * i + 1
                    || flag_val == 3 * i + 2
                    || save_error_clone2.load(Ordering::Acquire)
                {
                    break;
                }
            }

            batch_store_clone2.update_certified_timestamp(i as u64 + 30);
            start_clone2.fetch_add(1, Ordering::Relaxed);
        }
    });

    for (i, &digest) in digests.iter().enumerate().take(num_experiments) {
        // Set the conditions for experiment (both threads waiting).
        while start_flag.load(Ordering::Acquire) % 3 != 0 {
            assert!(!save_error.load(Ordering::Acquire));
        }

        if i % 2 == 1 {
            assert_ok!(batch_store.save(&request_for_test(&digest, i as u64 + 30, 1, None)));
        }

        // Unleash the threads.
        start_flag.fetch_add(1, Ordering::Relaxed);
    }
    // Finish the experiment
    while start_flag.load(Ordering::Acquire) % 3 != 0 {}

    // Expire everything, call for higher times as well.
    for i in 35..50 {
        batch_store.update_certified_timestamp((i + num_experiments) as u64);
    }
}

#[test]
fn test_quota_manager() {
    let mut qm = QuotaManager::new(20, 10, 7);
    assert_ok_eq!(qm.update_quota(5), StorageMode::MemoryAndPersisted);
    assert_ok_eq!(qm.update_quota(3), StorageMode::MemoryAndPersisted);
    assert_ok_eq!(qm.update_quota(2), StorageMode::MemoryAndPersisted);
    assert_ok_eq!(qm.update_quota(1), StorageMode::PersistedOnly);
    assert_ok_eq!(qm.update_quota(2), StorageMode::PersistedOnly);
    assert_ok_eq!(qm.update_quota(7), StorageMode::PersistedOnly);
    // 6 batches, fully used quotas

    // exceed storage quota.
    assert_err!(qm.update_quota(2));

    qm.free_quota(5, StorageMode::MemoryAndPersisted);
    // 5 batches, available memory and db quota: 5

    // exceed storage quota
    assert_err!(qm.update_quota(6));
    assert_ok_eq!(qm.update_quota(3), StorageMode::MemoryAndPersisted);

    // exceed storage quota
    assert_err!(qm.update_quota(3));
    assert_ok_eq!(qm.update_quota(1), StorageMode::MemoryAndPersisted);
    // 7 batches, available memory and DB quota: 1

    // Exceed batch quota
    assert_err!(qm.update_quota(1));

    qm.free_quota(1, StorageMode::PersistedOnly);
    // 6 batches, available memory quota: 1, available DB quota: 2

    // exceed storage quota
    assert_err!(qm.update_quota(3));
    assert_ok_eq!(qm.update_quota(2), StorageMode::PersistedOnly);
    // 7 batches, available memory quota: 1, available DB quota: 0

    qm.free_quota(2, StorageMode::MemoryAndPersisted);
    // 6 batches, available memory quota: 3, available DB quota: 2

    // while there is available memory quota, DB quota isn't enough.
    assert_err!(qm.update_quota(3));
    assert_ok_eq!(qm.update_quota(2), StorageMode::MemoryAndPersisted);
}

#[tokio::test]
async fn test_get_local_batch() {
    let store = batch_store_for_test(30);

    let digest_1 = HashValue::random();
    let request_1 = request_for_test(&digest_1, 50, 20, Some(vec![]));
    // Should be stored in memory and DB.
    assert!(!store.persist(vec![request_1]).is_empty());

    store.update_certified_timestamp(40);

    let digest_2 = HashValue::random();
    assert!(digest_2 != digest_1);
    // Expiration is before 40.
    let request_2_expired = request_for_test(&digest_2, 30, 20, Some(vec![]));
    assert!(store.persist(vec![request_2_expired]).is_empty());
    // Proper (in the future) expiration.
    let request_2 = request_for_test(&digest_2, 55, 20, Some(vec![]));
    // Should be stored in DB only
    assert!(!store.persist(vec![request_2]).is_empty());

    let digest_3 = HashValue::random();
    assert!(digest_3 != digest_1);
    assert!(digest_3 != digest_2);
    let request_3 = request_for_test(&digest_3, 56, 1970, Some(vec![]));
    // Out of quota - should not be stored
    assert!(store.persist(vec![request_3.clone()]).is_empty());

    assert_ok!(store.get_batch_from_local(&digest_1));
    assert_ok!(store.get_batch_from_local(&digest_2));
    store.update_certified_timestamp(51);
    // Expired value w. digest_1.
    assert_err!(store.get_batch_from_local(&digest_1));
    assert_ok!(store.get_batch_from_local(&digest_2));

    // Value w. digest_3 was never persisted
    assert_err!(store.get_batch_from_local(&digest_3));
    // Since payload is cleared, we can now persist value w. digest_3
    assert!(!store.persist(vec![request_3]).is_empty());
    assert_ok!(store.get_batch_from_local(&digest_3));

    store.update_certified_timestamp(52);
    assert_ok!(store.get_batch_from_local(&digest_2));
    assert_ok!(store.get_batch_from_local(&digest_3));

    store.update_certified_timestamp(55);
    // Expired value w. digest_2
    assert_err!(store.get_batch_from_local(&digest_2));
    assert_ok!(store.get_batch_from_local(&digest_3));

    store.update_certified_timestamp(56);
    // Expired value w. digest_3
    assert_err!(store.get_batch_from_local(&digest_1));
    assert_err!(store.get_batch_from_local(&digest_2));
    assert_err!(store.get_batch_from_local(&digest_3));
}
