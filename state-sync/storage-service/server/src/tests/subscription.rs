// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    moderator::RequestModerator,
    network::ResponseSender,
    storage::StorageReader,
    subscription,
    subscription::{SubscriptionRequest, SubscriptionStreamRequests},
    tests::{mock, mock::MockClient, utils},
};
use aptos_config::{
    config::{AptosDataClientConfig, StorageServiceConfig},
    network_id::PeerNetworkId,
};
use aptos_storage_service_types::{
    requests::{
        DataRequest, StorageServiceRequest, SubscribeTransactionDataWithProofRequest,
        SubscribeTransactionOutputsWithProofRequest,
        SubscribeTransactionsOrOutputsWithProofRequest, SubscribeTransactionsWithProofRequest,
        SubscriptionStreamMetadata, TransactionData, TransactionDataRequestType,
        TransactionOrOutputData,
    },
    responses::StorageServerSummary,
    StorageServiceError,
};
use aptos_time_service::TimeService;
use aptos_types::epoch_change::EpochChangeProof;
use arc_swap::ArcSwap;
use claims::assert_matches;
use dashmap::DashMap;
use futures::channel::oneshot;
use mini_moka::sync::Cache;
use std::sync::Arc;
use tokio::runtime::Handle;

#[tokio::test]
async fn test_peers_with_ready_subscriptions() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Create a mock time service and subscriptions map
        let time_service = TimeService::mock();
        let subscriptions = Arc::new(DashMap::new());

        // Create three peers with ready subscriptions
        let mut peer_network_ids = vec![];
        for known_version in &[1, 5, 10] {
            // Create a random peer network id
            let peer_network_id = PeerNetworkId::random();
            peer_network_ids.push(peer_network_id);

            // Create a subscription stream and insert it into the pending map
            let subscription_stream_requests = create_subscription_stream_requests(
                time_service.clone(),
                Some(*known_version),
                Some(1),
                Some(0),
                Some(0),
                use_request_v2,
            );
            subscriptions.insert(peer_network_id, subscription_stream_requests);
        }

        // Create epoch ending test data at version 9
        let epoch_ending_ledger_info = utils::create_epoch_ending_ledger_info(1, 9);
        let epoch_change_proof = EpochChangeProof {
            ledger_info_with_sigs: vec![epoch_ending_ledger_info],
            more: false,
        };

        // Create the mock db reader
        let mut db_reader = mock::create_mock_db_reader();
        utils::expect_get_epoch_ending_ledger_infos(&mut db_reader, 1, 2, epoch_change_proof);

        // Create the storage reader
        let storage_service_config = StorageServiceConfig::default();
        let storage_reader = StorageReader::new(storage_service_config, Arc::new(db_reader));

        // Create test data with an empty storage server summary
        let cached_storage_server_summary =
            Arc::new(ArcSwap::from(Arc::new(StorageServerSummary::default())));
        let optimistic_fetches = Arc::new(DashMap::new());
        let lru_response_cache = Cache::new(0);
        let request_moderator = Arc::new(RequestModerator::new(
            AptosDataClientConfig::default(),
            cached_storage_server_summary.clone(),
            mock::create_peers_and_metadata(vec![]),
            StorageServiceConfig::default(),
            time_service.clone(),
        ));

        // Verify that there are no peers with ready subscriptions
        let peers_with_ready_subscriptions = subscription::get_peers_with_ready_subscriptions(
            Handle::current(),
            storage_service_config,
            cached_storage_server_summary.clone(),
            optimistic_fetches.clone(),
            lru_response_cache.clone(),
            request_moderator.clone(),
            storage_reader.clone(),
            subscriptions.clone(),
            time_service.clone(),
        )
        .await
        .unwrap();
        assert!(peers_with_ready_subscriptions.is_empty());

        // Update the storage server summary so that there is new data (at version 2)
        let highest_synced_ledger_info =
            utils::update_storage_summary_cache(cached_storage_server_summary.clone(), 2, 1);

        // Verify that peer 1 has a ready subscription
        let peers_with_ready_subscriptions = subscription::get_peers_with_ready_subscriptions(
            Handle::current(),
            storage_service_config,
            cached_storage_server_summary.clone(),
            optimistic_fetches.clone(),
            lru_response_cache.clone(),
            request_moderator.clone(),
            storage_reader.clone(),
            subscriptions.clone(),
            time_service.clone(),
        )
        .await
        .unwrap();
        assert_eq!(peers_with_ready_subscriptions, vec![(
            peer_network_ids[0],
            highest_synced_ledger_info
        )]);

        // Manually remove subscription 1 from the map
        subscriptions.remove(&peer_network_ids[0]);

        // Update the storage server summary so that there is new data (at version 8)
        let highest_synced_ledger_info =
            utils::update_storage_summary_cache(cached_storage_server_summary.clone(), 8, 1);

        // Verify that peer 2 has a ready subscription
        let peers_with_ready_subscriptions = subscription::get_peers_with_ready_subscriptions(
            Handle::current(),
            storage_service_config,
            cached_storage_server_summary.clone(),
            optimistic_fetches.clone(),
            lru_response_cache.clone(),
            request_moderator.clone(),
            storage_reader.clone(),
            subscriptions.clone(),
            time_service.clone(),
        )
        .await
        .unwrap();
        assert_eq!(peers_with_ready_subscriptions, vec![(
            peer_network_ids[1],
            highest_synced_ledger_info
        )]);

        // Manually remove subscription 2 from the map
        subscriptions.remove(&peer_network_ids[1]);

        // Update the storage server summary so that there is new data (at version 100)
        let _ = utils::update_storage_summary_cache(cached_storage_server_summary.clone(), 100, 2);

        // Verify that subscription 3 is not returned because it was invalid
        // (i.e., the epoch ended at version 9, but the peer didn't respect it).
        let peers_with_ready_subscriptions = subscription::get_peers_with_ready_subscriptions(
            Handle::current(),
            storage_service_config,
            cached_storage_server_summary.clone(),
            optimistic_fetches.clone(),
            lru_response_cache.clone(),
            request_moderator.clone(),
            storage_reader.clone(),
            subscriptions.clone(),
            time_service.clone(),
        )
        .await
        .unwrap();
        assert_eq!(peers_with_ready_subscriptions, vec![]);

        // Verify that the subscriptions are now empty
        assert!(subscriptions.is_empty());
    }
}

