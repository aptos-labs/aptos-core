// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::tests::{mock, mock::MockClient, utils};
use aptos_config::{
    config::StorageServiceConfig,
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_storage_service_types::requests::{
    DataRequest, NewTransactionOutputsWithProofRequest, StorageServiceRequest,
};
use aptos_types::{epoch_change::EpochChangeProof, PeerId};
use claims::assert_none;
use futures::channel::oneshot::Receiver;

#[tokio::test(flavor = "multi_thread")]
async fn test_get_new_transaction_outputs() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Test small and large chunk sizes
        let max_output_chunk_size =
            StorageServiceConfig::default().max_transaction_output_chunk_size;
        for chunk_size in [1, 100, max_output_chunk_size] {
            // Create test data
            let highest_version = 5060;
            let highest_epoch = 30;
            let lowest_version = 101;
            let peer_version = highest_version - chunk_size;
            let highest_ledger_info =
                utils::create_test_ledger_info_with_sigs(highest_epoch, highest_version);
            let output_list_with_proof = utils::create_output_list_with_proof(
                peer_version + 1,
                highest_version,
                highest_version,
                use_request_v2,
            );

            // Create the mock db reader
            let mut db_reader = mock::create_mock_db_with_summary_updates(
                highest_ledger_info.clone(),
                lowest_version,
            );
            utils::expect_get_transaction_outputs(
                &mut db_reader,
                peer_version + 1,
                highest_version - peer_version,
                highest_version,
                output_list_with_proof.clone(),
            );

            // Create a storage service config
            let storage_config = utils::create_storage_config(use_request_v2);

            // Create the storage client and server
            let (mut mock_client, service, storage_service_notifier, mock_time, _) =
                MockClient::new(Some(db_reader), Some(storage_config));
            let active_optimistic_fetches = service.get_optimistic_fetches();
            tokio::spawn(service.start());

            // Send a request to optimistically fetch new transaction outputs
            let mut response_receiver = get_new_outputs_with_proof(
                &mut mock_client,
                peer_version,
                highest_epoch,
                use_request_v2,
            )
            .await;

            // Wait until the optimistic fetch is active
            utils::wait_for_active_optimistic_fetches(active_optimistic_fetches.clone(), 1).await;

            // Verify no optimistic fetch response has been received yet
            assert_none!(response_receiver.try_recv().unwrap());

            // Force the optimistic fetch handler to work
            utils::force_optimistic_fetch_handler_to_run(
                &mut mock_client,
                &mock_time,
                &storage_service_notifier,
            )
            .await;

            // Verify a response is received and that it contains the correct data
            utils::verify_new_transaction_outputs_with_proof(
                &mut mock_client,
                response_receiver,
                use_request_v2,
                output_list_with_proof,
                highest_ledger_info,
            )
            .await;
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_new_transaction_outputs_different_networks() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Test small and large chunk sizes
        let max_output_chunk_size =
            StorageServiceConfig::default().max_transaction_output_chunk_size;
        for chunk_size in [100, max_output_chunk_size] {
            // Create test data
            let highest_version = 5060;
            let highest_epoch = 30;
            let lowest_version = 101;
            let peer_version_1 = highest_version - chunk_size;
            let peer_version_2 = highest_version - (chunk_size - 50);
            let highest_ledger_info =
                utils::create_test_ledger_info_with_sigs(highest_epoch, highest_version);
            let output_list_with_proof_1 = utils::create_output_list_with_proof(
                peer_version_1 + 1,
                highest_version,
                highest_version,
                use_request_v2,
            );
            let output_list_with_proof_2 = utils::create_output_list_with_proof(
                peer_version_2 + 1,
                highest_version,
                highest_version,
                use_request_v2,
            );

            // Create the mock db reader
            let mut db_reader = mock::create_mock_db_with_summary_updates(
                highest_ledger_info.clone(),
                lowest_version,
            );
            utils::expect_get_transaction_outputs(
                &mut db_reader,
                peer_version_1 + 1,
                highest_version - peer_version_1,
                highest_version,
                output_list_with_proof_1.clone(),
            );
            utils::expect_get_transaction_outputs(
                &mut db_reader,
                peer_version_2 + 1,
                highest_version - peer_version_2,
                highest_version,
                output_list_with_proof_2.clone(),
            );

            // Create a storage service config
            let storage_config = utils::create_storage_config(use_request_v2);

            // Create the storage client and server
            let (mut mock_client, service, storage_service_notifier, mock_time, _) =
                MockClient::new(Some(db_reader), Some(storage_config));
            let active_optimistic_fetches = service.get_optimistic_fetches();
            tokio::spawn(service.start());

            // Send a request to optimistically fetch new transaction outputs for peer 1
            let peer_id = PeerId::random();
            let peer_network_1 = PeerNetworkId::new(NetworkId::Validator, peer_id);
            let mut response_receiver_1 = get_new_outputs_with_proof_for_peer(
                &mut mock_client,
                peer_version_1,
                highest_epoch,
                Some(peer_network_1),
                use_request_v2,
            )
            .await;

            // Send a request to optimistically fetch new transaction outputs for peer 2
            let peer_network_2 = PeerNetworkId::new(NetworkId::Vfn, peer_id);
            let mut response_receiver_2 = get_new_outputs_with_proof_for_peer(
                &mut mock_client,
                peer_version_2,
                highest_epoch,
                Some(peer_network_2),
                use_request_v2,
            )
            .await;

            // Wait until the optimistic fetches are active
            utils::wait_for_active_optimistic_fetches(active_optimistic_fetches.clone(), 2).await;

            // Verify no optimistic fetch response has been received yet
            assert_none!(response_receiver_1.try_recv().unwrap());
            assert_none!(response_receiver_2.try_recv().unwrap());

            // Force the optimistic fetch handler to work
            utils::force_optimistic_fetch_handler_to_run(
                &mut mock_client,
                &mock_time,
                &storage_service_notifier,
            )
            .await;

            // Verify a response is received and that it contains the correct data
            utils::verify_new_transaction_outputs_with_proof(
                &mut mock_client,
                response_receiver_1,
                use_request_v2,
                output_list_with_proof_1,
                highest_ledger_info.clone(),
            )
            .await;
            utils::verify_new_transaction_outputs_with_proof(
                &mut mock_client,
                response_receiver_2,
                use_request_v2,
                output_list_with_proof_2,
                highest_ledger_info,
            )
            .await;
        }
    }
}

