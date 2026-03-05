// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::ChunkyDKGManager;
use crate::{
    chunky::{
        test_utils::{ChunkyTestSetup, DummyNetworkSender},
        types::{CertifiedAggregatedSubtranscript, MissingTranscriptRequest},
    },
    network::{DummyRpcResponseSender, IncomingRpcRequest},
    types::DKGMessage,
};
use aptos_bounded_executor::BoundedExecutor;
use aptos_crypto::SigningKey;
use aptos_infallible::RwLock;
use aptos_network::{
    application::{interface::NetworkClient, storage::PeersAndMetadata},
    protocols::network::Event,
};
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_time_service::TimeService;
use aptos_types::dkg::chunky_dkg::ChunkyDKGStartEvent;
use aptos_validator_transaction_pool::VTxnPoolState;
use futures_channel::oneshot;
use std::{sync::Arc, time::Duration};
use tokio::runtime::Handle;
use tokio_retry::strategy::ExponentialBackoff;

fn create_test_manager(setup: &ChunkyTestSetup) -> ChunkyDKGManager {
    let my_addr = setup.addrs[0];

    let reliable_broadcast = Arc::new(ReliableBroadcast::new(
        my_addr,
        setup.addrs.clone(),
        Arc::new(DummyNetworkSender),
        ExponentialBackoff::from_millis(10),
        TimeService::real(),
        Duration::from_millis(500),
        BoundedExecutor::new(2, Handle::current()),
    ));

    // Construct a minimal NetworkSender for testing.
    let peers_and_metadata = PeersAndMetadata::new(&[]);
    let network_client: NetworkClient<DKGMessage> =
        NetworkClient::new(vec![], vec![], Default::default(), peers_and_metadata);
    let dkg_network_client = crate::network_interface::DKGNetworkClient::new(network_client);
    let (self_sender, _self_receiver): (
        aptos_channels::Sender<Event<DKGMessage>>,
        aptos_channels::Receiver<Event<DKGMessage>>,
    ) = aptos_channels::new_test(1);
    let network_sender =
        Arc::new(crate::network::NetworkSender::new(my_addr, dkg_network_client, self_sender));

    ChunkyDKGManager::new_for_testing(
        setup.private_keys[0].clone(),
        Arc::new(setup.public_keys[0].clone()),
        0,
        my_addr,
        setup.epoch_state.clone(),
        VTxnPoolState::default(),
        reliable_broadcast,
        network_sender,
    )
}

fn new_chunky_transcript_rpc_request(
    epoch: u64,
    sender: AccountAddress,
    response_collector: Arc<RwLock<Vec<anyhow::Result<DKGMessage>>>>,
) -> IncomingRpcRequest {
    use crate::chunky::types::ChunkyDKGTranscriptRequest;
    IncomingRpcRequest {
        msg: DKGMessage::ChunkyTranscriptRequest(ChunkyDKGTranscriptRequest::new(epoch)),
        sender,
        response_sender: Box::new(DummyRpcResponseSender::new(response_collector)),
    }
}

fn new_missing_transcript_rpc_request(
    epoch: u64,
    sender: AccountAddress,
    missing_dealer: AccountAddress,
    response_collector: Arc<RwLock<Vec<anyhow::Result<DKGMessage>>>>,
) -> IncomingRpcRequest {
    IncomingRpcRequest {
        msg: DKGMessage::MissingTranscriptRequest(MissingTranscriptRequest::new(
            epoch,
            missing_dealer,
        )),
        sender,
        response_sender: Box::new(DummyRpcResponseSender::new(response_collector)),
    }
}

use move_core_types::account_address::AccountAddress;

#[tokio::test]
async fn test_chunky_dkg_state_transition() {
    let setup = ChunkyTestSetup::new_uniform(4);
    let mut manager = create_test_manager(&setup);

    // Initial state should be Init.
    assert_eq!(manager.state_name(), "NotStarted");

    // Transcript request should fail in Init state.
    let rpc_response_collector = Arc::new(RwLock::new(vec![]));
    let rpc_req = new_chunky_transcript_rpc_request(
        999,
        setup.addrs[3],
        rpc_response_collector.clone(),
    );
    let result = manager.process_peer_rpc_msg(rpc_req).await;
    assert!(result.is_err()); // transcript request fails in Init state

    // process_dkg_start_event should transition to AwaitSubtranscriptAggregation.
    let event = ChunkyDKGStartEvent {
        session_metadata: setup.session_metadata.clone(),
        start_time_us: Duration::from_secs(1700000000).as_micros() as u64,
    };
    let result = manager.process_dkg_start_event(event.clone()).await;
    assert!(result.is_ok());
    assert_eq!(manager.state_name(), "AwaitSubtranscriptAggregation");

    // Transcript request should succeed now.
    let rpc_req = new_chunky_transcript_rpc_request(
        999,
        setup.addrs[3],
        rpc_response_collector.clone(),
    );
    let result = manager.process_peer_rpc_msg(rpc_req).await;
    assert!(result.is_ok());
    let last_responses = std::mem::take(&mut *rpc_response_collector.write());
    assert!(last_responses.len() == 1 && last_responses[0].is_ok());

    // Duplicate start event should fail.
    let result = manager.process_dkg_start_event(event).await;
    assert!(result.is_err());

    // process_aggregated_subtranscript should transition to AwaitAggregatedSubtranscriptCertification.
    let agg_subtrx = setup.aggregate_subtranscripts(&[0, 1, 2]);
    let result = manager.process_aggregated_subtranscript(agg_subtrx.clone()).await;
    assert!(result.is_ok());
    assert_eq!(
        manager.state_name(),
        "AwaitAggregatedSubtranscriptCertification"
    );

    // process_certified_aggregated_subtranscript should transition to Finished.
    // Build a valid certified subtranscript.
    let mut sigs = std::collections::BTreeMap::new();
    for i in 0..3 {
        let sig = setup.private_keys[i].sign(&agg_subtrx).unwrap();
        sigs.insert(setup.addrs[i], sig);
    }
    let aggregate_signature = setup
        .epoch_state
        .verifier
        .aggregate_signatures(sigs.iter())
        .unwrap();
    let certified = CertifiedAggregatedSubtranscript {
        aggregated_subtranscript: agg_subtrx,
        aggregate_signature,
    };
    let result = manager
        .process_certified_aggregated_subtranscript(certified)
        .await;
    assert!(result.is_ok());
    assert_eq!(manager.state_name(), "Finished");
}

