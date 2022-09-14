// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::error::DbError;
use crate::quorum_store::{
    quorum_store::{QuorumStoreCommand, QuorumStoreConfig},
    quorum_store_db::BatchIdDB,
    quorum_store_wrapper::QuorumStoreWrapper,
    tests::utils::{create_vec_serialized_transactions, create_vec_signed_transactions},
    types::{BatchId, SerializedTransaction},
};
use aptos_crypto::HashValue;
use aptos_mempool::{QuorumStoreRequest, QuorumStoreResponse};
use aptos_types::aggregate_signature::AggregateSignature;
use aptos_types::transaction::SignedTransaction;
use consensus_types::{
    common::{Payload, PayloadFilter, TransactionSummary},
    proof_of_store::{LogicalTime, ProofOfStore, SignedDigestInfo},
    request_response::{ConsensusResponse, WrapperCommand},
};
use futures::{
    channel::{
        mpsc::{channel, Receiver},
        oneshot,
    },
    StreamExt,
};
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio::{sync::mpsc::channel as TokioChannel, time::timeout};

pub struct MockBatchIdDB {}

impl MockBatchIdDB {
    pub fn new() -> Self {
        Self {}
    }
}

impl BatchIdDB for MockBatchIdDB {
    // The first batch will be index 1
    fn clean_and_get_batch_id(
        &self,
        _current_epoch: u64,
    ) -> anyhow::Result<Option<BatchId>, DbError> {
        Ok(Some(0))
    }

    fn save_batch_id(&self, _epoch: u64, _batch_id: BatchId) -> anyhow::Result<(), DbError> {
        Ok(())
    }
}

