// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::utils::ProofQueue;
use aptos_consensus_types::{
    common::TxnSummaryWithExpiration,
    proof_of_store::{BatchId, BatchInfo, ProofOfStore},
};
use aptos_crypto::HashValue;
use aptos_types::{aggregate_signature::AggregateSignature, PeerId};
use maplit::hashset;
use std::collections::HashSet;

/// Return a ProofOfStore with minimal fields used by ProofQueue tests.
fn proof_of_store(
    author: PeerId,
    batch_id: BatchId,
    gas_bucket_start: u64,
    expiration: u64,
) -> ProofOfStore {
    ProofOfStore::new(
        BatchInfo::new(
            author,
            batch_id,
            0,
            expiration,
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
        proof_of_store(author_0, BatchId::new_for_test(0), 100, 1),
        proof_of_store(author_0, BatchId::new_for_test(1), 200, 1),
        proof_of_store(author_0, BatchId::new_for_test(2), 50, 1),
        proof_of_store(author_0, BatchId::new_for_test(3), 300, 1),
    ];
    for batch in author_0_batches {
        proof_queue.push(batch);
    }
    let author_1_batches = vec![
        proof_of_store(author_1, BatchId::new_for_test(4), 500, 1),
        proof_of_store(author_1, BatchId::new_for_test(5), 400, 1),
        proof_of_store(author_1, BatchId::new_for_test(6), 600, 1),
        proof_of_store(author_1, BatchId::new_for_test(7), 50, 1),
    ];
    for batch in author_1_batches {
        proof_queue.push(batch);
    }

    // Expect: [600, 300]
    let (pulled, num_unique_txns, _) = proof_queue.pull_proofs(&hashset![], 4, 2, 2, true);
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
    assert_eq!(num_unique_txns, 2);

    // Expect: [600, 500, 300, 100]
    let (pulled, num_unique_txns, _) = proof_queue.pull_proofs(&hashset![], 6, 4, 4, true);
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
    assert_eq!(num_unique_txns, 4);
    assert_eq!(count_author_0, 2);
    assert_eq!(count_author_1, 2);
}

#[test]
fn test_proof_calculate_remaining_txns_and_proofs() {
    let my_peer_id = PeerId::random();
    let mut proof_queue = ProofQueue::new(my_peer_id);
    let now = aptos_infallible::duration_since_epoch().as_micros() as u64;
    let now_in_secs = aptos_infallible::duration_since_epoch().as_secs() as u64;
    let author_0 = PeerId::random();
    let author_1 = PeerId::random();
    let txns = vec![
        TxnSummaryWithExpiration::new(PeerId::ONE, 0, now_in_secs + 1, HashValue::zero()),
        TxnSummaryWithExpiration::new(PeerId::ONE, 1, now_in_secs + 1, HashValue::zero()),
        TxnSummaryWithExpiration::new(PeerId::ONE, 2, now_in_secs + 1, HashValue::zero()),
        TxnSummaryWithExpiration::new(PeerId::ONE, 3, now_in_secs + 1, HashValue::zero()),
    ];

    let author_0_batches = vec![
        proof_of_store(author_0, BatchId::new_for_test(0), 100, now + 50000),
        proof_of_store(author_0, BatchId::new_for_test(1), 200, now + 70000),
        proof_of_store(author_0, BatchId::new_for_test(2), 50, now + 20000),
        proof_of_store(author_0, BatchId::new_for_test(3), 300, now + 10000),
    ];

    let author_1_batches = vec![
        proof_of_store(author_1, BatchId::new_for_test(4), 500, now + 20000),
        proof_of_store(author_1, BatchId::new_for_test(5), 400, now + 30000),
        proof_of_store(author_1, BatchId::new_for_test(6), 600, now + 50000),
        proof_of_store(author_1, BatchId::new_for_test(7), 50, now + 60000),
    ];

    let info_1 = author_0_batches[0].info().clone();
    let info_2 = author_0_batches[1].info().clone();
    let info_3 = author_0_batches[2].info().clone();
    let info_4 = author_0_batches[3].info().clone();
    let info_5 = author_1_batches[0].info().clone();
    let info_6 = author_1_batches[1].info().clone();
    let info_7 = author_1_batches[2].info().clone();
    let info_8 = author_1_batches[3].info().clone();

    proof_queue.add_batch_summaries(vec![(info_1.clone(), vec![txns[0]])]);
    // batch_summaries: [1 -> txn_0]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (0, 0));

    proof_queue.push(author_0_batches[0].clone());
    // txns: [txn_0]
    // proofs: [1]
    // batch_summaries: [1 -> txn_0]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (1, 1));

    proof_queue.push(author_0_batches[1].clone());
    // txns: [txn_0] + txns(proof_2)
    // proofs: [1, 2]
    // batch_summaries: [1 -> txn_0]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (2, 2));

    proof_queue.add_batch_summaries(vec![(info_2, vec![txns[1]])]);
    // txns: [txn_0, txn_1]
    // proofs: [1, 2]
    // batch_summaries: [1 -> txn_0, 2 -> txn_1]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (2, 2));

    proof_queue.add_batch_summaries(vec![(info_3.clone(), vec![txns[0]])]);
    // txns: [txn_0, txn_1]
    // proofs: [1, 2]
    // batch_summaries: [1 -> txn_0, 2 -> txn_1, 3 -> txn_0]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (2, 2));

    // Adding the batch again shouldn't have an effect
    proof_queue.add_batch_summaries(vec![(info_3.clone(), vec![txns[0]])]);
    // txns: [txn_0, txn_1]
    // proofs: [1, 2]
    // batch_summaries: [1 -> txn_0, 2 -> txn_1, 3 -> txn_0]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (2, 2));

    proof_queue.push(author_0_batches[2].clone());
    // txns: [txn_0, txn_1]
    // proofs: [1, 2, 3]
    // batch_summaries: [1 -> txn_0, 2 -> txn_1, 3 -> txn_0]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (2, 3));

    // Adding the batch again shouldn't have an effect
    proof_queue.add_batch_summaries(vec![(info_3.clone(), vec![txns[0]])]);
    // txns: [txn_0, txn_1]
    // proofs: [1, 2, 3]
    // batch_summaries: [1 -> txn_0, 2 -> txn_1, 3 -> txn_0]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (2, 3));

    proof_queue.push(author_1_batches[0].clone());
    // txns: [txn_0, txn_1] + txns(proof_5)
    // proofs: [1, 2, 3, 5]
    // batch_summaries: [1 -> txn_0, 2 -> txn_1, 3 -> txn_0]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (3, 4));

    proof_queue.add_batch_summaries(vec![(info_5, vec![txns[1]])]);
    // txns: [txn_0, txn_1]
    // proofs: [1, 2, 3, 5]
    // batch_summaries: [1 -> txn_0, 2 -> txn_1, 3 -> txn_0, 5 -> txn_1]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (2, 4));

    proof_queue.add_batch_summaries(vec![(info_4, vec![txns[2]])]);
    // txns: [txn_0, txn_1]
    // proofs: [1, 2, 3, 5]
    // batch_summaries: [1 -> txn_0, 2 -> txn_1, 3 -> txn_0, 4 -> txn_2, 5 -> txn_1]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (2, 4));

    proof_queue.push(author_0_batches[3].clone());
    // txns: [txn_0, txn_1, txn_2]
    // proofs: [1, 2, 3, 4, 5]
    // batch_summaries: [1 -> txn_0, 2 -> txn_1, 3 -> txn_0, 4 -> txn_2, 5 -> txn_1]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (3, 5));

    proof_queue.mark_committed(vec![info_1.clone()]);
    // txns: [txn_0, txn_1, txn_2]
    // proofs: [2, 3, 4, 5]
    // batch_summaries: [2 -> txn_1, 3 -> txn_0, 4 -> txn_2, 5 -> txn_1]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (3, 4));

    proof_queue.push(author_1_batches[1].clone());
    // txns: [txn_0, txn_1, txn_2] + txns(proof_6)
    // proofs: [2, 3, 4, 5, 6]
    // batch_summaries: [2 -> txn_1, 3 -> txn_0, 4 -> txn_2, 5 -> txn_1]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (4, 5));

    proof_queue.handle_updated_block_timestamp(now + 20000);
    // Expires info_3, info_4, info_5
    // txns: [txn_1] + txns(proof_6)
    // proofs: [2, 6]
    // batch_summaries: [2 -> txn_1]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (2, 2));

    // Adding an expired batch again
    proof_queue.add_batch_summaries(vec![(info_3, vec![txns[0]])]);
    // txns: [txn_1] + txns(proof_6)
    // proofs: [2, 6]
    // batch_summaries: [2 -> txn_1, 3 -> txn_0]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (2, 2));

    // Adding an expired proof again. Should have no effect
    proof_queue.push(author_0_batches[2].clone());
    // txns: [txn_1] + txns(proof_6)
    // proofs: [2, 6]
    // batch_summaries: [2 -> txn_1, 3 -> txn_0]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (2, 2));

    proof_queue.add_batch_summaries(vec![(info_7, vec![txns[3]])]);
    // txns: [txn_1] + txns(proof_6)
    // proofs: [2, 6]
    // batch_summaries: [2 -> txn_1, 7 -> txn_3, 3 -> txn_0]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (2, 2));

    proof_queue.handle_updated_block_timestamp(now + 30000);
    // Expires info_6
    // txns: [txn_1]
    // proofs: [2]
    // batch_summaries: [2 -> txn_1, 7 -> txn_3, 3 -> txn_0]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (1, 1));

    proof_queue.add_batch_summaries(vec![(info_6, vec![txns[0]])]);
    // Expired batch not added to batch summaries
    // txns: [txn_1]
    // proofs: [2]
    // batch_summaries: [2 -> txn_1, 7 -> txn_3, 3 -> txn_0]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (1, 1));

    proof_queue.push(author_1_batches[2].clone());
    // txns: [txn_1, txn_3]
    // proofs: [2, 7]
    // batch_summaries: [2 -> txn_1, 7 -> txn_3, 3 -> txn_0]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (2, 2));

    proof_queue.push(author_1_batches[3].clone());
    // txns: [txn_1, txn_3] + txns(proof_8)
    // proofs: [2, 7, 8]
    // batch_summaries: [2 -> txn_1, 7 -> txn_3, 3 -> txn_0]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (3, 3));

    proof_queue.mark_committed(vec![info_8.clone()]);
    // txns: [txn_1, txn_3]
    // proofs: [2, 7]
    // batch_summaries: [2 -> txn_1, 7 -> txn_3, 3 -> txn_0]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (2, 2));

    proof_queue.add_batch_summaries(vec![(info_8, vec![txns[0]])]);
    // Committed batch not added to batch summaries
    // txns: [txn_1, txn_3]
    // proofs: [2, 7]
    // batch_summaries: [2 -> txn_1, 7 -> txn_3, 3 -> txn_0]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (2, 2));

    proof_queue.push(author_1_batches[3].clone());
    // Committed proof added again. Should have no effect
    // txns: [txn_1, txn_3]
    // proofs: [2, 7, 8]
    // batch_summaries: [2 -> txn_1, 7 -> txn_3, 3 -> txn_0]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (2, 2));

    proof_queue.handle_updated_block_timestamp(now + 70000);
    // Expires info_2, info_7
    // txns: []
    // proofs: []
    // batch_summaries: [3 -> txn_0]
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (0, 0));
}