#[tokio::test]
#[should_panic(expected = "Canceled")]
async fn test_get_new_transaction_outputs_disable_v2() {
    // Create a storage service config with transaction v2 disabled
    let storage_config = utils::create_storage_config(false);

    // Create the storage client and server
    let (mut mock_client, service, _, _, _) = MockClient::new(None, Some(storage_config));
    tokio::spawn(service.start());

    // Send a new transaction output v2 request. This will cause a test panic
    // as no response will be received (the receiver is dropped).
    let response_receiver = get_new_outputs_with_proof(
        &mut mock_client,
        0,
        0,
        true, // use_request_v2
    )
    .await;

    // Wait for the response, which should never come
    response_receiver.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_new_transaction_outputs_epoch_change() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Create test data
        let highest_version = 10000;
        let highest_epoch = 10000;
        let lowest_version = 0;
        let peer_version = highest_version - 1000;
        let peer_epoch = highest_epoch - 1000;
        let epoch_change_version = peer_version + 1;
        let epoch_change_proof = EpochChangeProof {
            ledger_info_with_sigs: vec![utils::create_test_ledger_info_with_sigs(
                peer_epoch,
                epoch_change_version,
            )],
            more: false,
        };
        let output_list_with_proof = utils::create_output_list_with_proof(
            peer_version + 1,
            epoch_change_version,
            epoch_change_version,
            use_request_v2,
        );

        // Create the mock db reader
        let mut db_reader = mock::create_mock_db_with_summary_updates(
            utils::create_test_ledger_info_with_sigs(highest_epoch, highest_version),
            lowest_version,
        );
        utils::expect_get_transaction_outputs(
            &mut db_reader,
            peer_version + 1,
            epoch_change_version - peer_version,
            epoch_change_version,
            output_list_with_proof.clone(),
        );
        utils::expect_get_epoch_ending_ledger_infos(
            &mut db_reader,
            peer_epoch,
            peer_epoch + 1,
            epoch_change_proof.clone(),
        );

        // Create a storage service config
        let storage_config = utils::create_storage_config(use_request_v2);

        // Create the storage client and server
        let (mut mock_client, service, storage_service_notifier, mock_time, _) =
            MockClient::new(Some(db_reader), Some(storage_config));
        let active_optimistic_fetches = service.get_optimistic_fetches();
        tokio::spawn(service.start());

        // Send a request to optimistically fetch new transaction outputs
        let response_receiver =
            get_new_outputs_with_proof(&mut mock_client, peer_version, peer_epoch, use_request_v2)
                .await;

        // Wait until the optimistic fetch is active
        utils::wait_for_active_optimistic_fetches(active_optimistic_fetches.clone(), 1).await;

        // Force the optimistic fetch handler to work
        utils::force_optimistic_fetch_handler_to_run(
            &mut mock_client,
            &mock_time,
            &storage_service_notifier,
        )
        .await;

        // Verify a response is received and that it contains the correct data
        utils::verify_new_transaction_outputs_with_proof(
            &mut mock_client,
            response_receiver,
            use_request_v2,
            output_list_with_proof,
            epoch_change_proof.ledger_info_with_sigs[0].clone(),
        )
        .await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_new_transaction_outputs_max_chunk() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Create a storage service config with a configured max chunk size
        let max_transaction_output_chunk_size = 400;
        let storage_service_config = StorageServiceConfig {
            max_transaction_output_chunk_size,
            enable_transaction_data_v2: use_request_v2,
            ..StorageServiceConfig::default()
        };

        // Create test data
        let highest_version = 65660;
        let highest_epoch = 30;
        let lowest_version = 101;
        let requested_chunk_size = max_transaction_output_chunk_size + 100;
        let peer_version = highest_version - requested_chunk_size;
        let highest_ledger_info =
            utils::create_test_ledger_info_with_sigs(highest_epoch, highest_version);
        let output_list_with_proof = utils::create_output_list_with_proof(
            peer_version + 1,
            peer_version + max_transaction_output_chunk_size,
            highest_version,
            use_request_v2,
        );

        // Create the mock db reader
        let mut db_reader =
            mock::create_mock_db_with_summary_updates(highest_ledger_info.clone(), lowest_version);
        utils::expect_get_transaction_outputs(
            &mut db_reader,
            peer_version + 1,
            max_transaction_output_chunk_size,
            highest_version,
            output_list_with_proof.clone(),
        );

        // Create the storage client and server
        let (mut mock_client, service, storage_service_notifier, mock_time, _) =
            MockClient::new(Some(db_reader), Some(storage_service_config));
        let active_optimistic_fetches = service.get_optimistic_fetches();
        tokio::spawn(service.start());

        // Send a request to optimistically fetch new transaction outputs
        let response_receiver = get_new_outputs_with_proof(
            &mut mock_client,
            peer_version,
            highest_epoch,
            use_request_v2,
        )
        .await;

        // Wait until the optimistic fetch is active
        utils::wait_for_active_optimistic_fetches(active_optimistic_fetches.clone(), 1).await;

        // Force the optimistic fetch handler to work
        utils::force_optimistic_fetch_handler_to_run(
            &mut mock_client,
            &mock_time,
            &storage_service_notifier,
        )
        .await;

        // Verify a response is received and that it contains the correct data
        utils::verify_new_transaction_outputs_with_proof(
            &mut mock_client,
            response_receiver,
            use_request_v2,
            output_list_with_proof,
            highest_ledger_info,
        )
        .await;
    }
}

