// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network_interface::ConsensusMsg,
    quorum_store::{
        batch_requester::BatchRequester,
        tests::utils::{compute_digest_from_signed_transaction, create_vec_signed_transactions},
        types::BatchRequest,
    },
    test_utils::mock_quorum_store_sender::MockQuorumStoreSender,
};
use aptos_types::account_address::AccountAddress;
use claims::{assert_err, assert_some};
use tokio::sync::{mpsc::channel, oneshot};

#[tokio::test(flavor = "multi_thread")]
async fn test_batch_requester() {
    let (tx, mut rx) = channel(100);
    let sender = MockQuorumStoreSender::new(tx);
    let epoch = 1;
    let id = AccountAddress::random();
    let request_num_peers = 3;
    let request_timeout_ms = 0;
    let mut batch_requester =
        BatchRequester::new(epoch, id, request_num_peers, request_timeout_ms, sender);

    let signed_transactions = create_vec_signed_transactions(100);
    let digest = compute_digest_from_signed_transaction(signed_transactions.clone());
    let (oneshot_tx, oneshot_rx) = oneshot::channel();
    let mut signers = Vec::new();

    for _ in 1..10 {
        signers.push(AccountAddress::random());
    }

    batch_requester
        .add_request(digest, signers, oneshot_tx)
        .await;
    let res = rx.recv().await;
    assert_some!(res.clone());
    let (msg, signers) = res.unwrap();
    match msg {
        ConsensusMsg::BatchRequestMsg(request) => {
            assert_eq!(*request, BatchRequest::new(id, epoch, digest))
        },
        _ => unreachable!(),
    }
    assert_eq!(signers.len(), 3);

    batch_requester.serve_request(digest, signed_transactions.clone());
    assert_eq!(
        oneshot_rx.await.expect("sender dropped"),
        Ok(signed_transactions)
    );

    // test timeout logic
    let signed_transactions = create_vec_signed_transactions(200);
    let digest = compute_digest_from_signed_transaction(signed_transactions.clone());
    let (oneshot_tx, oneshot_rx) = oneshot::channel();
    batch_requester
        .add_request(digest, signers, oneshot_tx)
        .await;
    batch_requester.handle_timeouts().await;
    assert_some!(rx.recv().await);
    batch_requester.handle_timeouts().await;
    assert_some!(rx.recv().await);
    batch_requester.handle_timeouts().await;
    assert_some!(rx.recv().await);
    batch_requester.handle_timeouts().await;
    assert_some!(rx.recv().await);
    batch_requester.handle_timeouts().await;
    assert_err!(oneshot_rx.await.unwrap());
}