async fn queue_mempool_batch_response(
    txns: Vec<SignedTransaction>,
    quorum_store_to_mempool_receiver: &mut Receiver<QuorumStoreRequest>,
) -> Vec<TransactionSummary> {
    if let QuorumStoreRequest::GetBatchRequest(
        _max_batch_size,
        _max_bytes,
        exclude_txns,
        callback,
    ) = timeout(
        Duration::from_millis(1_000),
        quorum_store_to_mempool_receiver.select_next_some(),
    )
    .await
    .unwrap()
    {
        callback
            .send(Ok(QuorumStoreResponse::GetBatchResponse(txns)))
            .unwrap();
        exclude_txns
    } else {
        panic!("Unexpected variant")
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_batch_creation() {
    let (quorum_store_to_mempool_tx, mut quorum_store_to_mempool_rx) = channel(1_024);
    let (wrapper_quorum_store_tx, mut wrapper_quorum_store_rx) = TokioChannel(100);

    let txn_size = 168;
    create_vec_serialized_transactions(50)
        .iter()
        .for_each(|txn| assert_eq!(txn_size, txn.len()));

    let config = QuorumStoreConfig {
        channel_size: 100,
        proof_timeout_ms: 1000,
        batch_request_num_peers: 3,
        end_batch_ms: 500,
        max_batch_bytes: 9 * txn_size,
        batch_request_timeout_ms: 1000,
        max_batch_expiry_round_gap: 20,
        batch_expiry_grace_rounds: 5,
        memory_quota: 100000000,
        db_quota: 10000000000,
        mempool_txn_pull_max_count: 100,
        mempool_txn_pull_max_bytes: 1000000,
    };

    let mut wrapper = QuorumStoreWrapper::new(
        0,
        Arc::new(MockBatchIdDB::new()),
        quorum_store_to_mempool_tx,
        wrapper_quorum_store_tx,
        10_000,
        config.mempool_txn_pull_max_count,
        config.mempool_txn_pull_max_bytes,
        config.max_batch_bytes as u64,
        config.max_batch_expiry_round_gap,
        config.end_batch_ms,
    );

    let serialize = |signed_txns: &Vec<SignedTransaction>| -> Vec<SerializedTransaction> {
        signed_txns
            .iter()
            .map(|signed_txn| SerializedTransaction::from_signed_txn(signed_txn))
            .collect()
    };

    let join_handle = tokio::spawn(async move {
        let mut num_txns = 0;

        let signed_txns = create_vec_signed_transactions(1);
        queue_mempool_batch_response(
            vec![signed_txns[0].clone()],
            &mut quorum_store_to_mempool_rx,
        )
        .await;
        // Expect AppendToBatch for 1 txn
        let quorum_store_command = wrapper_quorum_store_rx.recv().await.unwrap();
        if let QuorumStoreCommand::AppendToBatch(data, batch_id) = quorum_store_command {
            assert_eq!(batch_id, 1);
            assert_eq!(data.len(), signed_txns.len());
            assert_eq!(data, serialize(&signed_txns));
        } else {
            panic!("Unexpected variant")
        }
        num_txns += 1;

        let signed_txns = create_vec_signed_transactions(9);
        // Expect single exclude_txn
        let exclude_txns =
            queue_mempool_batch_response(signed_txns.clone(), &mut quorum_store_to_mempool_rx)
                .await;
        assert_eq!(exclude_txns.len(), num_txns);
        // Expect EndBatch for 1 + 9 = 10 txns. The last txn pulled is not included in the batch.
        let quorum_store_command = wrapper_quorum_store_rx.recv().await.unwrap();
        if let QuorumStoreCommand::EndBatch(data, _, _, _) = quorum_store_command {
            assert_eq!(data.len(), signed_txns.len() - 1);
            assert_eq!(data, serialize(&signed_txns[0..8].to_vec()));
        } else {
            panic!("Unexpected variant")
        }
        num_txns += 8;

        let signed_txns = create_vec_signed_transactions(9);
        // Expect 1 + 8 = 9 exclude_txn
        let exclude_txns =
            queue_mempool_batch_response(signed_txns.clone(), &mut quorum_store_to_mempool_rx)
                .await;
        assert_eq!(exclude_txns.len(), num_txns);
        // Expect AppendBatch for 9 txns
        let quorum_store_command = wrapper_quorum_store_rx.recv().await.unwrap();
        if let QuorumStoreCommand::AppendToBatch(data, batch_id) = quorum_store_command {
            assert_eq!(batch_id, 2);
            assert_eq!(data.len(), signed_txns.len());
            assert_eq!(data, serialize(&signed_txns));
        } else {
            panic!("Unexpected variant")
        }
    });

    let result = wrapper.handle_scheduled_pull().await;
    assert!(result.is_none());
    let result = wrapper.handle_scheduled_pull().await;
    assert!(result.is_some());
    let result = wrapper.handle_scheduled_pull().await;
    assert!(result.is_none());

    timeout(Duration::from_millis(10_000), join_handle)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn test_block_request() {
    let (quorum_store_to_mempool_tx, mut _quorum_store_to_mempool_rx) = channel(1_024);
    let (wrapper_quorum_store_tx, mut _wrapper_quorum_store_rx) = TokioChannel(100);

    let config = QuorumStoreConfig {
        channel_size: 100,
        proof_timeout_ms: 1000,
        batch_request_num_peers: 3,
        end_batch_ms: 500,
        max_batch_bytes: 1000000,
        batch_request_timeout_ms: 1000,
        max_batch_expiry_round_gap: 20,
        batch_expiry_grace_rounds: 5,
        memory_quota: 100000000,
        db_quota: 10000000000,
        mempool_txn_pull_max_count: 100,
        mempool_txn_pull_max_bytes: 1000000,
    };

    let mut wrapper = QuorumStoreWrapper::new(
        0,
        Arc::new(MockBatchIdDB::new()),
        quorum_store_to_mempool_tx,
        wrapper_quorum_store_tx,
        10_000,
        config.mempool_txn_pull_max_count,
        config.mempool_txn_pull_max_bytes,
        config.max_batch_bytes as u64,
        config.max_batch_expiry_round_gap,
        config.end_batch_ms,
    );

    let digest = HashValue::random();
    let proof = ProofOfStore::new(
        SignedDigestInfo::new(digest, LogicalTime::new(0, 10)),
        AggregateSignature::empty(),
    );
    wrapper.insert_proof(proof.clone()).await;

    let (callback_tx, callback_rx) = oneshot::channel();
    let req = WrapperCommand::GetBlockRequest(
        1,
        100,
        1000000,
        PayloadFilter::InQuorumStore(HashSet::new()),
        callback_tx,
    );
    wrapper.handle_consensus_request(req).await;
    let ConsensusResponse::GetBlockResponse(payload) = callback_rx.await.unwrap().unwrap();
    if let Payload::InQuorumStore(proofs) = payload {
        assert_eq!(proofs.len(), 1);
        assert_eq!(proofs[0], proof);
    } else {
        panic!("Unexpected variant")
    }
}