#[test]
fn test_proof_pull_proofs_with_duplicates() {
    let my_peer_id = PeerId::random();
    let mut proof_queue = ProofQueue::new(my_peer_id);
    let now = aptos_infallible::duration_since_epoch().as_micros() as u64;
    let now_in_secs = aptos_infallible::duration_since_epoch().as_secs() as u64;
    let txns = vec![
        TxnSummaryWithExpiration::new(PeerId::ONE, 0, now_in_secs + 1, HashValue::zero()),
        TxnSummaryWithExpiration::new(PeerId::ONE, 1, now_in_secs + 1, HashValue::zero()),
        TxnSummaryWithExpiration::new(PeerId::ONE, 2, now_in_secs + 1, HashValue::zero()),
        TxnSummaryWithExpiration::new(PeerId::ONE, 3, now_in_secs + 1, HashValue::zero()),
    ];

    let author_0 = PeerId::random();
    let author_1 = PeerId::random();

    let author_0_batches = vec![
        proof_of_store(author_0, BatchId::new_for_test(0), 100, now + 1_000_000),
        proof_of_store(author_0, BatchId::new_for_test(1), 200, now + 2_000_000),
        proof_of_store(author_0, BatchId::new_for_test(2), 50, now + 3_000_000),
        proof_of_store(author_0, BatchId::new_for_test(3), 300, now + 2_000_000),
    ];
    let info_0 = author_0_batches[0].info().clone();
    proof_queue.add_batch_summaries(vec![(author_0_batches[0].info().clone(), vec![txns[0]])]);
    proof_queue.add_batch_summaries(vec![(author_0_batches[1].info().clone(), vec![txns[1]])]);
    proof_queue.add_batch_summaries(vec![(author_0_batches[2].info().clone(), vec![txns[2]])]);
    proof_queue.add_batch_summaries(vec![(author_0_batches[3].info().clone(), vec![txns[0]])]);

    proof_queue.push(batch);
    for batch in author_0_batches {
        proof_queue.push(batch);
    }

    let author_1_batches = vec![
        proof_of_store(author_1, BatchId::new_for_test(4), 500, now + 5000),
        proof_of_store(author_1, BatchId::new_for_test(5), 400, now + 5000),
        proof_of_store(author_1, BatchId::new_for_test(6), 600, now + 5000),
        proof_of_store(author_1, BatchId::new_for_test(7), 50, now + 5000),
    ];
    proof_queue.add_batch_summaries(vec![(author_1_batches[0].info().clone(), vec![txns[1]])]);
    proof_queue.add_batch_summaries(vec![(author_1_batches[1].info().clone(), vec![txns[2]])]);
    proof_queue.add_batch_summaries(vec![(author_1_batches[2].info().clone(), vec![txns[3]])]);
    proof_queue.add_batch_summaries(vec![(author_1_batches[3].info().clone(), vec![txns[0]])]);

    for batch in author_1_batches {
        proof_queue.push(batch);
    }
    assert_eq!(proof_queue.remaining_txns_and_proofs(), (4, 8));

    let result = proof_queue.pull_proofs(&hashset![], 8, 4, 3000, true);
    assert!(result.0.len() >= 4);
    assert!(result.0.len() <= 8);
    let mut pulled_txns = HashSet::new();
    for proof in result.0 {
        match proof.batch_id() {
            BatchId { id: 0, nonce: 0 } => pulled_txns.insert(0),
            BatchId { id: 1, nonce: 0 } => pulled_txns.insert(1),
            BatchId { id: 2, nonce: 0 } => pulled_txns.insert(2),
            BatchId { id: 3, nonce: 0 } => pulled_txns.insert(0),
            BatchId { id: 4, nonce: 0 } => pulled_txns.insert(1),
            BatchId { id: 5, nonce: 0 } => pulled_txns.insert(2),
            BatchId { id: 6, nonce: 0 } => pulled_txns.insert(3),
            BatchId { id: 7, nonce: 0 } => pulled_txns.insert(0),
            _ => panic!("Unexpected batch id"),
        };
    }
    assert!(pulled_txns.len() == 4);
    assert!(result.1 == 4);
    assert!(
        proof_queue
            .pull_proofs(&hashset![info_0], 8, 4, 400, true)
            .0
            .len()
            == 7
    );
}
