// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::{
    batch_aggregator::{BatchAggregationError, BatchAggregator, IncrementalBatchState},
    tests::utils::create_vec_serialized_transactions,
    types::{BatchId, SerializedTransaction},
};
use aptos_types::transaction::SignedTransaction;
use claims::{assert_ge, assert_matches, assert_ok, assert_ok_eq};

fn split_vec(txns: &[SerializedTransaction], size: usize) -> Vec<Vec<SerializedTransaction>> {
    let mut ret = Vec::new();
    for chunk in txns.chunks(size) {
        ret.push(chunk.to_vec());
    }
    ret
}

fn append_fragments(
    state: &mut IncrementalBatchState,
    fragmented_txns: Vec<Vec<SerializedTransaction>>,
) {
    for (i, txns) in fragmented_txns.into_iter().enumerate() {
        assert!(state.append_transactions(txns).is_ok());
        assert_eq!(state.num_fragments(), i + 1);
    }
}

#[test]
fn test_batch_state_fragments() {
    let num_txns = 10;

    let all_txns = create_vec_serialized_transactions(num_txns);
    let txns_size = all_txns[0].len() * (num_txns as usize);
    let fragmented_txns1 = split_vec(&all_txns, 5);
    let fragmented_txns2 = split_vec(&all_txns, 3);

    let mut state_whole = IncrementalBatchState::new(txns_size * 2);
    let mut state_fragments1 = IncrementalBatchState::new(txns_size);
    let mut state_fragments2 = IncrementalBatchState::new(txns_size);

    assert_eq!(state_whole.num_fragments(), 0);
    assert_eq!(state_fragments1.num_fragments(), 0);
    assert_eq!(state_fragments2.num_fragments(), 0);

    assert_ok!(state_whole.append_transactions(all_txns));
    assert_eq!(state_whole.num_fragments(), 1);

    append_fragments(&mut state_fragments1, fragmented_txns1);
    append_fragments(&mut state_fragments2, fragmented_txns2);

    let batch = state_whole.finalize_batch();
    let batch1 = state_fragments1.finalize_batch();
    let batch2 = state_fragments2.finalize_batch();

    assert!(batch.is_ok());
    let whole_res = batch.unwrap();
    assert_eq!(whole_res.1.len(), num_txns as usize);
    assert_eq!(whole_res.0, txns_size);

    assert_ok_eq!(batch1, whole_res);
    assert_ok_eq!(batch2, whole_res);
}

#[test]
fn test_batch_state_size_limit() {
    let num_txns = 10;

    let all_txns = create_vec_serialized_transactions(num_txns);
    let txns_size = all_txns[0].len() * (num_txns as usize);

    assert_ge!(txns_size, 1500);
    let mut state_small = IncrementalBatchState::new(1500);
    assert_eq!(
        state_small.append_transactions(all_txns).unwrap_err(),
        BatchAggregationError::SizeLimitExceeded
    );
    assert_eq!(state_small.num_fragments(), 1);

    assert_eq!(
        state_small.append_transactions(Vec::new()).unwrap_err(),
        BatchAggregationError::SizeLimitExceeded
    );
    assert_eq!(state_small.num_fragments(), 2);

    let batch_overflow = state_small.finalize_batch();
    assert_eq!(
        batch_overflow.unwrap_err(),
        BatchAggregationError::SizeLimitExceeded
    );
}

#[test]
fn test_batch_state_deserialization() {
    let mut state = IncrementalBatchState::new(1000);
    let txns = vec![SerializedTransaction::from_bytes(vec![8, 2, 0])];

    assert_eq!(state.num_fragments(), 0);
    assert_eq!(
        state.append_transactions(txns).unwrap_err(),
        BatchAggregationError::DeserializationError
    );
    assert_eq!(state.num_fragments(), 1);

    assert_eq!(
        state.append_transactions(Vec::new()).unwrap_err(),
        BatchAggregationError::DeserializationError
    );
    assert_eq!(state.num_fragments(), 2);

    assert_eq!(
        state
            .append_transactions(create_vec_serialized_transactions(20))
            .unwrap_err(),
        BatchAggregationError::DeserializationError
    );

    // Would overflow a non-error state.
    assert_eq!(state.num_fragments(), 3);
    let batch = state.finalize_batch();
    assert_eq!(
        batch.unwrap_err(),
        BatchAggregationError::DeserializationError
    );
}

