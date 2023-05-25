// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    moderator::RequestModerator,
    network::ResponseSender,
    storage::StorageReader,
    subscription,
    subscription::DataSubscriptionRequest,
    tests::{mock, utils},
};
use aptos_config::{config::StorageServiceConfig, network_id::PeerNetworkId};
use aptos_infallible::{Mutex, RwLock};
use aptos_network::protocols::wire::handshake::v1::ProtocolId;
use aptos_storage_service_types::{
    requests::{
        DataRequest, NewTransactionOutputsWithProofRequest,
        NewTransactionsOrOutputsWithProofRequest, NewTransactionsWithProofRequest,
        StorageServiceRequest,
    },
    responses::{CompleteDataRange, StorageServerSummary},
};
use aptos_time_service::TimeService;
use aptos_types::epoch_change::EpochChangeProof;
use futures::channel::oneshot;
use lru::LruCache;
use rand::{rngs::OsRng, Rng};
use std::{collections::HashMap, sync::Arc, time::Duration};

#[tokio::test]
async fn test_peers_with_ready_subscriptions() {
    // Create a mock time service
    let time_service = TimeService::mock();

    // Create two peers and data subscriptions
    let peer_network_1 = PeerNetworkId::random();
    let peer_network_2 = PeerNetworkId::random();
    let data_subscription_1 = create_subscription_request(time_service.clone(), Some(1), Some(1));
    let data_subscription_2 = create_subscription_request(time_service.clone(), Some(10), Some(1));

    // Insert the data subscriptions into the pending map
    let data_subscriptions = Arc::new(Mutex::new(HashMap::new()));
    data_subscriptions
        .lock()
        .insert(peer_network_1, data_subscription_1);
    data_subscriptions
        .lock()
        .insert(peer_network_2, data_subscription_2);

    // Create epoch ending test data
    let epoch_ending_ledger_info = utils::create_epoch_ending_ledger_info(1, 5);
    let epoch_change_proof = EpochChangeProof {
        ledger_info_with_sigs: vec![epoch_ending_ledger_info],
        more: false,
    };

    // Create the mock db reader
    let mut db_reader = mock::create_mock_db_reader();
    utils::expect_get_epoch_ending_ledger_infos(&mut db_reader, 1, 2, epoch_change_proof);

    // Create the storage reader
    let storage_reader = StorageReader::new(StorageServiceConfig::default(), Arc::new(db_reader));

    // Create test data with an empty storage server summary
    let cached_storage_server_summary = Arc::new(RwLock::new(StorageServerSummary::default()));
    let lru_response_cache = Arc::new(Mutex::new(LruCache::new(0)));
    let request_moderator = Arc::new(RequestModerator::new(
        cached_storage_server_summary.clone(),
        mock::create_peers_and_metadata(vec![]),
        StorageServiceConfig::default(),
        time_service.clone(),
    ));

    // Verify that there are no peers with ready subscriptions
    let peers_with_ready_subscriptions = subscription::get_peers_with_ready_subscriptions(
        cached_storage_server_summary.clone(),
        data_subscriptions.clone(),
        lru_response_cache.clone(),
        request_moderator.clone(),
        storage_reader.clone(),
        time_service.clone(),
    )
    .unwrap();
    assert!(peers_with_ready_subscriptions.is_empty());

    // Update the storage server summary so that there is new data for subscription 1
    let mut storage_server_summary = StorageServerSummary::default();
    storage_server_summary
        .data_summary
        .epoch_ending_ledger_infos = Some(CompleteDataRange::new(0, 1).unwrap());
    let synced_ledger_info = utils::create_test_ledger_info_with_sigs(1, 2);
    storage_server_summary.data_summary.synced_ledger_info = Some(synced_ledger_info.clone());
    *cached_storage_server_summary.write() = storage_server_summary;

    // Verify that subscription 1 is ready
    let peers_with_ready_subscriptions = subscription::get_peers_with_ready_subscriptions(
        cached_storage_server_summary.clone(),
        data_subscriptions.clone(),
        lru_response_cache.clone(),
        request_moderator.clone(),
        storage_reader.clone(),
        time_service.clone(),
    )
    .unwrap();
    assert_eq!(peers_with_ready_subscriptions, vec![(
        peer_network_1,
        synced_ledger_info
    )]);

    // Manually remove subscription 1 from the map
    data_subscriptions.lock().remove(&peer_network_1);

    // Update the storage server summary so that there is new data for subscription 2,
    // but the subscription is invalid because it doesn't respect an epoch boundary.
    let mut storage_server_summary = StorageServerSummary::default();
    storage_server_summary
        .data_summary
        .epoch_ending_ledger_infos = Some(CompleteDataRange::new(0, 2).unwrap());
    let synced_ledger_info = utils::create_test_ledger_info_with_sigs(2, 100);
    storage_server_summary.data_summary.synced_ledger_info = Some(synced_ledger_info);
    *cached_storage_server_summary.write() = storage_server_summary;

    // Verify that subscription 2 is not returned because it was invalid
    let peers_with_ready_subscriptions = subscription::get_peers_with_ready_subscriptions(
        cached_storage_server_summary,
        data_subscriptions,
        lru_response_cache,
        request_moderator,
        storage_reader,
        time_service,
    )
    .unwrap();
    assert_eq!(peers_with_ready_subscriptions, vec![]);

    // Verify that data subscriptions no longer contains peer 2
    assert!(peers_with_ready_subscriptions.is_empty());
}

