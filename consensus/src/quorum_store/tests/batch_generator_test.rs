// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::{
    batch_coordinator::BatchCoordinatorCommand,
    batch_generator::BatchGenerator,
    quorum_store_db::MockQuorumStoreDB,
    tests::utils::{
        create_signed_transaction, create_vec_signed_transactions,
        create_vec_signed_transactions_with_gas,
    },
};
use aptos_config::config::QuorumStoreConfig;
use aptos_consensus_types::{common::TransactionInProgress, proof_of_store::BatchId};
use aptos_mempool::{QuorumStoreRequest, QuorumStoreResponse};
use aptos_types::transaction::SignedTransaction;
use futures::{
    channel::mpsc::{channel, Receiver},
    StreamExt,
};
use move_core_types::account_address::AccountAddress;
use std::{sync::Arc, time::Duration};
use tokio::{sync::mpsc::channel as TokioChannel, time::timeout};

#[allow(clippy::needless_collect)]
async fn queue_mempool_batch_response(
    txns: Vec<SignedTransaction>,
    max_size: usize,
    quorum_store_to_mempool_receiver: &mut Receiver<QuorumStoreRequest>,
) -> Vec<TransactionInProgress> {
    if let QuorumStoreRequest::GetBatchRequest(
        _max_batch_size,
        _max_bytes,
        _return_non_full,
        _include_gas_upgraded,
        exclude_txns,
        callback,
    ) = timeout(
        Duration::from_millis(1_000),
        quorum_store_to_mempool_receiver.select_next_some(),
    )
    .await
    .unwrap()
    {
        let mut size = 0;
        let mut sorted_txns = txns.clone();
        sorted_txns.sort_by_key(|txn| txn.gas_unit_price());
        let chosen_txns: Vec<_> = sorted_txns
            .into_iter()
            .rev()
            .take_while(|txn| {
                size += txn.raw_txn_bytes_len();
                size <= max_size
            })
            .collect();
        let ret: Vec<_> = chosen_txns.into_iter().rev().collect();
        callback
            .send(Ok(QuorumStoreResponse::GetBatchResponse(ret)))
            .unwrap();
        exclude_txns
    } else {
        panic!("Unexpected variant")
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_batch_creation() {
    let (quorum_store_to_mempool_tx, mut quorum_store_to_mempool_rx) = channel(1_024);
    let (batch_coordinator_cmd_tx, mut batch_coordinator_cmd_rx) = TokioChannel(100);

    let txn_size = 69;
    let max_size = 9 * txn_size + 1;

    let config = QuorumStoreConfig {
        sender_max_batch_bytes: max_size,
        ..Default::default()
    };

    let mut batch_generator = BatchGenerator::new(
        0,
        AccountAddress::random(),
        config,
        Arc::new(MockQuorumStoreDB::new()),
        quorum_store_to_mempool_tx,
        1000,
    );

    let join_handle = tokio::spawn(async move {
        let mut num_txns = 0;

        let signed_txns = create_vec_signed_transactions(1);
        queue_mempool_batch_response(
            signed_txns.clone(),
            max_size,
            &mut quorum_store_to_mempool_rx,
        )
        .await;
        // Expect Batch for 1 txn
        let quorum_store_command = batch_coordinator_cmd_rx.recv().await.unwrap();
        if let BatchCoordinatorCommand::NewBatches(data) = quorum_store_command {
            assert_eq!(1, data.len());
            let data = data[0].clone();
            assert_eq!(data.batch_id(), BatchId::new_for_test(1));
            let txns = data.into_transactions();
            assert_eq!(txns.len(), signed_txns.len());
            assert_eq!(txns, signed_txns);
        } else {
            panic!("Unexpected variant")
        }
        num_txns += 1;

        let signed_txns = create_vec_signed_transactions(10);
        // Expect single exclude_txn
        let exclude_txns = queue_mempool_batch_response(
            signed_txns.clone(),
            max_size,
            &mut quorum_store_to_mempool_rx,
        )
        .await;
        assert_eq!(exclude_txns.len(), num_txns);
        // Expect Batch for 9 (due to size limit).
        let quorum_store_command = batch_coordinator_cmd_rx.recv().await.unwrap();
        if let BatchCoordinatorCommand::NewBatches(data) = quorum_store_command {
            assert_eq!(1, data.len());
            let data = data[0].clone();
            assert_eq!(data.batch_id(), BatchId::new_for_test(2));
            let txns = data.into_transactions();
            assert_eq!(txns.len(), signed_txns.len() - 1);
            // let expected: Vec<_> = signed_txns[0..9].iter().rev().cloned().collect();
            // assert_eq!(txns, expected);
        } else {
            panic!("Unexpected variant")
        }
        num_txns += 9;

        let signed_txns = create_vec_signed_transactions(9);
        // Expect 1 + 9 = 10 exclude_txn
        let exclude_txns = queue_mempool_batch_response(
            signed_txns.clone(),
            max_size,
            &mut quorum_store_to_mempool_rx,
        )
        .await;
        assert_eq!(exclude_txns.len(), num_txns);
        // Expect AppendBatch for 9 txns
        let quorum_store_command = batch_coordinator_cmd_rx.recv().await.unwrap();
        if let BatchCoordinatorCommand::NewBatches(data) = quorum_store_command {
            assert_eq!(1, data.len());
            let data = data[0].clone();
            assert_eq!(data.batch_id(), BatchId::new_for_test(3));
            let txns = data.into_transactions();
            assert_eq!(txns.len(), signed_txns.len());
            // assert_eq!(txns, signed_txns);
        } else {
            panic!("Unexpected variant")
        }
    });

    for _ in 0..3 {
        let result = batch_generator.handle_scheduled_pull(300).await;
        batch_coordinator_cmd_tx
            .send(BatchCoordinatorCommand::NewBatches(result))
            .await
            .unwrap();
    }
    timeout(Duration::from_millis(10_000), join_handle)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_bucketed_batch_creation() {
    let (quorum_store_to_mempool_tx, mut quorum_store_to_mempool_rx) = channel(1_024);
    let (batch_coordinator_cmd_tx, mut batch_coordinator_cmd_rx) = TokioChannel(100);

    let txn_size = 69;
    let max_size = 9 * txn_size + 1;

    let config = QuorumStoreConfig {
        sender_max_batch_bytes: max_size,
        ..Default::default()
    };
    let buckets = config.batch_buckets.clone();

    let mut batch_generator = BatchGenerator::new(
        0,
        AccountAddress::random(),
        config,
        Arc::new(MockQuorumStoreDB::new()),
        quorum_store_to_mempool_tx,
        1000,
    );

    let mut num_txns = 0;

    let join_handle = tokio::spawn(async move {
        let signed_txns = create_vec_signed_transactions_with_gas(1, buckets[1]);
        queue_mempool_batch_response(
            signed_txns.clone(),
            max_size,
            &mut quorum_store_to_mempool_rx,
        )
        .await;

        // Expect Batch for 1 txn
        let quorum_store_command = batch_coordinator_cmd_rx.recv().await.unwrap();
        if let BatchCoordinatorCommand::NewBatches(data) = quorum_store_command {
            assert_eq!(1, data.len());
            let data = data[0].clone();
            assert_eq!(data.batch_id(), BatchId::new_for_test(1));
            let txns = data.into_transactions();
            assert_eq!(txns.len(), signed_txns.len());
            assert_eq!(txns, signed_txns);
        } else {
            panic!("Unexpected variant")
        }
        num_txns += 1;

        let mut signed_txns = vec![];
        let bucket_0 = create_vec_signed_transactions_with_gas(3, buckets[0]);
        signed_txns.append(&mut bucket_0.clone());
        let bucket_1 = create_vec_signed_transactions_with_gas(3, buckets[1]);
        signed_txns.append(&mut bucket_1.clone());
        let bucket_4 = create_vec_signed_transactions_with_gas(4, buckets[4]);
        signed_txns.append(&mut bucket_4.clone());
        // Expect single exclude_txn
        let exclude_txns = queue_mempool_batch_response(
            signed_txns.clone(),
            max_size,
            &mut quorum_store_to_mempool_rx,
        )
        .await;
        assert_eq!(exclude_txns.len(), num_txns);
        // Expect Batch for 9 (due to size limit).
        let quorum_store_command = batch_coordinator_cmd_rx.recv().await.unwrap();
        if let BatchCoordinatorCommand::NewBatches(batches) = quorum_store_command {
            assert_eq!(3, batches.len());

            let data = batches[0].clone();
            assert_eq!(data.batch_id(), BatchId::new_for_test(2));
            assert_eq!(data.batch_info().gas_bucket_start(), buckets[4]);
            let txns = data.into_transactions();
            // This gas bucket should have all elements
            assert_eq!(txns.len(), bucket_4.len());

            let data = batches[1].clone();
            assert_eq!(data.batch_id(), BatchId::new_for_test(3));
            assert_eq!(data.batch_info().gas_bucket_start(), buckets[1]);
            let txns = data.into_transactions();
            // This gas bucket should have all elements
            assert_eq!(txns.len(), bucket_1.len());

            let data = batches[2].clone();
            assert_eq!(data.batch_id(), BatchId::new_for_test(4));
            assert_eq!(data.batch_info().gas_bucket_start(), buckets[0]);
            let txns = data.into_transactions();
            // As only 9 items fit, the least gas bucket has less items.
            assert_eq!(txns.len(), bucket_0.len() - 1);
        } else {
            panic!("Unexpected variant")
        }
        num_txns += 9;

        let signed_txns = create_vec_signed_transactions_with_gas(9, u64::MAX);
        // Expect 1 + 9 = 10 exclude_txn
        let exclude_txns = queue_mempool_batch_response(
            signed_txns.clone(),
            max_size,
            &mut quorum_store_to_mempool_rx,
        )
        .await;
        assert_eq!(exclude_txns.len(), num_txns);
        // Expect AppendBatch for 9 txns
        let quorum_store_command = batch_coordinator_cmd_rx.recv().await.unwrap();
        if let BatchCoordinatorCommand::NewBatches(data) = quorum_store_command {
            assert_eq!(1, data.len());
            let data = data[0].clone();
            assert_eq!(data.batch_id(), BatchId::new_for_test(5));
            assert_eq!(
                data.batch_info().gas_bucket_start(),
                buckets[buckets.len() - 1]
            );
            let txns = data.into_transactions();
            assert_eq!(txns.len(), signed_txns.len());
        } else {
            panic!("Unexpected variant")
        }
    });

    for _ in 0..3 {
        let result = batch_generator.handle_scheduled_pull(300).await;
        batch_coordinator_cmd_tx
            .send(BatchCoordinatorCommand::NewBatches(result))
            .await
            .unwrap();
    }
    timeout(Duration::from_millis(10_000), join_handle)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn test_max_batch_txns() {
    let (quorum_store_to_mempool_tx, mut quorum_store_to_mempool_rx) = channel(1_024);
    let (batch_coordinator_cmd_tx, mut batch_coordinator_cmd_rx) = TokioChannel(100);

    let config = QuorumStoreConfig {
        sender_max_batch_txns: 10,
        ..Default::default()
    };
    let max_batch_bytes = config.sender_max_batch_bytes;

    let mut batch_generator = BatchGenerator::new(
        0,
        AccountAddress::random(),
        config,
        Arc::new(MockQuorumStoreDB::new()),
        quorum_store_to_mempool_tx,
        1000,
    );

    let join_handle = tokio::spawn(async move {
        let signed_txns = create_vec_signed_transactions(25);
        queue_mempool_batch_response(
            signed_txns.clone(),
            max_batch_bytes,
            &mut quorum_store_to_mempool_rx,
        )
        .await;

        let quorum_store_command = batch_coordinator_cmd_rx.recv().await.unwrap();
        if let BatchCoordinatorCommand::NewBatches(result) = quorum_store_command {
            assert_eq!(result.len(), 3);
            assert_eq!(result[0].num_txns(), 10);
            assert_eq!(result[1].num_txns(), 10);
            assert_eq!(result[2].num_txns(), 5);

            assert_eq!(&result[0].clone().into_transactions(), &signed_txns[0..10]);
            assert_eq!(&result[1].clone().into_transactions(), &signed_txns[10..20]);
            assert_eq!(&result[2].clone().into_transactions(), &signed_txns[20..]);
        } else {
            panic!("Unexpected variant")
        }
    });

    let result = batch_generator.handle_scheduled_pull(300).await;
    batch_coordinator_cmd_tx
        .send(BatchCoordinatorCommand::NewBatches(result))
        .await
        .unwrap();

    timeout(Duration::from_millis(10_000), join_handle)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn test_last_bucketed_batch() {
    let (quorum_store_to_mempool_tx, mut quorum_store_to_mempool_rx) = channel(1_024);
    let (batch_coordinator_cmd_tx, mut batch_coordinator_cmd_rx) = TokioChannel(100);

    let config = QuorumStoreConfig {
        sender_max_batch_txns: 10,
        ..Default::default()
    };
    let max_batch_bytes = config.sender_max_batch_bytes;
    let buckets = config.batch_buckets.clone();

    let mut batch_generator = BatchGenerator::new(
        0,
        AccountAddress::random(),
        config,
        Arc::new(MockQuorumStoreDB::new()),
        quorum_store_to_mempool_tx,
        1000,
    );

    let join_handle = tokio::spawn(async move {
        let low_gas_txn = create_signed_transaction(1);
        let high_gas_txn_other_account = create_signed_transaction(u64::MAX);
        let signed_txns = vec![low_gas_txn, high_gas_txn_other_account];

        queue_mempool_batch_response(
            signed_txns.clone(),
            max_batch_bytes,
            &mut quorum_store_to_mempool_rx,
        )
        .await;

        let quorum_store_command = batch_coordinator_cmd_rx.recv().await.unwrap();
        if let BatchCoordinatorCommand::NewBatches(result) = quorum_store_command {
            assert_eq!(result.len(), 2);
            assert_eq!(result[0].num_txns(), 1);
            assert_eq!(result[1].num_txns(), 1);
            assert_eq!(result[0].gas_bucket_start(), buckets[buckets.len() - 1]);
            assert_eq!(result[1].gas_bucket_start(), 0);

            assert_eq!(&result[0].clone().into_transactions(), &signed_txns[1..]);
            assert_eq!(&result[1].clone().into_transactions(), &signed_txns[0..1]);
        } else {
            panic!("Unexpected variant")
        }
    });

    let result = batch_generator.handle_scheduled_pull(300).await;
    batch_coordinator_cmd_tx
        .send(BatchCoordinatorCommand::NewBatches(result))
        .await
        .unwrap();

    timeout(Duration::from_millis(10_000), join_handle)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn test_sender_max_num_batches_single_bucket() {
    let (quorum_store_to_mempool_tx, mut quorum_store_to_mempool_rx) = channel(1_024);
    let (batch_coordinator_cmd_tx, mut batch_coordinator_cmd_rx) = TokioChannel(100);

    let config = QuorumStoreConfig {
        sender_max_batch_txns: 10,
        sender_max_num_batches: 3,
        ..Default::default()
    };
    let max_batch_txns = config.sender_max_batch_txns;
    let max_batch_bytes = config.sender_max_batch_bytes;
    let max_num_batches = config.sender_max_num_batches;

    let mut batch_generator = BatchGenerator::new(
        0,
        AccountAddress::random(),
        config,
        Arc::new(MockQuorumStoreDB::new()),
        quorum_store_to_mempool_tx,
        1000,
    );

    let join_handle = tokio::spawn(async move {
        let signed_txns =
            create_vec_signed_transactions((max_batch_txns * max_num_batches + 1) as u64);
        queue_mempool_batch_response(
            signed_txns.clone(),
            max_batch_bytes,
            &mut quorum_store_to_mempool_rx,
        )
        .await;

        let quorum_store_command = batch_coordinator_cmd_rx.recv().await.unwrap();
        if let BatchCoordinatorCommand::NewBatches(result) = quorum_store_command {
            assert_eq!(result.len(), max_num_batches);
            for batch in &result {
                assert_eq!(batch.num_txns(), max_batch_txns as u64);
            }
        } else {
            panic!("Unexpected variant")
        }
    });

    let result = batch_generator.handle_scheduled_pull(300).await;
    batch_coordinator_cmd_tx
        .send(BatchCoordinatorCommand::NewBatches(result))
        .await
        .unwrap();

    timeout(Duration::from_millis(10_000), join_handle)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn test_sender_max_num_batches_multi_buckets() {
    let (quorum_store_to_mempool_tx, mut quorum_store_to_mempool_rx) = channel(1_024);
    let (batch_coordinator_cmd_tx, mut batch_coordinator_cmd_rx) = TokioChannel(100);

    let config = QuorumStoreConfig {
        sender_max_batch_txns: 10,
        sender_max_num_batches: 3,
        ..Default::default()
    };
    let max_batch_txns = config.sender_max_batch_txns;
    let max_batch_bytes = config.sender_max_batch_bytes;
    let max_num_batches = config.sender_max_num_batches;
    let buckets = config.batch_buckets.clone();

    let mut batch_generator = BatchGenerator::new(
        0,
        AccountAddress::random(),
        config,
        Arc::new(MockQuorumStoreDB::new()),
        quorum_store_to_mempool_tx,
        1000,
    );

    let join_handle = tokio::spawn(async move {
        let mut signed_txns = vec![];
        for min_gas_price in buckets.iter().take(max_num_batches) {
            let mut new_txns = create_vec_signed_transactions_with_gas(
                max_batch_txns as u64 + 1,
                *min_gas_price + 1,
            );
            signed_txns.append(&mut new_txns);
        }
        queue_mempool_batch_response(
            signed_txns.clone(),
            max_batch_bytes,
            &mut quorum_store_to_mempool_rx,
        )
        .await;

        let quorum_store_command = batch_coordinator_cmd_rx.recv().await.unwrap();
        if let BatchCoordinatorCommand::NewBatches(result) = quorum_store_command {
            assert_eq!(result.len(), max_num_batches);
            for (i, batch) in result.iter().enumerate() {
                if i % 2 == 0 {
                    assert_eq!(batch.num_txns(), max_batch_txns as u64);
                } else {
                    assert_eq!(batch.num_txns(), 1);
                }
            }
        } else {
            panic!("Unexpected variant")
        }
    });

    let result = batch_generator.handle_scheduled_pull(300).await;
    batch_coordinator_cmd_tx
        .send(BatchCoordinatorCommand::NewBatches(result))
        .await
        .unwrap();

    timeout(Duration::from_millis(10_000), join_handle)
        .await
        .unwrap()
        .unwrap();
}