#[tokio::test]
async fn test_remove_expired_subscriptions_no_new_data() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Create a storage service config
        let max_subscription_period_ms = 100;
        let storage_service_config = StorageServiceConfig {
            max_subscription_period_ms,
            ..Default::default()
        };

        // Create the mock storage reader and time service
        let db_reader = mock::create_mock_db_reader();
        let storage_reader = StorageReader::new(storage_service_config, Arc::new(db_reader));
        let time_service = TimeService::mock();

        // Create test data with an empty storage server summary
        let cached_storage_server_summary =
            Arc::new(ArcSwap::from(Arc::new(StorageServerSummary::default())));
        let optimistic_fetches = Arc::new(DashMap::new());
        let lru_response_cache = Cache::new(0);
        let request_moderator = Arc::new(RequestModerator::new(
            AptosDataClientConfig::default(),
            cached_storage_server_summary.clone(),
            mock::create_peers_and_metadata(vec![]),
            StorageServiceConfig::default(),
            time_service.clone(),
        ));

        // Create the first batch of test subscriptions
        let num_subscriptions_in_batch = 10;
        let subscriptions = Arc::new(DashMap::new());
        for _ in 0..num_subscriptions_in_batch {
            let subscription_stream_requests = create_subscription_stream_requests(
                time_service.clone(),
                Some(9),
                Some(9),
                None,
                None,
                use_request_v2,
            );
            subscriptions.insert(PeerNetworkId::random(), subscription_stream_requests);
        }

        // Verify the number of active subscriptions
        assert_eq!(subscriptions.len(), num_subscriptions_in_batch);

        // Elapse a small amount of time (not enough to expire the subscriptions)
        utils::elapse_time(max_subscription_period_ms / 2, &time_service).await;

        // Update the storage server summary so that there is new data
        let _ = utils::update_storage_summary_cache(cached_storage_server_summary.clone(), 1, 1);

        // Remove the expired subscriptions and verify none were removed
        let peers_with_ready_subscriptions = subscription::get_peers_with_ready_subscriptions(
            Handle::current(),
            storage_service_config,
            cached_storage_server_summary.clone(),
            optimistic_fetches.clone(),
            lru_response_cache.clone(),
            request_moderator.clone(),
            storage_reader.clone(),
            subscriptions.clone(),
            time_service.clone(),
        )
        .await
        .unwrap();
        assert!(peers_with_ready_subscriptions.is_empty());
        assert_eq!(subscriptions.len(), num_subscriptions_in_batch);

        // Create another batch of test subscriptions
        for _ in 0..num_subscriptions_in_batch {
            let subscription_stream_requests = create_subscription_stream_requests(
                time_service.clone(),
                Some(9),
                Some(9),
                None,
                None,
                use_request_v2,
            );
            subscriptions.insert(PeerNetworkId::random(), subscription_stream_requests);
        }

        // Verify the new number of active subscriptions
        assert_eq!(subscriptions.len(), num_subscriptions_in_batch * 2);

        // Elapse enough time to expire the first batch of subscriptions
        utils::elapse_time(max_subscription_period_ms, &time_service).await;

        // Remove the expired subscriptions and verify the first batch was removed
        let peers_with_ready_subscriptions = subscription::get_peers_with_ready_subscriptions(
            Handle::current(),
            storage_service_config,
            cached_storage_server_summary.clone(),
            optimistic_fetches.clone(),
            lru_response_cache.clone(),
            request_moderator.clone(),
            storage_reader.clone(),
            subscriptions.clone(),
            time_service.clone(),
        )
        .await
        .unwrap();
        assert!(peers_with_ready_subscriptions.is_empty());
        assert_eq!(subscriptions.len(), num_subscriptions_in_batch);

        // Elapse enough time to expire the second batch of subscriptions
        utils::elapse_time(max_subscription_period_ms, &time_service).await;

        // Remove the expired subscriptions and verify the second batch was removed
        let peers_with_ready_subscriptions = subscription::get_peers_with_ready_subscriptions(
            Handle::current(),
            storage_service_config,
            cached_storage_server_summary.clone(),
            optimistic_fetches.clone(),
            lru_response_cache.clone(),
            request_moderator.clone(),
            storage_reader.clone(),
            subscriptions.clone(),
            time_service.clone(),
        )
        .await
        .unwrap();
        assert!(peers_with_ready_subscriptions.is_empty());
        assert!(subscriptions.is_empty());
    }
}