/// Creates and sends a request for new transaction outputs
async fn get_new_outputs_with_proof(
    mock_client: &mut MockClient,
    known_version: u64,
    known_epoch: u64,
    use_request_v2: bool,
) -> Receiver<Result<bytes::Bytes, aptos_network::protocols::network::RpcError>> {
    get_new_outputs_with_proof_for_peer(
        mock_client,
        known_version,
        known_epoch,
        None,
        use_request_v2,
    )
    .await
}

/// Creates and sends a request for new transaction outputs for the specified peer
async fn get_new_outputs_with_proof_for_peer(
    mock_client: &mut MockClient,
    known_version: u64,
    known_epoch: u64,
    peer_network_id: Option<PeerNetworkId>,
    use_request_v2: bool,
) -> Receiver<Result<bytes::Bytes, aptos_network::protocols::network::RpcError>> {
    // Create the data request
    let data_request = if use_request_v2 {
        DataRequest::get_new_transaction_output_data_with_proof(known_version, known_epoch, 0)
    } else {
        DataRequest::GetNewTransactionOutputsWithProof(NewTransactionOutputsWithProofRequest {
            known_version,
            known_epoch,
        })
    };
    let storage_request = StorageServiceRequest::new(data_request, true);

    // Send the request
    let (peer_id, network_id) = utils::extract_peer_and_network_id(peer_network_id);
    mock_client
        .send_request(storage_request, peer_id, network_id)
        .await
}
