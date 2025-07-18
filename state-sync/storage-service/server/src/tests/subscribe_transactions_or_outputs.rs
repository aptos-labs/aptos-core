// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::tests::{mock, mock::MockClient, utils};
use aptos_config::{
    config::StorageServiceConfig,
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_network::protocols::network::RpcError;
use aptos_types::{
    epoch_change::EpochChangeProof,
    ledger_info::LedgerInfoWithSignatures,
    transaction::{
        PersistedAuxiliaryInfo, TransactionListWithProof, TransactionOutputListWithProof,
    },
    PeerId,
};
use bytes::Bytes;
use claims::assert_none;
use futures::channel::oneshot::Receiver;
use std::collections::HashMap;

#[tokio::test(flavor = "multi_thread")]
async fn test_subscribe_transactions_or_outputs_different_network() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Test small and large chunk sizes
        let max_output_chunk_size =
            StorageServiceConfig::default().max_transaction_output_chunk_size;
        for chunk_size in [100, max_output_chunk_size] {
            // Test fallback to transaction syncing
            for fallback_to_transactions in [false, true] {
                // Create test data
                let highest_version = 5060;
                let highest_epoch = 30;
                let lowest_version = 101;
                let peer_version_1 = highest_version - chunk_size;
                let peer_version_2 = highest_version - chunk_size + 1;
                let highest_ledger_info =
                    utils::create_test_ledger_info_with_sigs(highest_epoch, highest_version);
                let output_list_with_proof_1 = utils::create_output_list_with_proof(
                    peer_version_1 + 1,
                    highest_version,
                    highest_version,
                );
                let output_list_with_proof_2 = utils::create_output_list_with_proof(
                    peer_version_2 + 1,
                    highest_version,
                    highest_version,
                );
                let transaction_list_with_proof_1 = utils::create_transaction_list_with_proof(
                    peer_version_1 + 1,
                    highest_version,
                    highest_version,
                    false,
                );
                let transaction_list_with_proof_2 = utils::create_transaction_list_with_proof(
                    peer_version_2 + 1,
                    highest_version,
                    highest_version,
                    false,
                );
                let persisted_auxiliary_infos_1 = utils::create_persisted_auxiliary_infos(
                    peer_version_1 + 1,
                    highest_version,
                    use_request_v2,
                );
                let persisted_auxiliary_infos_2 = utils::create_persisted_auxiliary_infos(
                    peer_version_2 + 1,
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
                    use_request_v2,
                    persisted_auxiliary_infos_1.clone(),
                );
                utils::expect_get_transaction_outputs(
                    &mut db_reader,
                    peer_version_2 + 1,
                    highest_version - peer_version_2,
                    highest_version,
                    output_list_with_proof_2.clone(),
                    use_request_v2,
                    persisted_auxiliary_infos_2.clone(),
                );
                if fallback_to_transactions {
                    utils::expect_get_transactions(
                        &mut db_reader,
                        peer_version_1 + 1,
                        highest_version - peer_version_1,
                        highest_version,
                        false,
                        transaction_list_with_proof_1.clone(),
                        use_request_v2,
                        persisted_auxiliary_infos_1.clone(),
                    );
                    utils::expect_get_transactions(
                        &mut db_reader,
                        peer_version_2 + 1,
                        highest_version - peer_version_2,
                        highest_version,
                        false,
                        transaction_list_with_proof_2.clone(),
                        use_request_v2,
                        persisted_auxiliary_infos_2.clone(),
                    );
                }

                // Create the storage client and server
                let storage_config = utils::configure_network_chunk_limit(
                    fallback_to_transactions,
                    &output_list_with_proof_1,
                    &transaction_list_with_proof_1,
                    use_request_v2,
                );
                let (mut mock_client, service, storage_service_notifier, mock_time, _) =
                    MockClient::new(Some(db_reader), Some(storage_config));
                let active_subscriptions = service.get_subscriptions();
                tokio::spawn(service.start());

                // Send a request to subscribe to transactions or outputs for peer 1
                let peer_id = PeerId::random();
                let subscription_stream_id = 56756;
                let peer_network_1 = PeerNetworkId::new(NetworkId::Public, peer_id);
                let mut response_receiver_1 = utils::subscribe_to_transactions_or_outputs_for_peer(
                    &mut mock_client,
                    peer_version_1,
                    highest_epoch,
                    false,
                    0, // Outputs cannot be reduced and will fallback to transactions
                    subscription_stream_id,
                    0,
                    Some(peer_network_1),
                    use_request_v2,
                )
                .await;

                // Send a request to subscribe to transactions or outputs for peer 2
                let peer_network_2 = PeerNetworkId::new(NetworkId::Vfn, peer_id);
                let mut response_receiver_2 = utils::subscribe_to_transactions_or_outputs_for_peer(
                    &mut mock_client,
                    peer_version_2,
                    highest_epoch,
                    false,
                    0, // Outputs cannot be reduced and will fallback to transactions
                    subscription_stream_id,
                    0,
                    Some(peer_network_2),
                    use_request_v2,
                )
                .await;

                // Wait until the subscriptions are active
                utils::wait_for_active_subscriptions(active_subscriptions.clone(), 2).await;

                // Verify no response has been received yet
                assert_none!(response_receiver_1.try_recv().unwrap());
                assert_none!(response_receiver_2.try_recv().unwrap());

                // Force the subscription handler to work
                utils::force_subscription_handler_to_run(
                    &mut mock_client,
                    &mock_time,
                    &storage_service_notifier,
                )
                .await;

                // Verify a response is received and that it contains the correct data
                if fallback_to_transactions {
                    utils::verify_new_transactions_or_outputs_with_proof(
                        &mut mock_client,
                        response_receiver_1,
                        use_request_v2,
                        Some(transaction_list_with_proof_1.clone()),
                        None,
                        highest_ledger_info.clone(),
                        persisted_auxiliary_infos_1.clone(),
                    )
                    .await;
                    utils::verify_new_transactions_or_outputs_with_proof(
                        &mut mock_client,
                        response_receiver_2,
                        use_request_v2,
                        Some(transaction_list_with_proof_2.clone()),
                        None,
                        highest_ledger_info,
                        persisted_auxiliary_infos_2.clone(),
                    )
                    .await;
                } else {
                    utils::verify_new_transactions_or_outputs_with_proof(
                        &mut mock_client,
                        response_receiver_1,
                        use_request_v2,
                        None,
                        Some(output_list_with_proof_1.clone()),
                        highest_ledger_info.clone(),
                        persisted_auxiliary_infos_1.clone(),
                    )
                    .await;
                    utils::verify_new_transactions_or_outputs_with_proof(
                        &mut mock_client,
                        response_receiver_2,
                        use_request_v2,
                        None,
                        Some(output_list_with_proof_2.clone()),
                        highest_ledger_info,
                        persisted_auxiliary_infos_2.clone(),
                    )
                    .await;
                }
            }
        }
    }
}

