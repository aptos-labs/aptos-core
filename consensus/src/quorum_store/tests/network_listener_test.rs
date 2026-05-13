// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    quorum_store::{
        batch_coordinator::{BatchCoordinatorCommand, BatchCoordinatorQueueKey},
        counters,
        network_listener::NetworkListener,
        proof_coordinator::ProofCoordinatorCommand,
        proof_manager::ProofManagerCommand,
        types::{Batch, BatchMsg},
    },
    round_manager::VerifiedEvent,
    test_utils::create_vec_signed_transactions,
};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_consensus_types::proof_of_store::{SignedBatchInfo, SignedBatchInfoMsg};
use aptos_types::{quorum_store::BatchId, validator_verifier::random_validator_verifier};
use futures::StreamExt;
use tokio::{
    sync::mpsc::channel,
    time::{timeout, Duration},
};

#[tokio::test(flavor = "multi_thread")]
async fn test_full_batch_queue_does_not_block_following_messages() {
    aptos_logger::Logger::init_for_testing();

    let (signers, _verifier) = random_validator_verifier(4, None, true);
    let batch_author = signers[0].author();

    let batch = Batch::new_v1(
        BatchId::new_for_test(1),
        create_vec_signed_transactions(1),
        1,
        10,
        batch_author,
        0,
    );
    let signed_batch_info = SignedBatchInfo::dummy(batch.batch_info().clone(), signers[1].author());

    let (network_msg_tx, network_msg_rx) =
        aptos_channel::new::<_, (_, VerifiedEvent)>(QueueStyle::FIFO, 10, None);
    let (proof_coordinator_tx, mut proof_coordinator_rx) = channel(10);
    let (proof_manager_tx, _proof_manager_rx) = channel::<ProofManagerCommand>(10);
    let (batch_coordinator_tx, mut batch_coordinator_rx) =
        aptos_channel::new(QueueStyle::FIFO, 1, None);
    let dropped_count = counters::QUORUM_STORE_MSG_COUNT
        .with_label_values(&["NetworkListener::batchmsg_queue_full"])
        .get();

    batch_coordinator_tx
        .push(
            BatchCoordinatorQueueKey::Author(batch_author),
            BatchCoordinatorCommand::NewBatches(batch_author, vec![batch.clone()]),
        )
        .unwrap();

    network_msg_tx
        .push(
            batch_author,
            (
                batch_author,
                VerifiedEvent::BatchMsg(Box::new(BatchMsg::new(vec![batch]))),
            ),
        )
        .unwrap();
    network_msg_tx
        .push(
            signers[1].author(),
            (
                signers[1].author(),
                VerifiedEvent::SignedBatchInfo(Box::new(SignedBatchInfoMsg::new(vec![
                    signed_batch_info,
                ]))),
            ),
        )
        .unwrap();

    let listener = NetworkListener::new(
        network_msg_rx,
        proof_coordinator_tx,
        vec![batch_coordinator_tx],
        proof_manager_tx,
    );
    let listener_handle = tokio::spawn(listener.start());

    let forwarded = timeout(Duration::from_secs(1), proof_coordinator_rx.recv())
        .await
        .expect("timed out waiting for signed batch info")
        .expect("proof coordinator channel dropped");

    match forwarded {
        ProofCoordinatorCommand::AppendSignature(sender, signed_batch_infos) => {
            assert_eq!(sender, signers[1].author());
            assert_eq!(signed_batch_infos.take().len(), 1);
        },
        msg => panic!("unexpected proof coordinator command: {:?}", msg),
    }

    let queued = timeout(Duration::from_secs(1), batch_coordinator_rx.next())
        .await
        .expect("timed out waiting for queued batch")
        .expect("batch coordinator queue should retain only the pre-filled entry");
    match queued {
        BatchCoordinatorCommand::NewBatches(author, batches) => {
            assert_eq!(author, batch_author);
            assert_eq!(batches.len(), 1);
        },
        msg => panic!("unexpected batch coordinator command: {:?}", msg),
    }
    assert!(
        timeout(Duration::from_millis(50), batch_coordinator_rx.next())
            .await
            .is_err()
    );
    assert_eq!(
        counters::QUORUM_STORE_MSG_COUNT
            .with_label_values(&["NetworkListener::batchmsg_queue_full"])
            .get(),
        dropped_count + 1
    );

    listener_handle.abort();
    let _ = listener_handle.await;
}