fn check_outdated_fragments(aggregator: &mut BatchAggregator, pairs: Vec<(BatchId, usize)>) {
    for (i, j) in pairs {
        assert_eq!(
            aggregator
                .append_transactions(i, j, Vec::new())
                .unwrap_err(),
            BatchAggregationError::OutdatedFragment
        );

        assert_eq!(
            aggregator.end_batch(i, j, Vec::new()).unwrap_err(),
            BatchAggregationError::OutdatedFragment
        );
    }
}

#[test]
fn test_batch_aggregator_ids() {
    let mut aggregator = BatchAggregator::new(1000);

    // Forwards batch_id to 3, realizes fragment 0 is skipped.
    assert_eq!(
        aggregator
            .append_transactions(BatchId::new_for_test(3), 1, Vec::new())
            .unwrap_err(),
        BatchAggregationError::MissedFragment
    );
    // Everything <= batch 3 is now outdated.
    for i in 0..3 {
        for j in 0..2 {
            assert_eq!(
                aggregator
                    .append_transactions(BatchId::new_for_test(i), j, Vec::new())
                    .unwrap_err(),
                BatchAggregationError::OutdatedFragment
            );
        }
    }

    assert_ok!(aggregator.append_transactions(BatchId::new_for_test(4), 0, Vec::new()));
    assert_ok!(aggregator.append_transactions(BatchId::new_for_test(4), 1, Vec::new()));
    assert_ok!(aggregator.append_transactions(BatchId::new_for_test(4), 2, Vec::new()));
    check_outdated_fragments(
        &mut aggregator,
        (0..5)
            .into_iter()
            .flat_map(move |i| {
                (0..3)
                    .into_iter()
                    .map(move |j| (BatchId::new_for_test(i), j))
            })
            .collect(),
    );
    assert_ok!(aggregator.append_transactions(BatchId::new_for_test(4), 3, Vec::new()));
    let _empty_vec: Vec<SignedTransaction> = Vec::new();
    assert_matches!(
        aggregator.end_batch(BatchId::new_for_test(4), 4, Vec::new()),
        Ok((0, _empty_vec, _))
    );

    // Starting with (3,0) should work for a newly created aggregator.
    aggregator = BatchAggregator::new(1000);
    assert_ok!(aggregator.append_transactions(BatchId::new_for_test(3), 0, Vec::new()));

    check_outdated_fragments(&mut aggregator, vec![
        (BatchId::new_for_test(3), 0),
        (BatchId::new_for_test(2), 0),
        (BatchId::new_for_test(2), 7),
        (BatchId::new_for_test(0), 0),
    ]);
    assert_matches!(
        aggregator.end_batch(BatchId::new_for_test(3), 1, Vec::new()),
        Ok((0, _empty_vec, _))
    );
}

