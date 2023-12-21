// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::utils::ProofQueue;
use aptos_consensus_types::proof_of_store::{BatchId, BatchInfo, ProofOfStore};
use aptos_crypto::HashValue;
use aptos_types::{aggregate_signature::AggregateSignature, PeerId};
use maplit::hashset;

/// Return a ProofOfStore with minimal fields used by ProofQueue tests.
fn proof_of_store(author: PeerId, batch_id: BatchId, gas_bucket_start: u64) -> ProofOfStore {
    ProofOfStore::new(
        BatchInfo::new(
            author,
            batch_id,
            0,
            0,
            HashValue::random(),
            1,
            1,
            gas_bucket_start,
        ),
        AggregateSignature::empty(),
    )
}

#[test]
fn test_proof_queue_sorting() {
    let my_peer_id = PeerId::random();
    let mut proof_queue = ProofQueue::new(my_peer_id);

    let author_0 = PeerId::random();
    let author_1 = PeerId::random();

    let author_0_batches = vec![
        proof_of_store(author_0, BatchId::new_for_test(0), 100),
        proof_of_store(author_0, BatchId::new_for_test(1), 200),
        proof_of_store(author_0, BatchId::new_for_test(2), 50),
        proof_of_store(author_0, BatchId::new_for_test(3), 300),
    ];
    for batch in author_0_batches {
        proof_queue.push(batch);
    }
    let author_1_batches = vec![
        proof_of_store(author_1, BatchId::new_for_test(4), 500),
        proof_of_store(author_1, BatchId::new_for_test(5), 400),
        proof_of_store(author_1, BatchId::new_for_test(6), 600),
        proof_of_store(author_1, BatchId::new_for_test(7), 50),
    ];
    for batch in author_1_batches {
        proof_queue.push(batch);
    }

    // Expect: [600, 300]
    let pulled = proof_queue.pull_proofs(&hashset![], 2, 2, true);
    let mut count_author_0 = 0;
    let mut count_author_1 = 0;
    let mut prev: Option<&ProofOfStore> = None;
    for batch in &pulled {
        if let Some(prev) = prev {
            assert!(prev.gas_bucket_start() >= batch.gas_bucket_start());
        } else {
            assert_eq!(batch.gas_bucket_start(), 600);
        }
        if batch.author() == author_0 {
            count_author_0 += 1;
        } else {
            count_author_1 += 1;
        }
        prev = Some(batch);
    }
    assert_eq!(count_author_0, 1);
    assert_eq!(count_author_1, 1);

    // Expect: [600, 500, 300, 100]
    let pulled = proof_queue.pull_proofs(&hashset![], 4, 4, true);
    let mut count_author_0 = 0;
    let mut count_author_1 = 0;
    let mut prev: Option<&ProofOfStore> = None;
    for batch in &pulled {
        if let Some(prev) = prev {
            assert!(prev.gas_bucket_start() >= batch.gas_bucket_start());
        } else {
            assert_eq!(batch.gas_bucket_start(), 600);
        }
        if batch.author() == author_0 {
            count_author_0 += 1;
        } else {
            count_author_1 += 1;
        }
        prev = Some(batch);
    }
    assert_eq!(count_author_0, 2);
    assert_eq!(count_author_1, 2);
}