#[tokio::test]
async fn test_remove_expired_subscriptions_blocked_stream() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Create a storage service config
        let max_subscription_period_ms = 100;
        let storage_service_config = StorageServiceConfig {
            max_subscription_period_ms,
            ..Default::default()
        };

        // Create a mock time service
        let time_service = TimeService::mock();

        // Create a batch of test subscriptions
        let num_subscriptions_in_batch = 10;
        let subscriptions = Arc::new(DashMap::new());
        let mut peer_network_ids = vec![];
        for i in 0..num_subscriptions_in_batch {
            // Create a new peer
            let peer_network_id = PeerNetworkId::random();
            peer_network_ids.push(peer_network_id);

            // Create a subscription stream request for the peer
            let subscription_stream_requests = create_subscription_stream_requests(
                time_service.clone(),
                Some(1),
                Some(1),
                Some(i as u64),
                Some(0),
                use_request_v2,
            );
            subscriptions.insert(peer_network_id, subscription_stream_requests);
        }

        // Create test data with an empty storage server summary
        let cached_storage_server_summary =
            Arc::new(ArcSwap::from(Arc::new(StorageServerSummary::default())));
        let optimistic_fetches = Arc::new(DashMap::new());
        let lru_response_cache = Cache::new(0);
        let request_moderator = Arc::new(RequestModerator::new(
            AptosDataClientConfig::default(),
            cached_storage_server_summary.clone(),
            mock::create_peers_and_metadata(vec![]),
            StorageServiceConfig::default(),
            time_service.clone(),
        ));
        let storage_reader = StorageReader::new(
            storage_service_config,
            Arc::new(mock::create_mock_db_reader()),
        );

        // Update the storage server summary so that there is new data (at version 5)
        let _ = utils::update_storage_summary_cache(cached_storage_server_summary.clone(), 5, 1);

        // Handle the active subscriptions
        subscription::handle_active_subscriptions(
            Handle::current(),
            cached_storage_server_summary.clone(),
            storage_service_config,
            optimistic_fetches.clone(),
            lru_response_cache.clone(),
            request_moderator.clone(),
            storage_reader.clone(),
            subscriptions.clone(),
            time_service.clone(),
        )
        .await
        .unwrap();

        // Verify that all subscription streams are now empty because
        // the pending requests were sent.
        assert_eq!(subscriptions.len(), num_subscriptions_in_batch);
        for subscription in subscriptions.iter() {
            assert!(subscription.value().first_pending_request().is_none());
        }

        // Elapse enough time to expire the blocked streams
        utils::elapse_time(max_subscription_period_ms + 1, &time_service).await;

        // Add a new subscription request to the first subscription stream
        let subscription_request = create_subscription_request(
            &time_service,
            Some(1),
            Some(1),
            Some(0),
            Some(1),
            use_request_v2,
        );
        add_subscription_request_to_stream(
            subscription_request,
            subscriptions.clone(),
            &peer_network_ids[0],
        )
        .unwrap();

        // Remove the expired subscriptions and verify the second batch was removed
        let peers_with_ready_subscriptions = subscription::get_peers_with_ready_subscriptions(
            Handle::current(),
            storage_service_config,
            cached_storage_server_summary.clone(),
            optimistic_fetches.clone(),
            lru_response_cache.clone(),
            request_moderator.clone(),
            storage_reader.clone(),
            subscriptions.clone(),
            time_service.clone(),
        )
        .await
        .unwrap();
        assert!(peers_with_ready_subscriptions.is_empty());
        assert_eq!(subscriptions.len(), 1);
        assert!(subscriptions.contains_key(&peer_network_ids[0]));
    }
}