#[tokio::test]
async fn test_remove_expired_subscriptions() {
    // Create a storage service config
    let max_subscription_period_ms = 100;
    let storage_service_config = StorageServiceConfig {
        max_subscription_period_ms,
        ..Default::default()
    };

    // Create a mock time service
    let time_service = TimeService::mock();

    // Create the first batch of test data subscriptions
    let num_subscriptions_in_batch = 10;
    let data_subscriptions = Arc::new(Mutex::new(HashMap::new()));
    for _ in 0..num_subscriptions_in_batch {
        let data_subscription = create_subscription_request(time_service.clone(), None, None);
        data_subscriptions
            .lock()
            .insert(PeerNetworkId::random(), data_subscription);
    }

    // Verify the number of active data subscriptions
    assert_eq!(data_subscriptions.lock().len(), num_subscriptions_in_batch);

    // Elapse a small amount of time (not enough to expire the subscriptions)
    time_service
        .clone()
        .into_mock()
        .advance_async(Duration::from_millis(max_subscription_period_ms / 2))
        .await;

    // Remove the expired subscriptions and verify none were removed
    subscription::remove_expired_data_subscriptions(
        storage_service_config,
        data_subscriptions.clone(),
    );
    assert_eq!(data_subscriptions.lock().len(), num_subscriptions_in_batch);

    // Create another batch of data subscriptions
    for _ in 0..num_subscriptions_in_batch {
        let data_subscription = create_subscription_request(time_service.clone(), None, None);
        data_subscriptions
            .lock()
            .insert(PeerNetworkId::random(), data_subscription);
    }

    // Verify the new number of active data subscriptions
    assert_eq!(
        data_subscriptions.lock().len(),
        num_subscriptions_in_batch * 2
    );

    // Elapse enough time to expire the first batch of subscriptions
    time_service
        .clone()
        .into_mock()
        .advance_async(Duration::from_millis(max_subscription_period_ms))
        .await;

    // Remove the expired subscriptions and verify the first batch was removed
    subscription::remove_expired_data_subscriptions(
        storage_service_config,
        data_subscriptions.clone(),
    );
    assert_eq!(data_subscriptions.lock().len(), num_subscriptions_in_batch);

    // Elapse enough time to expire the second batch of subscriptions
    time_service
        .into_mock()
        .advance_async(Duration::from_millis(max_subscription_period_ms))
        .await;

    // Remove the expired subscriptions and verify the second batch was removed
    subscription::remove_expired_data_subscriptions(
        storage_service_config,
        data_subscriptions.clone(),
    );
    assert!(data_subscriptions.lock().is_empty());
}

/// Creates a random data request for subscription data
fn create_subscription_data_request(
    known_version: Option<u64>,
    known_epoch: Option<u64>,
) -> DataRequest {
    let known_version = known_version.unwrap_or_default();
    let known_epoch = known_epoch.unwrap_or_default();

    // Generate the random data request
    let mut rng = OsRng;
    let random_number: u8 = rng.gen();
    match random_number % 3 {
        0 => DataRequest::GetNewTransactionsWithProof(NewTransactionsWithProofRequest {
            known_version,
            known_epoch,
            include_events: false,
        }),
        1 => {
            DataRequest::GetNewTransactionOutputsWithProof(NewTransactionOutputsWithProofRequest {
                known_version,
                known_epoch,
            })
        },
        2 => DataRequest::GetNewTransactionsOrOutputsWithProof(
            NewTransactionsOrOutputsWithProofRequest {
                known_version,
                known_epoch,
                include_events: true,
                max_num_output_reductions: 1,
            },
        ),
        num => panic!("This shouldn't be possible! Got num: {:?}", num),
    }
}

/// Creates a random data subscription request
fn create_subscription_request(
    time_service: TimeService,
    known_version: Option<u64>,
    known_epoch: Option<u64>,
) -> DataSubscriptionRequest {
    // Create a storage service request
    let data_request = create_subscription_data_request(known_version, known_epoch);
    let storage_service_request = StorageServiceRequest::new(data_request, true);

    // Create the response sender
    let (callback, _) = oneshot::channel();
    let response_sender = ResponseSender::new(callback);

    // Create and return the data subscription request
    DataSubscriptionRequest::new(
        ProtocolId::StorageServiceRpc,
        storage_service_request,
        response_sender,
        time_service,
    )
}