#[tokio::test]
#[should_panic(expected = "Canceled")]
async fn test_subscribe_transactions_or_outputs_disable_v2() {
    // Create a storage service config with transaction v2 disabled
    let storage_config = utils::create_storage_config(false);

    // Create the storage client and server
    let (mut mock_client, service, _, _, _) = MockClient::new(None, Some(storage_config));
    tokio::spawn(service.start());

    // Send a transaction or output v2 request. This will cause a test panic
    // as no response will be received (the receiver is dropped).
    let response_receiver = utils::subscribe_to_transactions_or_outputs(
        &mut mock_client,
        0,
        0,
        false,
        0,
        0,
        0,
        true, // Use transaction v2
    )
    .await;

    // Wait for the response, which should never come
    response_receiver.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_subscribe_transactions_or_outputs_epoch_change() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Test fallback to transaction syncing
        for fallback_to_transactions in [false, true] {
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
            );
            let transaction_list_with_proof = utils::create_transaction_list_with_proof(
                peer_version + 1,
                epoch_change_version,
                epoch_change_version,
                false,
            );
            let persisted_auxiliary_infos = utils::create_persisted_auxiliary_infos(
                peer_version + 1,
                epoch_change_version,
                use_request_v2,
            );

            // Create the mock db reader
            let mut db_reader = mock::create_mock_db_with_summary_updates(
                utils::create_test_ledger_info_with_sigs(highest_epoch, highest_version),
                lowest_version,
            );
            utils::expect_get_epoch_ending_ledger_infos(
                &mut db_reader,
                peer_epoch,
                peer_epoch + 1,
                epoch_change_proof.clone(),
            );
            utils::expect_get_transaction_outputs(
                &mut db_reader,
                peer_version + 1,
                epoch_change_version - peer_version,
                epoch_change_version,
                output_list_with_proof.clone(),
                use_request_v2,
                persisted_auxiliary_infos.clone(),
            );
            if fallback_to_transactions {
                utils::expect_get_transactions(
                    &mut db_reader,
                    peer_version + 1,
                    epoch_change_version - peer_version,
                    epoch_change_version,
                    false,
                    transaction_list_with_proof.clone(),
                    use_request_v2,
                    persisted_auxiliary_infos.clone(),
                );
            }

            // Create the storage client and server
            let storage_config = utils::configure_network_chunk_limit(
                fallback_to_transactions,
                &output_list_with_proof,
                &transaction_list_with_proof,
                use_request_v2,
            );
            let (mut mock_client, service, storage_service_notifier, mock_time, _) =
                MockClient::new(Some(db_reader), Some(storage_config));
            let active_subscriptions = service.get_subscriptions();
            tokio::spawn(service.start());

            // Send a request to subscribe to new transactions or outputs
            let response_receiver = utils::subscribe_to_transactions_or_outputs(
                &mut mock_client,
                peer_version,
                peer_epoch,
                false,
                5,
                utils::get_random_u64(),
                0,
                use_request_v2,
            )
            .await;

            // Wait until the subscription is active
            utils::wait_for_active_subscriptions(active_subscriptions.clone(), 1).await;

            // Force the subscription handler to work
            utils::force_subscription_handler_to_run(
                &mut mock_client,
                &mock_time,
                &storage_service_notifier,
            )
            .await;

            // Verify a response is received and that it contains the correct data
            if fallback_to_transactions {
                utils::verify_new_transactions_or_outputs_with_proof(
                    &mut mock_client,
                    response_receiver,
                    use_request_v2,
                    Some(transaction_list_with_proof),
                    None,
                    epoch_change_proof.ledger_info_with_sigs[0].clone(),
                    persisted_auxiliary_infos.clone(),
                )
                .await;
            } else {
                utils::verify_new_transactions_or_outputs_with_proof(
                    &mut mock_client,
                    response_receiver,
                    use_request_v2,
                    None,
                    Some(output_list_with_proof),
                    epoch_change_proof.ledger_info_with_sigs[0].clone(),
                    persisted_auxiliary_infos.clone(),
                )
                .await;
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_subscribe_transactions_or_outputs_max_chunk() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Test fallback to transaction syncing
        for fallback_to_transactions in [false, true] {
            // Create test data
            let highest_version = 65660;
            let highest_epoch = 30;
            let lowest_version = 101;
            let max_transaction_output_chunk_size =
                StorageServiceConfig::default().max_transaction_output_chunk_size;
            let requested_chunk_size = max_transaction_output_chunk_size + 100;
            let peer_version = highest_version - requested_chunk_size;
            let highest_ledger_info =
                utils::create_test_ledger_info_with_sigs(highest_epoch, highest_version);
            let output_list_with_proof = utils::create_output_list_with_proof(
                peer_version + 1,
                peer_version + max_transaction_output_chunk_size,
                highest_version,
            );
            let transaction_list_with_proof = utils::create_transaction_list_with_proof(
                peer_version + 1,
                peer_version + max_transaction_output_chunk_size,
                peer_version + max_transaction_output_chunk_size,
                false,
            );
            let persisted_auxiliary_infos = utils::create_persisted_auxiliary_infos(
                peer_version + 1,
                peer_version + max_transaction_output_chunk_size,
                use_request_v2,
            );

            // Create the mock db reader
            let max_num_output_reductions = 5;
            let mut db_reader = mock::create_mock_db_with_summary_updates(
                highest_ledger_info.clone(),
                lowest_version,
            );
            for i in 0..=max_num_output_reductions {
                utils::expect_get_transaction_outputs(
                    &mut db_reader,
                    peer_version + 1,
                    (max_transaction_output_chunk_size as u32 / (u32::pow(2, i as u32))) as u64,
                    highest_version,
                    output_list_with_proof.clone(),
                    use_request_v2,
                    persisted_auxiliary_infos.clone(),
                );
            }
            if fallback_to_transactions {
                utils::expect_get_transactions(
                    &mut db_reader,
                    peer_version + 1,
                    max_transaction_output_chunk_size,
                    highest_version,
                    false,
                    transaction_list_with_proof.clone(),
                    use_request_v2,
                    persisted_auxiliary_infos.clone(),
                );
            }

            // Create the storage client and server
            let storage_config = utils::configure_network_chunk_limit(
                fallback_to_transactions,
                &output_list_with_proof,
                &transaction_list_with_proof,
                use_request_v2,
            );
            let (mut mock_client, service, storage_service_notifier, mock_time, _) =
                MockClient::new(Some(db_reader), Some(storage_config));
            let active_subscriptions = service.get_subscriptions();
            tokio::spawn(service.start());

            // Send a request to subscribe to new transactions or outputs
            let response_receiver = utils::subscribe_to_transactions_or_outputs(
                &mut mock_client,
                peer_version,
                highest_epoch,
                false,
                max_num_output_reductions,
                utils::get_random_u64(),
                0,
                use_request_v2,
            )
            .await;

            // Wait until the subscription is active
            utils::wait_for_active_subscriptions(active_subscriptions.clone(), 1).await;

            // Force the subscription handler to work
            utils::force_subscription_handler_to_run(
                &mut mock_client,
                &mock_time,
                &storage_service_notifier,
            )
            .await;

            // Verify a response is received and that it contains the correct data
            if fallback_to_transactions {
                utils::verify_new_transactions_or_outputs_with_proof(
                    &mut mock_client,
                    response_receiver,
                    use_request_v2,
                    Some(transaction_list_with_proof),
                    None,
                    highest_ledger_info,
                    persisted_auxiliary_infos.clone(),
                )
                .await;
            } else {
                utils::verify_new_transactions_or_outputs_with_proof(
                    &mut mock_client,
                    response_receiver,
                    use_request_v2,
                    None,
                    Some(output_list_with_proof),
                    highest_ledger_info,
                    persisted_auxiliary_infos.clone(),
                )
                .await;
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_subscribe_transaction_or_outputs_streaming() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Test fallback to transaction syncing
        for fallback_to_transactions in [false, true] {
            // Create test data
            let max_transaction_output_chunk_size = 90;
            let num_stream_requests = 30;
            let highest_version = 45576;
            let highest_epoch = 43;
            let lowest_version = 2;
            let peer_version = 1;
            let highest_ledger_info =
                utils::create_test_ledger_info_with_sigs(highest_epoch, highest_version);

            // Create the transaction and output lists with proofs
            let chunk_start_and_end_versions = (0..num_stream_requests)
                .map(|i| {
                    let start_version = peer_version + (i * max_transaction_output_chunk_size) + 1;
                    let end_version = start_version + max_transaction_output_chunk_size - 1;
                    (start_version, end_version)
                })
                .collect::<Vec<_>>();
            let output_lists_with_proofs: Vec<_> = chunk_start_and_end_versions
                .iter()
                .map(|(start_version, end_version)| {
                    utils::create_output_list_with_proof(
                        *start_version,
                        *end_version,
                        highest_version,
                    )
                })
                .collect();
            let transaction_lists_with_proofs: Vec<_> = chunk_start_and_end_versions
                .iter()
                .map(|(start_version, end_version)| {
                    utils::create_transaction_list_with_proof(
                        *start_version,
                        *end_version,
                        highest_version,
                        false,
                    )
                })
                .collect();

            // Create the persisted auxiliary infos
            let persisted_auxiliary_infos: Vec<_> = chunk_start_and_end_versions
                .iter()
                .map(|(start_version, end_version)| {
                    utils::create_persisted_auxiliary_infos(
                        *start_version,
                        *end_version,
                        use_request_v2,
                    )
                })
                .collect();

            // Create the mock db reader
            let mut db_reader = mock::create_mock_db_with_summary_updates(
                highest_ledger_info.clone(),
                lowest_version,
            );
            for (i, (start_version, _)) in chunk_start_and_end_versions.iter().enumerate() {
                // Set expectations for transaction output reads
                utils::expect_get_transaction_outputs(
                    &mut db_reader,
                    *start_version,
                    max_transaction_output_chunk_size,
                    highest_version,
                    output_lists_with_proofs[i].clone(),
                    use_request_v2,
                    persisted_auxiliary_infos[i].clone(),
                );

                // Set expectations for transaction reads
                if fallback_to_transactions {
                    utils::expect_get_transactions(
                        &mut db_reader,
                        *start_version,
                        max_transaction_output_chunk_size,
                        highest_version,
                        false,
                        transaction_lists_with_proofs[i].clone(),
                        use_request_v2,
                        persisted_auxiliary_infos[i].clone(),
                    );
                }
            }

            // Create the storage service config
            let mut storage_service_config = utils::configure_network_chunk_limit(
                fallback_to_transactions,
                &output_lists_with_proofs[0],
                &transaction_lists_with_proofs[0],
                use_request_v2,
            );
            storage_service_config.max_transaction_output_chunk_size =
                max_transaction_output_chunk_size;

            // Create the storage client and server
            let (mut mock_client, service, storage_service_notifier, mock_time, _) =
                MockClient::new(Some(db_reader), Some(storage_service_config));
            let active_subscriptions = service.get_subscriptions();
            tokio::spawn(service.start());

            // Create a new peer and stream ID
            let peer_network_id = PeerNetworkId::random();
            let stream_id = utils::get_random_u64();

            // Send multiple batches of requests to the server and verify the responses
            let num_batches_to_send = 5;
            for batch_id in 0..num_batches_to_send {
                // Send the request batch to subscribe to transaction outputs
                let num_requests_per_batch = num_stream_requests / num_batches_to_send;
                let first_request_index = batch_id * num_requests_per_batch;
                let last_request_index =
                    (batch_id * num_requests_per_batch) + num_requests_per_batch - 1;
                let mut response_receivers = send_transaction_or_output_subscription_request_batch(
                    &mut mock_client,
                    peer_network_id,
                    first_request_index,
                    last_request_index,
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
                    num_requests_per_batch as usize,
                )
                .await;

                // Force the subscription handler to work
                utils::force_cache_update_notification(
                    &mut mock_client,
                    &mock_time,
                    &storage_service_notifier,
                    true,
                    true,
                )
                .await;

                // Continuously run the subscription service until the batch responses are received
                for stream_request_index in first_request_index..=last_request_index {
                    // Verify that the correct response is received
                    verify_transaction_or_output_subscription_response(
                        transaction_lists_with_proofs.clone(),
                        output_lists_with_proofs.clone(),
                        persisted_auxiliary_infos.clone(),
                        highest_ledger_info.clone(),
                        fallback_to_transactions,
                        &mut mock_client,
                        &mut response_receivers,
                        stream_request_index,
                        use_request_v2,
                    )
                    .await;
                }
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_subscribe_transactions_or_outputs_streaming_epoch_change() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Test fallback to transaction syncing
        for fallback_to_transactions in [false, true] {
            // Create test data
            let max_transaction_output_chunk_size = 10;
            let max_num_active_subscriptions = 50;
            let highest_version = 1000;
            let highest_epoch = 2;
            let lowest_version = 0;
            let peer_version = highest_version - 900;
            let peer_epoch = highest_epoch - 1;
            let epoch_change_version = peer_version + 97;

            // Create the highest ledger info and epoch change proof
            let highest_ledger_info =
                utils::create_test_ledger_info_with_sigs(highest_epoch, highest_version);
            let epoch_change_ledger_info =
                utils::create_epoch_ending_ledger_info(peer_epoch, epoch_change_version);
            let epoch_change_proof = EpochChangeProof {
                ledger_info_with_sigs: vec![epoch_change_ledger_info.clone()],
                more: false,
            };

            // Create the transaction and output lists with proofs
            let chunk_start_and_end_versions = utils::create_data_chunks_with_epoch_boundary(
                max_transaction_output_chunk_size,
                max_num_active_subscriptions,
                peer_version,
                epoch_change_version,
            );
            let output_lists_with_proofs: Vec<_> = chunk_start_and_end_versions
                .iter()
                .map(|(start_version, end_version)| {
                    utils::create_output_list_with_proof(
                        *start_version,
                        *end_version,
                        highest_version,
                    )
                })
                .collect();
            let transaction_lists_with_proofs: Vec<_> = chunk_start_and_end_versions
                .iter()
                .map(|(start_version, end_version)| {
                    utils::create_transaction_list_with_proof(
                        *start_version,
                        *end_version,
                        highest_version,
                        false,
                    )
                })
                .collect();

            // Create the persisted auxiliary infos
            let persisted_auxiliary_infos: Vec<_> = chunk_start_and_end_versions
                .iter()
                .map(|(start_version, end_version)| {
                    utils::create_persisted_auxiliary_infos(
                        *start_version,
                        *end_version,
                        use_request_v2,
                    )
                })
                .collect();

            // Create the mock db reader
            let mut db_reader = mock::create_mock_db_with_summary_updates(
                highest_ledger_info.clone(),
                lowest_version,
            );
            utils::expect_get_epoch_ending_ledger_infos(
                &mut db_reader,
                peer_epoch,
                peer_epoch + 1,
                epoch_change_proof.clone(),
            );
            for (i, (start_version, end_version)) in chunk_start_and_end_versions.iter().enumerate()
            {
                // Set expectations for transaction output reads
                let proof_version = if *end_version <= epoch_change_version {
                    epoch_change_version
                } else {
                    highest_version
                };
                utils::expect_get_transaction_outputs(
                    &mut db_reader,
                    *start_version,
                    end_version - start_version + 1,
                    proof_version,
                    output_lists_with_proofs[i].clone(),
                    use_request_v2,
                    persisted_auxiliary_infos[i].clone(),
                );

                // Set expectations for transaction reads
                if fallback_to_transactions {
                    utils::expect_get_transactions(
                        &mut db_reader,
                        *start_version,
                        end_version - start_version + 1,
                        proof_version,
                        false,
                        transaction_lists_with_proofs[i].clone(),
                        use_request_v2,
                        persisted_auxiliary_infos[i].clone(),
                    );
                }
            }

            // Create the storage service config
            let mut storage_service_config = utils::configure_network_chunk_limit(
                fallback_to_transactions,
                &output_lists_with_proofs[0],
                &transaction_lists_with_proofs[0],
                use_request_v2,
            );
            storage_service_config.max_transaction_output_chunk_size =
                max_transaction_output_chunk_size;
            storage_service_config.max_num_active_subscriptions = max_num_active_subscriptions;

            // Create the storage client and server
            let (mut mock_client, service, storage_service_notifier, mock_time, _) =
                MockClient::new(Some(db_reader), Some(storage_service_config));
            let active_subscriptions = service.get_subscriptions();
            tokio::spawn(service.start());

            // Create a new peer and stream ID
            let peer_network_id = PeerNetworkId::random();
            let stream_id = utils::get_random_u64();

            // Send the request batch to subscribe to transactions or outputs
            let mut response_receivers = send_transaction_or_output_subscription_request_batch(
                &mut mock_client,
                peer_network_id,
                0,
                max_num_active_subscriptions - 1,
                stream_id,
                peer_version,
                peer_epoch,
                use_request_v2,
            )
            .await;

            // Wait until the stream requests are active
            utils::wait_for_active_stream_requests(
                active_subscriptions.clone(),
                peer_network_id,
                max_num_active_subscriptions as usize,
            )
            .await;

            // Force the subscription handler to work
            utils::force_subscription_handler_to_run(
                &mut mock_client,
                &mock_time,
                &storage_service_notifier,
            )
            .await;

            // Continuously run the subscription service until all the responses are received
            for stream_request_index in 0..max_num_active_subscriptions {
                // Determine the target ledger info for the response
                let first_version = output_lists_with_proofs[stream_request_index as usize]
                    .get_first_output_version()
                    .unwrap();
                let target_ledger_info = if first_version > epoch_change_version {
                    highest_ledger_info.clone()
                } else {
                    epoch_change_ledger_info.clone()
                };

                // If we're syncing to the epoch change, then we don't need
                // to fallback as the configured network limit won't be reached.
                let epoch_change_version = epoch_change_ledger_info.ledger_info().version();
                let fallback_to_transactions = if fallback_to_transactions
                    && (first_version < epoch_change_version)
                    && (first_version + max_transaction_output_chunk_size) >= epoch_change_version
                {
                    false
                } else {
                    fallback_to_transactions
                };

                // Verify that the correct response is received
                verify_transaction_or_output_subscription_response(
                    transaction_lists_with_proofs.clone(),
                    output_lists_with_proofs.clone(),
                    persisted_auxiliary_infos.clone(),
                    target_ledger_info.clone(),
                    fallback_to_transactions,
                    &mut mock_client,
                    &mut response_receivers,
                    stream_request_index,
                    use_request_v2,
                )
                .await;
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_subscribe_transaction_or_outputs_streaming_loop() {
    // Test both v1 and v2 data requests
    for use_request_v2 in [false, true] {
        // Test fallback to transaction syncing
        for fallback_to_transactions in [false, true] {
            // Create test data
            let max_transaction_output_chunk_size = 90;
            let num_stream_requests = 30;
            let highest_version = 45576;
            let highest_epoch = 43;
            let lowest_version = 2;
            let peer_version = 1;
            let highest_ledger_info =
                utils::create_test_ledger_info_with_sigs(highest_epoch, highest_version);

            // Create the transaction and output lists with proofs
            let chunk_start_and_end_versions = (0..num_stream_requests)
                .map(|i| {
                    let start_version = peer_version + (i * max_transaction_output_chunk_size) + 1;
                    let end_version = start_version + max_transaction_output_chunk_size - 1;
                    (start_version, end_version)
                })
                .collect::<Vec<_>>();
            let output_lists_with_proofs: Vec<_> = chunk_start_and_end_versions
                .iter()
                .map(|(start_version, end_version)| {
                    utils::create_output_list_with_proof(
                        *start_version,
                        *end_version,
                        highest_version,
                    )
                })
                .collect();
            let transaction_lists_with_proofs: Vec<_> = chunk_start_and_end_versions
                .iter()
                .map(|(start_version, end_version)| {
                    utils::create_transaction_list_with_proof(
                        *start_version,
                        *end_version,
                        highest_version,
                        false,
                    )
                })
                .collect();

            // Create the persisted auxiliary infos
            let persisted_auxiliary_infos: Vec<_> = chunk_start_and_end_versions
                .iter()
                .map(|(start_version, end_version)| {
                    utils::create_persisted_auxiliary_infos(
                        *start_version,
                        *end_version,
                        use_request_v2,
                    )
                })
                .collect();

            // Create the mock db reader
            let mut db_reader = mock::create_mock_db_with_summary_updates(
                highest_ledger_info.clone(),
                lowest_version,
            );
            for (i, (start_version, _)) in chunk_start_and_end_versions.iter().enumerate() {
                // Set expectations for transaction output reads
                utils::expect_get_transaction_outputs(
                    &mut db_reader,
                    *start_version,
                    max_transaction_output_chunk_size,
                    highest_version,
                    output_lists_with_proofs[i].clone(),
                    use_request_v2,
                    persisted_auxiliary_infos[i].clone(),
                );

                // Set expectations for transaction reads
                if fallback_to_transactions {
                    utils::expect_get_transactions(
                        &mut db_reader,
                        *start_version,
                        max_transaction_output_chunk_size,
                        highest_version,
                        false,
                        transaction_lists_with_proofs[i].clone(),
                        use_request_v2,
                        persisted_auxiliary_infos[i].clone(),
                    );
                }
            }

            // Create the storage service config
            let mut storage_service_config = utils::configure_network_chunk_limit(
                fallback_to_transactions,
                &output_lists_with_proofs[0],
                &transaction_lists_with_proofs[0],
                use_request_v2,
            );
            storage_service_config.max_transaction_output_chunk_size =
                max_transaction_output_chunk_size;

            // Create the storage client and server
            let (mut mock_client, service, storage_service_notifier, mock_time, _) =
                MockClient::new(Some(db_reader), Some(storage_service_config));
            let active_subscriptions = service.get_subscriptions();
            tokio::spawn(service.start());

            // Create a new peer and stream ID
            let peer_network_id = PeerNetworkId::random();
            let stream_id = utils::get_random_u64();

            // Send the requests to the server and verify the responses
            let mut response_receivers = send_transaction_or_output_subscription_request_batch(
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

            // Verify the state of the subscription stream
            utils::verify_subscription_stream_entry(
                active_subscriptions.clone(),
                peer_network_id,
                num_stream_requests,
                peer_version,
                highest_epoch,
                max_transaction_output_chunk_size,
            );

            // Force the subscription handler to work
            utils::force_subscription_handler_to_run(
                &mut mock_client,
                &mock_time,
                &storage_service_notifier,
            )
            .await;

            // Verify all responses are received
            for stream_request_index in 0..num_stream_requests {
                let response_receiver = response_receivers.remove(&stream_request_index).unwrap();
                if fallback_to_transactions {
                    utils::verify_new_transactions_or_outputs_with_proof(
                        &mut mock_client,
                        response_receiver,
                        use_request_v2,
                        Some(transaction_lists_with_proofs[stream_request_index as usize].clone()),
                        None,
                        highest_ledger_info.clone(),
                        persisted_auxiliary_infos[stream_request_index as usize].clone(),
                    )
                    .await;
                } else {
                    utils::verify_new_transactions_or_outputs_with_proof(
                        &mut mock_client,
                        response_receiver,
                        use_request_v2,
                        None,
                        Some(output_lists_with_proofs[stream_request_index as usize].clone()),
                        highest_ledger_info.clone(),
                        persisted_auxiliary_infos[stream_request_index as usize].clone(),
                    )
                    .await;
                }
            }
        }
    }
}

/// Sends a batch of transaction or output requests and
/// returns the response receivers for each request.
async fn send_transaction_or_output_subscription_request_batch(
    mock_client: &mut MockClient,
    peer_network_id: PeerNetworkId,
    first_stream_request_index: u64,
    last_stream_request_index: u64,
    stream_id: u64,
    peer_version: u64,
    peer_epoch: u64,
    use_request_v2: bool,
) -> HashMap<u64, Receiver<Result<Bytes, RpcError>>> {
    // Shuffle the stream request indices to emulate out of order requests
    let stream_request_indices =
        utils::create_shuffled_vector(first_stream_request_index, last_stream_request_index);

    // Send the requests and gather the response receivers
    let mut response_receivers = HashMap::new();
    for stream_request_index in stream_request_indices {
        // Send the transaction output subscription request
        let response_receiver = utils::subscribe_to_transactions_or_outputs_for_peer(
            mock_client,
            peer_version,
            peer_epoch,
            false,
            0, // Outputs cannot be reduced and will fallback to transactions
            stream_id,
            stream_request_index,
            Some(peer_network_id),
            use_request_v2,
        )
        .await;

        // Save the response receiver
        response_receivers.insert(stream_request_index, response_receiver);
    }

    response_receivers
}

/// Verifies that a response is received for a given stream request index
/// and that the response contains the correct data.
async fn verify_transaction_or_output_subscription_response(
    expected_transaction_lists_with_proofs: Vec<TransactionListWithProof>,
    expected_output_lists_with_proofs: Vec<TransactionOutputListWithProof>,
    expected_persisted_auxiliary_infos: Vec<Option<Vec<PersistedAuxiliaryInfo>>>,
    expected_target_ledger_info: LedgerInfoWithSignatures,
    fallback_to_transactions: bool,
    mock_client: &mut MockClient,
    response_receivers: &mut HashMap<u64, Receiver<Result<Bytes, RpcError>>>,
    stream_request_index: u64,
    use_request_v2: bool,
) {
    let response_receiver = response_receivers.remove(&stream_request_index).unwrap();
    if fallback_to_transactions {
        utils::verify_new_transactions_or_outputs_with_proof(
            mock_client,
            response_receiver,
            use_request_v2,
            Some(expected_transaction_lists_with_proofs[stream_request_index as usize].clone()),
            None,
            expected_target_ledger_info.clone(),
            expected_persisted_auxiliary_infos[stream_request_index as usize].clone(),
        )
        .await;
    } else {
        utils::verify_new_transactions_or_outputs_with_proof(
            mock_client,
            response_receiver,
            use_request_v2,
            None,
            Some(expected_output_lists_with_proofs[stream_request_index as usize].clone()),
            expected_target_ledger_info.clone(),
            expected_persisted_auxiliary_infos[stream_request_index as usize].clone(),
        )
        .await;
    }
}