#[tokio::test]
async fn test_remove_expired_subscriptions_blocked_stream_index() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Create a storage service config
        let max_subscription_period_ms = 100;
        let storage_service_config = StorageServiceConfig {
            max_subscription_period_ms,
            ..Default::default()
        };

        // Create a mock time service
        let time_service = TimeService::mock();

        // Create the first batch of test subscriptions
        let num_subscriptions_in_batch = 10;
        let subscriptions = Arc::new(DashMap::new());
        for _ in 0..num_subscriptions_in_batch {
            let subscription_stream_requests = create_subscription_stream_requests(
                time_service.clone(),
                Some(1),
                Some(1),
                None,
                Some(0),
                use_request_v2,
            );
            subscriptions.insert(PeerNetworkId::random(), subscription_stream_requests);
        }

        // Create test data with an empty storage server summary
        let cached_storage_server_summary =
            Arc::new(ArcSwap::from(Arc::new(StorageServerSummary::default())));
        let optimistic_fetches = Arc::new(DashMap::new());
        let lru_response_cache = Cache::new(0);
        let request_moderator = Arc::new(RequestModerator::new(
            AptosDataClientConfig::default(),
            cached_storage_server_summary.clone(),
            mock::create_peers_and_metadata(vec![]),
            StorageServiceConfig::default(),
            time_service.clone(),
        ));
        let storage_reader = StorageReader::new(
            storage_service_config,
            Arc::new(mock::create_mock_db_reader()),
        );

        // Update the storage server summary so that there is new data (at version 5)
        let highest_synced_ledger_info =
            utils::update_storage_summary_cache(cached_storage_server_summary.clone(), 5, 1);

        // Verify that all peers have ready subscriptions (but don't serve them!)
        let peers_with_ready_subscriptions = subscription::get_peers_with_ready_subscriptions(
            Handle::current(),
            storage_service_config,
            cached_storage_server_summary.clone(),
            optimistic_fetches.clone(),
            lru_response_cache.clone(),
            request_moderator.clone(),
            storage_reader.clone(),
            subscriptions.clone(),
            time_service.clone(),
        )
        .await
        .unwrap();
        assert_eq!(
            peers_with_ready_subscriptions.len(),
            num_subscriptions_in_batch
        );

        // Elapse enough time to expire the subscriptions
        utils::elapse_time(max_subscription_period_ms + 1, &time_service).await;

        // Remove the expired subscriptions and verify they were all removed
        let peers_with_ready_subscriptions = subscription::get_peers_with_ready_subscriptions(
            Handle::current(),
            storage_service_config,
            cached_storage_server_summary.clone(),
            optimistic_fetches.clone(),
            lru_response_cache.clone(),
            request_moderator.clone(),
            storage_reader.clone(),
            subscriptions.clone(),
            time_service.clone(),
        )
        .await
        .unwrap();
        assert!(peers_with_ready_subscriptions.is_empty());
        assert!(subscriptions.is_empty());

        // Create another batch of test subscriptions (where the stream is
        // blocked on the next index to serve).
        let mut peer_network_ids = vec![];
        for i in 0..num_subscriptions_in_batch {
            // Create a new peer
            let peer_network_id = PeerNetworkId::random();
            peer_network_ids.push(peer_network_id);

            // Create a subscription stream request for the peer
            let subscription_stream_requests = create_subscription_stream_requests(
                time_service.clone(),
                Some(1),
                Some(1),
                None,
                Some(i as u64 + 1),
                use_request_v2,
            );
            subscriptions.insert(peer_network_id, subscription_stream_requests);
        }

        // Verify the number of active subscriptions
        assert_eq!(subscriptions.len(), num_subscriptions_in_batch);

        // Verify that none of the subscriptions are ready to be served (they are blocked)
        let peers_with_ready_subscriptions = subscription::get_peers_with_ready_subscriptions(
            Handle::current(),
            storage_service_config,
            cached_storage_server_summary.clone(),
            optimistic_fetches.clone(),
            lru_response_cache.clone(),
            request_moderator.clone(),
            storage_reader.clone(),
            subscriptions.clone(),
            time_service.clone(),
        )
        .await
        .unwrap();
        assert!(peers_with_ready_subscriptions.is_empty());

        // Elapse enough time to expire the batch of subscriptions
        utils::elapse_time(max_subscription_period_ms + 1, &time_service).await;

        // Add a new subscription request to the first subscription stream (to unblock it)
        let subscription_request = create_subscription_request(
            &time_service,
            Some(1),
            Some(1),
            None,
            Some(0),
            use_request_v2,
        );
        add_subscription_request_to_stream(
            subscription_request,
            subscriptions.clone(),
            &peer_network_ids[0],
        )
        .unwrap();

        // Verify that the first peer subscription stream is unblocked
        let peers_with_ready_subscriptions = subscription::get_peers_with_ready_subscriptions(
            Handle::current(),
            storage_service_config,
            cached_storage_server_summary.clone(),
            optimistic_fetches.clone(),
            lru_response_cache.clone(),
            request_moderator.clone(),
            storage_reader.clone(),
            subscriptions.clone(),
            time_service.clone(),
        )
        .await
        .unwrap();
        assert_eq!(peers_with_ready_subscriptions.len(), 1);
        assert!(peers_with_ready_subscriptions
            .contains(&(peer_network_ids[0], highest_synced_ledger_info)));

        // Remove the expired subscriptions and verify all but one were removed
        let _ = subscription::get_peers_with_ready_subscriptions(
            Handle::current(),
            storage_service_config,
            cached_storage_server_summary.clone(),
            optimistic_fetches.clone(),
            lru_response_cache.clone(),
            request_moderator.clone(),
            storage_reader.clone(),
            subscriptions.clone(),
            time_service.clone(),
        )
        .await
        .unwrap();
        assert_eq!(subscriptions.len(), 1);
        assert!(subscriptions.contains_key(&peer_network_ids[0]));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_subscription_invalid_requests() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Create a mock time service
        let time_service = TimeService::mock();

        // Create a new batch of subscriptions that includes a single stream and request
        let subscriptions = Arc::new(DashMap::new());
        let peer_network_id = PeerNetworkId::random();
        let peer_known_version = 10;
        let peer_known_epoch = 1;
        let subscription_stream_id = utils::get_random_u64();
        let subscription_stream_requests = create_subscription_stream_requests(
            time_service.clone(),
            Some(peer_known_version),
            Some(peer_known_epoch),
            Some(subscription_stream_id),
            Some(0),
            use_request_v2,
        );
        subscriptions.insert(peer_network_id, subscription_stream_requests);

        // Add a request to the stream that is invalid (the stream id is incorrect)
        let subscription_request = create_subscription_request(
            &time_service,
            Some(peer_known_version),
            Some(peer_known_epoch),
            Some(subscription_stream_id + 1),
            Some(1),
            use_request_v2,
        );
        let (error, _) = add_subscription_request_to_stream(
            subscription_request,
            subscriptions.clone(),
            &peer_network_id,
        )
        .unwrap_err();
        assert_matches!(error, Error::InvalidRequest(_));

        // Add a request to the stream that is invalid (the known version is incorrect)
        let subscription_request = create_subscription_request(
            &time_service,
            Some(peer_known_version + 1),
            Some(peer_known_epoch),
            Some(subscription_stream_id),
            Some(1),
            use_request_v2,
        );
        let (error, _) = add_subscription_request_to_stream(
            subscription_request,
            subscriptions.clone(),
            &peer_network_id,
        )
        .unwrap_err();
        assert_matches!(error, Error::InvalidRequest(_));

        // Add a request to the stream that is invalid (the known epoch is incorrect)
        let subscription_request = create_subscription_request(
            &time_service,
            Some(peer_known_version),
            Some(peer_known_epoch + 1),
            Some(subscription_stream_id),
            Some(1),
            use_request_v2,
        );
        let (error, _) = add_subscription_request_to_stream(
            subscription_request,
            subscriptions.clone(),
            &peer_network_id,
        )
        .unwrap_err();
        assert_matches!(error, Error::InvalidRequest(_));

        // Update the next index to serve for the stream
        let next_index_to_serve = 10;
        let mut subscription = subscriptions.get_mut(&peer_network_id).unwrap();
        let subscription_stream_requests = subscription.value_mut();
        subscription_stream_requests.set_next_index_to_serve(next_index_to_serve);
        drop(subscription);

        // Add a request to the stream that is invalid (the stream index is less than the next index to serve)
        let subscription_request = create_subscription_request(
            &time_service,
            Some(peer_known_version),
            Some(peer_known_epoch),
            Some(subscription_stream_id),
            Some(next_index_to_serve - 1),
            use_request_v2,
        );
        let (error, _) = add_subscription_request_to_stream(
            subscription_request,
            subscriptions.clone(),
            &peer_network_id,
        )
        .unwrap_err();
        assert_matches!(error, Error::InvalidRequest(_));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_subscription_max_pending_requests() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Create a storage service config
        let max_transaction_output_chunk_size = 5;
        let max_num_active_subscriptions = 10;
        let storage_service_config = StorageServiceConfig {
            max_num_active_subscriptions,
            max_transaction_output_chunk_size,
            enable_transaction_data_v2: use_request_v2,
            ..Default::default()
        };

        // Create test data
        let num_stream_requests = max_num_active_subscriptions * 10; // Send more requests than allowed
        let highest_version = 45576;
        let highest_epoch = 43;
        let lowest_version = 0;
        let peer_version = 50;
        let highest_ledger_info =
            utils::create_test_ledger_info_with_sigs(highest_epoch, highest_version);

        // Create the output lists with proofs
        let output_lists_with_proofs: Vec<_> = (0..num_stream_requests)
            .map(|i| {
                let start_version = peer_version + (i * max_transaction_output_chunk_size) + 1;
                let end_version = start_version + max_transaction_output_chunk_size - 1;
                utils::create_output_list_with_proof(
                    start_version,
                    end_version,
                    highest_version,
                    use_request_v2,
                )
            })
            .collect();

        // Create the mock db reader
        let mut db_reader =
            mock::create_mock_db_with_summary_updates(highest_ledger_info.clone(), lowest_version);
        for stream_request_index in 0..num_stream_requests {
            utils::expect_get_transaction_outputs(
                &mut db_reader,
                peer_version + (stream_request_index * max_transaction_output_chunk_size) + 1,
                max_transaction_output_chunk_size,
                highest_version,
                output_lists_with_proofs[stream_request_index as usize].clone(),
            );
        }

        // Create the storage client and server
        let (mut mock_client, service, storage_service_notifier, mock_time, _) =
            MockClient::new(Some(db_reader), Some(storage_service_config));
        let active_subscriptions = service.get_subscriptions();
        tokio::spawn(service.start());

        // Send the maximum number of stream requests
        let peer_network_id = PeerNetworkId::random();
        let stream_id = 101;
        let mut response_receivers = utils::send_output_subscription_request_batch(
            &mut mock_client,
            peer_network_id,
            0,
            max_num_active_subscriptions - 1,
            stream_id,
            peer_version,
            highest_epoch,
            use_request_v2,
        )
        .await;

        // Wait until the maximum number of stream requests are active
        utils::wait_for_active_stream_requests(
            active_subscriptions.clone(),
            peer_network_id,
            max_num_active_subscriptions as usize,
        )
        .await;

        // Send another batch of stream requests (to exceed the maximum number of
        // subscriptions), and verify that the client receives a failure for each request.
        for stream_request_index in max_num_active_subscriptions..max_num_active_subscriptions * 2 {
            // Send the transaction output subscription request
            let response_receiver = utils::subscribe_to_transaction_outputs_for_peer(
                &mut mock_client,
                peer_version,
                highest_epoch,
                stream_id,
                stream_request_index,
                Some(peer_network_id),
                use_request_v2,
            )
            .await;

            // Verify that the client receives an invalid request error
            let response = mock_client
                .wait_for_response(response_receiver)
                .await
                .unwrap_err();
            assert!(matches!(response, StorageServiceError::InvalidRequest(_)));
        }

        // Verify the request indices that are pending
        verify_pending_subscription_request_indices(
            active_subscriptions.clone(),
            peer_network_id,
            0,
            max_num_active_subscriptions,
            num_stream_requests,
        );

        // Force the subscription handler to work
        utils::force_subscription_handler_to_run(
            &mut mock_client,
            &mock_time,
            &storage_service_notifier,
        )
        .await;

        // Continuously run the subscription service until all of the responses are sent
        for stream_request_index in 0..max_num_active_subscriptions {
            // Verify that the correct response is received
            utils::verify_output_subscription_response(
                output_lists_with_proofs.clone(),
                highest_ledger_info.clone(),
                &mut mock_client,
                &mut response_receivers,
                stream_request_index,
                use_request_v2,
            )
            .await;
        }

        // Send another batch of requests for transaction outputs
        let _response_receivers = utils::send_output_subscription_request_batch(
            &mut mock_client,
            peer_network_id,
            max_num_active_subscriptions,
            (max_num_active_subscriptions * 2) - 1,
            stream_id,
            peer_version,
            highest_epoch,
            use_request_v2,
        )
        .await;

        // Wait until the maximum number of stream requests are active
        utils::wait_for_active_stream_requests(
            active_subscriptions.clone(),
            peer_network_id,
            max_num_active_subscriptions as usize,
        )
        .await;

        // Send another batch of stream requests (to exceed the maximum number of
        // subscriptions), and verify that the client receives a failure for each request.
        for stream_request_index in
            max_num_active_subscriptions * 2..max_num_active_subscriptions * 3
        {
            // Send the transaction output subscription request
            let response_receiver = utils::subscribe_to_transaction_outputs_for_peer(
                &mut mock_client,
                peer_version,
                highest_epoch,
                stream_id,
                stream_request_index,
                Some(peer_network_id),
                use_request_v2,
            )
            .await;

            // Verify that the client receives an invalid request error
            let response = mock_client
                .wait_for_response(response_receiver)
                .await
                .unwrap_err();
            assert!(matches!(response, StorageServiceError::InvalidRequest(_)));
        }

        // Verify the request indices that are pending
        verify_pending_subscription_request_indices(
            active_subscriptions,
            peer_network_id,
            max_num_active_subscriptions,
            max_num_active_subscriptions * 2,
            num_stream_requests,
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_subscription_overwrite_streams() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Create test data
        let highest_version = 45576;
        let highest_epoch = 43;
        let lowest_version = 0;
        let peer_version = highest_version - 100;
        let highest_ledger_info =
            utils::create_test_ledger_info_with_sigs(highest_epoch, highest_version);
        let output_list_with_proof = utils::create_output_list_with_proof(
            peer_version + 1,
            highest_version,
            highest_version,
            use_request_v2,
        );
        let transaction_list_with_proof = utils::create_transaction_list_with_proof(
            peer_version + 1,
            highest_version,
            highest_version,
            false,
            use_request_v2,
        );

        // Create the mock db reader
        let mut db_reader =
            mock::create_mock_db_with_summary_updates(highest_ledger_info.clone(), lowest_version);
        utils::expect_get_transaction_outputs(
            &mut db_reader,
            peer_version + 1,
            highest_version - peer_version,
            highest_version,
            output_list_with_proof.clone(),
        );
        utils::expect_get_transactions(
            &mut db_reader,
            peer_version + 1,
            highest_version - peer_version,
            highest_version,
            false,
            transaction_list_with_proof.clone(),
        );

        // Create a storage service config
        let storage_config = utils::create_storage_config(use_request_v2);

        // Create the storage client and server
        let (mut mock_client, service, storage_service_notifier, mock_time, _) =
            MockClient::new(Some(db_reader), Some(storage_config));
        let active_subscriptions = service.get_subscriptions();
        tokio::spawn(service.start());

        // Create a peer network ID and stream ID
        let peer_network_id = PeerNetworkId::random();
        let stream_id = utils::get_random_u64();

        // Send multiple requests to subscribe to transaction outputs with the stream ID
        let num_stream_requests = 10;
        let mut response_receivers = utils::send_output_subscription_request_batch(
            &mut mock_client,
            peer_network_id,
            0,
            num_stream_requests - 1,
            stream_id,
            peer_version,
            highest_epoch,
            use_request_v2,
        )
        .await;

        // Wait until the stream requests are active
        utils::wait_for_active_stream_requests(
            active_subscriptions.clone(),
            peer_network_id,
            num_stream_requests as usize,
        )
        .await;

        // Verify no subscription response has been received yet
        utils::verify_no_subscription_responses(&mut response_receivers);

        // Force the subscription handler to work
        utils::force_subscription_handler_to_run(
            &mut mock_client,
            &mock_time,
            &storage_service_notifier,
        )
        .await;

        // Verify that the correct response is received (when it comes through)
        utils::verify_output_subscription_response(
            vec![output_list_with_proof.clone()],
            highest_ledger_info.clone(),
            &mut mock_client,
            &mut response_receivers,
            0,
            use_request_v2,
        )
        .await;

        // Send a request to subscribe to transactions with a new stream ID
        let new_stream_id = utils::get_random_u64();
        let response_receiver = utils::subscribe_to_transactions_for_peer(
            &mut mock_client,
            peer_version,
            highest_epoch,
            false,
            new_stream_id,
            0,
            Some(peer_network_id),
            use_request_v2,
        )
        .await;

        // Wait until the stream requests are active
        utils::wait_for_active_stream_requests(active_subscriptions.clone(), peer_network_id, 1)
            .await;

        // Verify the new stream ID has been used
        utils::verify_active_stream_id_for_peer(
            active_subscriptions.clone(),
            peer_network_id,
            new_stream_id,
        );

        // Force the subscription handler to work
        utils::force_cache_update_notification(
            &mut mock_client,
            &mock_time,
            &storage_service_notifier,
            true,
            true,
        )
        .await;

        // Verify a response is received and that it contains the correct data
        utils::verify_new_transactions_with_proof(
            &mut mock_client,
            response_receiver,
            use_request_v2,
            transaction_list_with_proof,
            highest_ledger_info,
        )
        .await;
    }
}