#[test]
fn test_batch_aggregator() {
    let num_txns = 10;
    let all_txns = create_vec_serialized_transactions(num_txns);
    let all_txns_clone = all_txns.clone();
    let txns_size = all_txns[0].len() * (num_txns as usize);
    let fragmented_txns1 = split_vec(&all_txns, 5);
    let fragmented_txns2 = split_vec(&all_txns, 3);

    let mut base_state = IncrementalBatchState::new(txns_size);
    assert_ok!(base_state.append_transactions(all_txns));
    let base_res = base_state.finalize_batch().unwrap();
    assert_eq!(base_res.0, txns_size);

    let mut aggregator = BatchAggregator::new(txns_size);
    assert_ok!(aggregator.append_transactions(
        BatchId::new_for_test(4),
        0,
        fragmented_txns1[0].clone()
    ));
    assert_ok!(aggregator.append_transactions(
        BatchId::new_for_test(4),
        1,
        fragmented_txns1[1].clone()
    ));
    check_outdated_fragments(&mut aggregator, vec![
        (BatchId::new_for_test(4), 0),
        (BatchId::new_for_test(4), 1),
        (BatchId::new_for_test(3), 0),
    ]);
    assert_ok_eq!(
        aggregator.end_batch(BatchId::new_for_test(4), 2, Vec::new()),
        base_res
    );

    assert_ok!(aggregator.append_transactions(
        BatchId::new_for_test(6),
        0,
        fragmented_txns1[0].clone()
    ));
    assert_ok!(aggregator.append_transactions(BatchId::new_for_test(6), 1, Vec::new()));
    assert_ok!(aggregator.append_transactions(BatchId::new_for_test(6), 2, Vec::new()));
    assert_ok_eq!(
        aggregator.end_batch(BatchId::new_for_test(6), 3, fragmented_txns1[1].clone()),
        base_res
    );

    assert_eq!(
        aggregator
            .append_transactions(BatchId::new_for_test(6), 0, Vec::new())
            .unwrap_err(),
        BatchAggregationError::OutdatedFragment
    );

    assert_eq!(
        aggregator
            .append_transactions(BatchId::new_for_test(7), 1, fragmented_txns1[0].clone())
            .unwrap_err(),
        BatchAggregationError::MissedFragment
    );
    assert_eq!(
        aggregator
            .append_transactions(BatchId::new_for_test(7), 0, Vec::new())
            .unwrap_err(),
        BatchAggregationError::OutdatedFragment
    );
    assert_ok_eq!(
        aggregator.end_batch(BatchId::new_for_test(8), 0, all_txns_clone),
        base_res
    );

    assert_ok!(aggregator.append_transactions(
        BatchId::new_for_test(9),
        0,
        fragmented_txns1[0].clone()
    ));
    assert_ok!(aggregator.append_transactions(
        BatchId::new_for_test(9),
        1,
        fragmented_txns1[0].clone()
    ));
    assert_eq!(
        aggregator
            .append_transactions(BatchId::new_for_test(9), 2, fragmented_txns1[1].clone())
            .unwrap_err(),
        BatchAggregationError::SizeLimitExceeded
    );

    assert_eq!(
        aggregator
            .end_batch(BatchId::new_for_test(9), 2, Vec::new())
            .unwrap_err(),
        BatchAggregationError::OutdatedFragment
    );
    assert_eq!(
        aggregator
            .end_batch(BatchId::new_for_test(9), 3, Vec::new())
            .unwrap_err(),
        BatchAggregationError::SizeLimitExceeded
    );

    // Observes missed fragment but still starts aggregating 10. Since aggregation was
    // successful, the return type is Ok(()).
    assert_ok!(aggregator.append_transactions(BatchId::new_for_test(10), 0, Vec::new()));

    // Observes missed fragment but still processes batch 11.
    let half_res = aggregator
        .end_batch(BatchId::new_for_test(11), 0, fragmented_txns1[0].clone())
        .unwrap();
    assert_eq!(half_res.1.len(), 5);
    assert_ne!(half_res.2, base_res.2);

    for (i, txn) in fragmented_txns2.iter().enumerate().take(3) {
        assert_ok!(aggregator.append_transactions(BatchId::new_for_test(12), 2 * i, txn.clone()));
        assert_ok!(aggregator.append_transactions(
            BatchId::new_for_test(12),
            2 * i + 1,
            Vec::new()
        ));
    }
    assert_ok_eq!(
        aggregator.end_batch(BatchId::new_for_test(12), 6, fragmented_txns2[3].clone()),
        base_res
    );

    assert_ok!(aggregator.append_transactions(
        BatchId::new_for_test(15),
        0,
        fragmented_txns1[0].clone()
    ));
    assert_eq!(
        aggregator
            .end_batch(BatchId::new_for_test(15), 2, fragmented_txns1[1].clone())
            .unwrap_err(),
        BatchAggregationError::MissedFragment
    );
    assert_eq!(
        aggregator
            .end_batch(BatchId::new_for_test(15), 2, fragmented_txns1[1].clone())
            .unwrap_err(),
        BatchAggregationError::OutdatedFragment
    );

    assert_ok!(aggregator.append_transactions(
        BatchId::new_for_test(16),
        0,
        fragmented_txns1[0].clone()
    ));
    assert_eq!(
        aggregator
            .append_transactions(BatchId::new_for_test(16), 2, fragmented_txns1[1].clone())
            .unwrap_err(),
        BatchAggregationError::MissedFragment
    );
    assert_eq!(
        aggregator
            .end_batch(BatchId::new_for_test(16), 2, fragmented_txns1[1].clone())
            .unwrap_err(),
        BatchAggregationError::OutdatedFragment
    );

    assert_ok!(aggregator.end_batch(BatchId::new_for_test(17), 0, Vec::new()));
    assert_ok!(aggregator.end_batch(BatchId::new_for_test(18), 0, Vec::new()));
}
