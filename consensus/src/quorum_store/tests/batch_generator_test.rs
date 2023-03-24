// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::{
    batch_coordinator::BatchCoordinatorCommand, batch_generator::BatchGenerator,
    quorum_store_db::MockQuorumStoreDB, tests::utils::create_vec_signed_transactions,
};
use aptos_config::config::QuorumStoreConfig;
use aptos_consensus_types::{common::TransactionSummary, proof_of_store::BatchId};
use aptos_mempool::{QuorumStoreRequest, QuorumStoreResponse};
use aptos_types::transaction::SignedTransaction;
use futures::{
    channel::mpsc::{channel, Receiver},
    StreamExt,
};
use move_core_types::account_address::AccountAddress;
use std::{sync::Arc, time::Duration};
use tokio::{sync::mpsc::channel as TokioChannel, time::timeout};

async fn queue_mempool_batch_response(
    txns: Vec<SignedTransaction>,
    max_size: usize,
    quorum_store_to_mempool_receiver: &mut Receiver<QuorumStoreRequest>,
) -> Vec<TransactionSummary> {
    if let QuorumStoreRequest::GetBatchRequest(
        _max_batch_size,
        _max_bytes,
        _return_non_full,
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
        let ret = txns
            .into_iter()
            .take_while(|txn| {
                size += txn.raw_txn_bytes_len();
                size <= max_size
            })
            .collect();
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
        max_batch_bytes: max_size,
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
        if let BatchCoordinatorCommand::NewBatch(data) = quorum_store_command {
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
        if let BatchCoordinatorCommand::NewBatch(data) = quorum_store_command {
            assert_eq!(data.batch_id(), BatchId::new_for_test(2));
            let txns = data.into_transactions();
            assert_eq!(txns.len(), signed_txns.len() - 1);
            assert_eq!(txns, signed_txns[0..9].to_vec());
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
        if let BatchCoordinatorCommand::NewBatch(data) = quorum_store_command {
            assert_eq!(data.batch_id(), BatchId::new_for_test(3));
            let txns = data.into_transactions();
            assert_eq!(txns.len(), signed_txns.len());
            assert_eq!(txns, signed_txns);
        } else {
            panic!("Unexpected variant")
        }
    });

    for _ in 0..3 {
        let result = batch_generator.handle_scheduled_pull(300).await.unwrap();
        batch_coordinator_cmd_tx
            .send(BatchCoordinatorCommand::NewBatch(Box::new(result)))
            .await
            .unwrap();
    }
    timeout(Duration::from_millis(10_000), join_handle)
        .await
        .unwrap()
        .unwrap();
}