/// Adds a subscription request to the subscription stream for the given peer
fn add_subscription_request_to_stream(
    subscription_request: SubscriptionRequest,
    subscriptions: Arc<DashMap<PeerNetworkId, SubscriptionStreamRequests>>,
    peer_network_id: &PeerNetworkId,
) -> Result<(), (Error, SubscriptionRequest)> {
    let mut subscription = subscriptions.get_mut(peer_network_id).unwrap();
    let subscription_stream_requests = subscription.value_mut();
    subscription_stream_requests
        .add_subscription_request(StorageServiceConfig::default(), subscription_request)
}

/// Creates a random request for subscription data
fn create_subscription_data_request(
    known_version_at_stream_start: Option<u64>,
    known_epoch_at_stream_start: Option<u64>,
    subscription_stream_id: Option<u64>,
    subscription_stream_index: Option<u64>,
    use_request_v2: bool,
) -> DataRequest {
    // Get the request data
    let known_version_at_stream_start = known_version_at_stream_start.unwrap_or_default();
    let known_epoch_at_stream_start = known_epoch_at_stream_start.unwrap_or_default();
    let subscription_stream_id = subscription_stream_id.unwrap_or_default();
    let subscription_stream_index = subscription_stream_index.unwrap_or_default();

    // Create the subscription stream metadata
    let subscription_stream_metadata = SubscriptionStreamMetadata {
        known_version_at_stream_start,
        known_epoch_at_stream_start,
        subscription_stream_id,
    };

    // Generate the random data request
    let random_number = utils::get_random_u64();
    match random_number % 3 {
        0 => {
            if use_request_v2 {
                let transaction_data_request_type =
                    TransactionDataRequestType::TransactionData(TransactionData {
                        include_events: true,
                    });
                DataRequest::SubscribeTransactionDataWithProof(
                    SubscribeTransactionDataWithProofRequest {
                        transaction_data_request_type,
                        subscription_stream_metadata,
                        subscription_stream_index,
                        max_response_bytes: 0,
                    },
                )
            } else {
                DataRequest::SubscribeTransactionsWithProof(SubscribeTransactionsWithProofRequest {
                    subscription_stream_metadata,
                    subscription_stream_index,
                    include_events: true,
                })
            }
        },
        1 => {
            if use_request_v2 {
                let transaction_data_request_type =
                    TransactionDataRequestType::TransactionOutputData;
                DataRequest::SubscribeTransactionDataWithProof(
                    SubscribeTransactionDataWithProofRequest {
                        transaction_data_request_type,
                        subscription_stream_metadata,
                        subscription_stream_index,
                        max_response_bytes: 0,
                    },
                )
            } else {
                DataRequest::SubscribeTransactionOutputsWithProof(
                    SubscribeTransactionOutputsWithProofRequest {
                        subscription_stream_metadata,
                        subscription_stream_index,
                    },
                )
            }
        },
        2 => {
            if use_request_v2 {
                let transaction_data_request_type =
                    TransactionDataRequestType::TransactionOrOutputData(TransactionOrOutputData {
                        include_events: true,
                    });
                DataRequest::SubscribeTransactionDataWithProof(
                    SubscribeTransactionDataWithProofRequest {
                        transaction_data_request_type,
                        subscription_stream_metadata,
                        subscription_stream_index,
                        max_response_bytes: 0,
                    },
                )
            } else {
                DataRequest::SubscribeTransactionsOrOutputsWithProof(
                    SubscribeTransactionsOrOutputsWithProofRequest {
                        subscription_stream_metadata,
                        include_events: false,
                        max_num_output_reductions: 0,
                        subscription_stream_index,
                    },
                )
            }
        },
        number => panic!("This shouldn't be possible! Got: {:?}", number),
    }
}