#[tokio::test]
async fn test_rpc_handling() {
    let setup = ChunkyTestSetup::new_uniform(4);
    let mut manager = create_test_manager(&setup);

    // Start DKG to get into a state where RPCs work.
    let event = ChunkyDKGStartEvent {
        session_metadata: setup.session_metadata.clone(),
        start_time_us: Duration::from_secs(1700000000).as_micros() as u64,
    };
    manager.process_dkg_start_event(event).await.unwrap();

    let rpc_response_collector = Arc::new(RwLock::new(vec![]));

    // Transcript request should return own transcript.
    let rpc_req = new_chunky_transcript_rpc_request(
        999,
        setup.addrs[1],
        rpc_response_collector.clone(),
    );
    manager.process_peer_rpc_msg(rpc_req).await.unwrap();
    let last_responses = std::mem::take(&mut *rpc_response_collector.write());
    assert_eq!(last_responses.len(), 1);
    assert!(last_responses[0].is_ok());
    match last_responses[0].as_ref().unwrap() {
        DKGMessage::ChunkyTranscriptResponse(trx) => {
            assert_eq!(trx.metadata.epoch, 999);
            assert_eq!(trx.metadata.author, setup.addrs[0]);
        },
        other => panic!("unexpected response: {:?}", other),
    }

    // Missing transcript request — transcript not stored yet, should return error.
    let rpc_req = new_missing_transcript_rpc_request(
        999,
        setup.addrs[1],
        setup.addrs[2],
        rpc_response_collector.clone(),
    );
    manager.process_peer_rpc_msg(rpc_req).await.unwrap();
    let last_responses = std::mem::take(&mut *rpc_response_collector.write());
    assert_eq!(last_responses.len(), 1);
    assert!(last_responses[0].is_err());

    // Wrong epoch RPC should return error.
    let rpc_req = new_chunky_transcript_rpc_request(
        1, // wrong epoch
        setup.addrs[1],
        rpc_response_collector.clone(),
    );
    let result = manager.process_peer_rpc_msg(rpc_req).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_close_and_notifications() {
    let setup = ChunkyTestSetup::new_uniform(4);
    let mut manager = create_test_manager(&setup);

    // Start DKG and advance to Finished state.
    let event = ChunkyDKGStartEvent {
        session_metadata: setup.session_metadata.clone(),
        start_time_us: Duration::from_secs(1700000000).as_micros() as u64,
    };
    manager.process_dkg_start_event(event).await.unwrap();

    let agg_subtrx = setup.aggregate_subtranscripts(&[0, 1, 2]);
    manager
        .process_aggregated_subtranscript(agg_subtrx.clone())
        .await
        .unwrap();

    let mut sigs = std::collections::BTreeMap::new();
    for i in 0..3 {
        let sig = setup.private_keys[i].sign(&agg_subtrx).unwrap();
        sigs.insert(setup.addrs[i], sig);
    }
    let aggregate_signature = setup
        .epoch_state
        .verifier
        .aggregate_signatures(sigs.iter())
        .unwrap();
    let certified = CertifiedAggregatedSubtranscript {
        aggregated_subtranscript: agg_subtrx,
        aggregate_signature,
    };
    manager
        .process_certified_aggregated_subtranscript(certified)
        .await
        .unwrap();
    assert_eq!(manager.state_name(), "Finished");

    // process_dkg_txn_pulled_notification should work in Finished state.
    let dummy_txn = Arc::new(aptos_types::validator_txn::ValidatorTransaction::DKGResult(
        aptos_types::dkg::DKGTranscript {
            metadata: aptos_types::dkg::DKGTranscriptMetadata {
                epoch: 999,
                author: setup.addrs[0],
            },
            transcript_bytes: vec![],
        },
    ));
    let result = manager
        .process_dkg_txn_pulled_notification(dummy_txn.clone())
        .await;
    assert!(result.is_ok());

    // process_close_cmd should set stopped=true.
    let (ack_tx, ack_rx) = oneshot::channel();
    let result = manager.process_close_cmd(Some(ack_tx));
    assert!(result.is_ok());
    assert!(manager.is_stopped());
    assert!(ack_rx.await.is_ok());

    // Pull notification in non-Finished state should fail.
    let mut manager2 = create_test_manager(&setup);
    let result = manager2
        .process_dkg_txn_pulled_notification(dummy_txn)
        .await;
    assert!(result.is_err());
}
