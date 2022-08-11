// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::direct_mempool_quorum_store::DirectMempoolQuorumStore;
use aptos_mempool::{QuorumStoreRequest, QuorumStoreResponse};
use consensus_types::{
    common::{Payload, PayloadFilter},
    request_response::{ConsensusRequest, ConsensusResponse},
};
use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test(flavor = "multi_thread")]
async fn test_block_request_no_txns() {
    let (quorum_store_to_mempool_sender, mut quorum_store_to_mempool_receiver) =
        mpsc::channel(1_024);
    let (mut consensus_to_quorum_store_sender, consensus_to_quorum_store_receiver) =
        mpsc::channel(1_024);
    let quorum_store = DirectMempoolQuorumStore::new(
        consensus_to_quorum_store_receiver,
        quorum_store_to_mempool_sender,
        10_000,
    );
    let join_handle = tokio::spawn(quorum_store.start());

    let (consensus_callback, consensus_callback_rcv) = oneshot::channel();
    consensus_to_quorum_store_sender
        .try_send(ConsensusRequest::GetBlockRequest(
            100,
            1000,
            PayloadFilter::DirectMempool(vec![]),
            consensus_callback,
        ))
        .unwrap();

    if let QuorumStoreRequest::GetBatchRequest(
        _max_batch_size,
        _max_bytes,
        _exclude_txns,
        callback,
    ) = timeout(
        Duration::from_millis(1_000),
        quorum_store_to_mempool_receiver.select_next_some(),
    )
    .await
    .unwrap()
    {
        callback
            .send(Ok(QuorumStoreResponse::GetBatchResponse(vec![])))
            .unwrap();
    } else {
        panic!("Unexpected variant")
    }

    match timeout(Duration::from_millis(1_000), consensus_callback_rcv)
        .await
        .unwrap()
        .unwrap()
        .unwrap()
    {
        ConsensusResponse::GetBlockResponse(payload) => {
            assert!(payload.is_empty());
            match payload {
                Payload::DirectMempool(txns) => assert!(txns.is_empty()),
                _ => panic!("Unexpected payload {:?}", payload),
            }
        }
        _ => {
            panic!("Unexpected variant")
        }
    }

    std::mem::drop(consensus_to_quorum_store_sender);
    timeout(Duration::from_millis(1_000), join_handle)
        .await
        .unwrap()
        .unwrap();
}