/// Creates a random subscription request using the given data
fn create_subscription_request(
    time_service: &TimeService,
    known_version: Option<u64>,
    known_epoch: Option<u64>,
    subscription_stream_id: Option<u64>,
    subscription_stream_index: Option<u64>,
    use_request_v2: bool,
) -> SubscriptionRequest {
    // Create a storage service request
    let data_request = create_subscription_data_request(
        known_version,
        known_epoch,
        subscription_stream_id,
        subscription_stream_index,
        use_request_v2,
    );
    let storage_service_request = StorageServiceRequest::new(data_request, true);

    // Create the response sender
    let (callback, _) = oneshot::channel();
    let response_sender = ResponseSender::new(callback);

    // Create a subscription request
    SubscriptionRequest::new(
        storage_service_request,
        response_sender,
        time_service.clone(),
    )
}

/// Creates a random subscription stream using the given data
fn create_subscription_stream_requests(
    time_service: TimeService,
    known_version: Option<u64>,
    known_epoch: Option<u64>,
    subscription_stream_id: Option<u64>,
    subscription_stream_index: Option<u64>,
    use_request_v2: bool,
) -> SubscriptionStreamRequests {
    // Create a new subscription request
    let subscription_request = create_subscription_request(
        &time_service,
        known_version,
        known_epoch,
        subscription_stream_id,
        subscription_stream_index,
        use_request_v2,
    );

    // Create and return the subscription stream containing the request
    SubscriptionStreamRequests::new(subscription_request, time_service)
}

/// Verifies that the pending subscription request indices are valid.
/// Note the expected end indices are exclusive.
fn verify_pending_subscription_request_indices(
    active_subscriptions: Arc<DashMap<PeerNetworkId, SubscriptionStreamRequests>>,
    peer_network_id: PeerNetworkId,
    expected_start_index: u64,
    expected_end_index: u64,
    ignored_end_index: u64,
) {
    // Get the pending subscription requests
    let mut subscription = active_subscriptions.get_mut(&peer_network_id).unwrap();
    let subscription_stream_requests = subscription.value_mut();
    let pending_subscription_requests =
        subscription_stream_requests.get_pending_subscription_requests();

    // Verify that the expected indices are present
    for request_index in expected_start_index..expected_end_index {
        assert!(pending_subscription_requests.contains_key(&request_index));
    }

    // Verify that the ignored indices are not present
    for request_index in expected_end_index..ignored_end_index {
        assert!(!pending_subscription_requests.contains_key(&request_index));
    }
}
