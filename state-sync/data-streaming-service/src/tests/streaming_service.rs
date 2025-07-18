// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_notification::DataPayload,
    data_stream::DataStreamListener,
    error::Error,
    streaming_client::{
        new_streaming_service_client_listener_pair, DataStreamingClient, NotificationAndFeedback,
        NotificationFeedback, StreamingServiceClient,
    },
    streaming_service::DataStreamingService,
    tests::utils::{
        create_ledger_info, get_data_notification, initialize_logger, MockAptosDataClient,
        MAX_ADVERTISED_EPOCH_END, MAX_ADVERTISED_STATES, MAX_ADVERTISED_TRANSACTION,
        MAX_ADVERTISED_TRANSACTION_OUTPUT, MAX_REAL_EPOCH_END, MAX_REAL_TRANSACTION,
        MAX_REAL_TRANSACTION_OUTPUT, MIN_ADVERTISED_EPOCH_END, MIN_ADVERTISED_STATES,
        MIN_ADVERTISED_TRANSACTION, MIN_ADVERTISED_TRANSACTION_OUTPUT, TOTAL_NUM_STATE_VALUES,
    },
};
use aptos_config::config::{AptosDataClientConfig, DataStreamingServiceConfig};
use aptos_time_service::TimeService;
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    transaction::{TransactionListWithProof, TransactionOutputListWithProofV2},
};
use claims::{assert_le, assert_matches, assert_ok, assert_some};

macro_rules! unexpected_payload_type {
    ($received:expr) => {
        panic!("Unexpected payload type: {:?}", $received)
    };
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_state_values() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request a state value stream and get a data stream listener
    let mut stream_listener = streaming_client
        .get_all_state_values(MAX_ADVERTISED_STATES, None)
        .await
        .unwrap();

    // Verify that the stream listener receives all state value notifications
    verify_continuous_state_value_notifications(&mut stream_listener).await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_state_values_limited_chunks() {
    // Create a new streaming client and service where chunks may be truncated
    let streaming_client = create_streaming_client_and_service_with_chunk_limits();

    // Request a new state value stream starting at the next expected index
    let mut stream_listener = streaming_client
        .get_all_state_values(MAX_ADVERTISED_STATES, Some(0))
        .await
        .unwrap();

    // Verify that the stream listener receives all state value notifications
    verify_continuous_state_value_notifications(&mut stream_listener).await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_state_values_multiple_streams() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request a new state value stream starting at the next expected index.
    let mut next_expected_index = 0;
    let mut stream_listener = streaming_client
        .get_all_state_values(MAX_ADVERTISED_STATES, Some(next_expected_index))
        .await
        .unwrap();

    // Terminate and request new state value streams at increasing versions
    loop {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        match data_notification.data_payload {
            DataPayload::StateValuesWithProof(state_values_with_proof) => {
                // Verify the indices
                assert_eq!(state_values_with_proof.first_index, next_expected_index);

                // Update the next expected index
                next_expected_index += state_values_with_proof.raw_values.len() as u64;

                // Terminate the stream if we haven't reached the end
                if next_expected_index < TOTAL_NUM_STATE_VALUES {
                    // Terminate the stream
                    streaming_client
                        .terminate_stream_with_feedback(
                            stream_listener.data_stream_id,
                            Some(NotificationAndFeedback::new(
                                data_notification.notification_id,
                                NotificationFeedback::InvalidPayloadData,
                            )),
                        )
                        .await
                        .unwrap();

                    // Fetch a new stream
                    stream_listener = streaming_client
                        .get_all_state_values(MAX_ADVERTISED_STATES, Some(next_expected_index))
                        .await
                        .unwrap();
                }
            },
            DataPayload::EndOfStream => {
                assert_eq!(next_expected_index, TOTAL_NUM_STATE_VALUES);
                return; // We've reached the end
            },
            data_payload => unexpected_payload_type!(data_payload),
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_continuous_outputs() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request a continuous output stream and get a data stream listener
    let mut stream_listener = streaming_client
        .continuously_stream_transaction_outputs(
            MIN_ADVERTISED_TRANSACTION_OUTPUT - 1,
            MIN_ADVERTISED_EPOCH_END,
            None,
        )
        .await
        .unwrap();

    // Verify that the stream listener receives all output notifications
    verify_continuous_output_notifications(&mut stream_listener, false).await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_continuous_outputs_limited_chunks() {
    // Create a new streaming client and service where chunks may be truncated
    let streaming_client = create_streaming_client_and_service_with_chunk_limits();

    // Request a continuous output stream starting at the next expected version
    let mut stream_listener = streaming_client
        .continuously_stream_transaction_outputs(
            MIN_ADVERTISED_TRANSACTION_OUTPUT - 1,
            MIN_ADVERTISED_EPOCH_END,
            None,
        )
        .await
        .unwrap();

    // Verify that the stream listener receives all output notifications
    verify_continuous_output_notifications(&mut stream_listener, false).await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_continuous_outputs_target() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request a continuous output stream and get a data stream listener
    let target_version = MAX_ADVERTISED_TRANSACTION_OUTPUT - 551;
    let target = create_ledger_info(target_version, MAX_ADVERTISED_EPOCH_END, false);
    let mut stream_listener = streaming_client
        .continuously_stream_transaction_outputs(
            MIN_ADVERTISED_TRANSACTION_OUTPUT - 1,
            MIN_ADVERTISED_EPOCH_END,
            Some(target),
        )
        .await
        .unwrap();

    // Read the data notifications from the stream and verify the payloads
    let mut next_expected_epoch = MIN_ADVERTISED_EPOCH_END;
    let mut next_expected_version = MIN_ADVERTISED_TRANSACTION_OUTPUT;
    loop {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        match data_notification.data_payload {
            DataPayload::ContinuousTransactionOutputsWithProof(
                ledger_info_with_sigs,
                outputs_with_proofs,
            ) => {
                // Verify the epoch of the ledger info
                let ledger_info = ledger_info_with_sigs.ledger_info();
                assert_eq!(ledger_info.epoch(), next_expected_epoch);

                // Verify the output start version matches the expected version
                let first_output_version = outputs_with_proofs.get_first_output_version();
                assert_eq!(Some(next_expected_version), first_output_version);

                // Update the next expected version
                let num_outputs = outputs_with_proofs.get_num_outputs() as u64;
                next_expected_version += num_outputs;

                // Update epochs if we've hit the epoch end
                let last_output_version = first_output_version.unwrap() + num_outputs - 1;
                if ledger_info.version() == last_output_version && ledger_info.ends_epoch() {
                    next_expected_epoch += 1;
                }
            },
            DataPayload::EndOfStream => {
                return assert_eq!(next_expected_version, target_version + 1)
            },
            data_payload => unexpected_payload_type!(data_payload),
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_continuous_outputs_multiple_streams() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();
    let end_epoch = MIN_ADVERTISED_EPOCH_END + 5;

    // Request a continuous output stream starting at the next expected version
    let mut next_expected_version = MIN_ADVERTISED_TRANSACTION_OUTPUT;
    let mut next_expected_epoch = MIN_ADVERTISED_EPOCH_END;
    let mut stream_listener = streaming_client
        .continuously_stream_transaction_outputs(
            next_expected_version - 1,
            next_expected_epoch,
            None,
        )
        .await
        .unwrap();

    // Terminate and request new output streams at increasing versions
    loop {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        match data_notification.data_payload {
            DataPayload::ContinuousTransactionOutputsWithProof(
                ledger_info_with_sigs,
                outputs_with_proofs,
            ) => {
                // Verify the first output version
                let first_output_version = outputs_with_proofs.get_first_output_version();
                assert_eq!(Some(next_expected_version), first_output_version);

                // Update the next expected version
                let num_outputs = outputs_with_proofs.get_num_outputs() as u64;
                next_expected_version += num_outputs;

                // Update the next expected epoch if we've hit the epoch end
                let last_output_version = first_output_version.unwrap() + num_outputs - 1;
                let ledger_info = ledger_info_with_sigs.ledger_info();
                if ledger_info.version() == last_output_version && ledger_info.ends_epoch() {
                    next_expected_epoch += 1;
                }

                // Terminate the stream if we haven't reached the end
                if next_expected_version < MAX_ADVERTISED_TRANSACTION_OUTPUT {
                    // Terminate the stream
                    streaming_client
                        .terminate_stream_with_feedback(
                            stream_listener.data_stream_id,
                            Some(NotificationAndFeedback::new(
                                data_notification.notification_id,
                                NotificationFeedback::InvalidPayloadData,
                            )),
                        )
                        .await
                        .unwrap();

                    // Fetch a new stream
                    stream_listener = streaming_client
                        .continuously_stream_transaction_outputs(
                            next_expected_version - 1,
                            next_expected_epoch,
                            None,
                        )
                        .await
                        .unwrap();
                }

                // Check if we've reached the end
                if next_expected_epoch > end_epoch {
                    return;
                }
            },
            data_payload => unexpected_payload_type!(data_payload),
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_continuous_transactions() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request a continuous transaction stream and get a data stream listener
    let mut stream_listener = streaming_client
        .continuously_stream_transactions(
            MIN_ADVERTISED_TRANSACTION - 1,
            MIN_ADVERTISED_EPOCH_END,
            true,
            None,
        )
        .await
        .unwrap();

    // Verify that the stream listener receives all transaction notifications
    verify_continuous_transaction_notifications(&mut stream_listener, false).await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_continuous_transactions_limited_chunks() {
    // Create a new streaming client and service where chunks may be truncated
    let streaming_client = create_streaming_client_and_service_with_chunk_limits();

    // Request a continuous transaction stream and get a data stream listener
    let mut stream_listener = streaming_client
        .continuously_stream_transactions(
            MIN_ADVERTISED_TRANSACTION - 1,
            MIN_ADVERTISED_EPOCH_END,
            true,
            None,
        )
        .await
        .unwrap();

    // Verify that the stream listener receives all transaction notifications
    verify_continuous_transaction_notifications(&mut stream_listener, false).await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_continuous_transactions_target() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request a continuous transaction stream and get a data stream listener
    let target_version = MAX_ADVERTISED_TRANSACTION - 101;
    let target = create_ledger_info(target_version, MAX_ADVERTISED_EPOCH_END, true);
    let mut stream_listener = streaming_client
        .continuously_stream_transactions(
            MIN_ADVERTISED_TRANSACTION - 1,
            MIN_ADVERTISED_EPOCH_END,
            true,
            Some(target),
        )
        .await
        .unwrap();

    // Read the data notifications from the stream and verify the payloads
    let mut next_expected_epoch = MIN_ADVERTISED_EPOCH_END;
    let mut next_expected_version = MIN_ADVERTISED_TRANSACTION;
    loop {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        match data_notification.data_payload {
            DataPayload::ContinuousTransactionsWithProof(
                ledger_info_with_sigs,
                transactions_with_proof,
            ) => {
                // Verify the epoch of the ledger info
                let ledger_info = ledger_info_with_sigs.ledger_info();
                assert_eq!(ledger_info.epoch(), next_expected_epoch);

                // Verify the transaction start version matches the expected version
                let first_transaction_version =
                    transactions_with_proof.get_first_transaction_version();
                assert_eq!(Some(next_expected_version), first_transaction_version);

                // Verify the payload contains events
                assert!(transactions_with_proof.events.is_some());

                // Update the next expected version
                let num_transactions = transactions_with_proof.get_num_transactions() as u64;
                next_expected_version += num_transactions;

                // Update epochs if we've hit the epoch end
                let last_transaction_version =
                    first_transaction_version.unwrap() + num_transactions - 1;
                if ledger_info.version() == last_transaction_version && ledger_info.ends_epoch() {
                    next_expected_epoch += 1;
                }
            },
            DataPayload::EndOfStream => {
                return assert_eq!(next_expected_version, target_version + 1)
            },
            data_payload => unexpected_payload_type!(data_payload),
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_continuous_transactions_multiple_streams() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();
    let end_epoch = MIN_ADVERTISED_EPOCH_END + 5;

    // Request a continuous transaction stream starting at the next expected version
    let mut next_expected_version = MIN_ADVERTISED_TRANSACTION;
    let mut next_expected_epoch = MIN_ADVERTISED_EPOCH_END;
    let mut stream_listener = streaming_client
        .continuously_stream_transactions(
            next_expected_version - 1,
            next_expected_epoch,
            true,
            None,
        )
        .await
        .unwrap();

    // Terminate and request new transaction streams at increasing versions
    loop {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        match data_notification.data_payload {
            DataPayload::ContinuousTransactionsWithProof(
                ledger_info_with_sigs,
                transactions_with_proofs,
            ) => {
                // Verify the first transaction version
                let first_transaction_version =
                    transactions_with_proofs.get_first_transaction_version();
                assert_eq!(Some(next_expected_version), first_transaction_version);

                // Update the next expected version
                let num_transactions = transactions_with_proofs.get_num_transactions() as u64;
                next_expected_version += num_transactions;

                // Update the next expected epoch if we've hit the epoch end
                let last_transaction_version =
                    first_transaction_version.unwrap() + num_transactions - 1;
                let ledger_info = ledger_info_with_sigs.ledger_info();
                if ledger_info.version() == last_transaction_version && ledger_info.ends_epoch() {
                    next_expected_epoch += 1;
                }

                // Terminate the stream if we haven't reached the end
                if next_expected_version < MAX_ADVERTISED_TRANSACTION_OUTPUT {
                    // Terminate the stream
                    streaming_client
                        .terminate_stream_with_feedback(
                            stream_listener.data_stream_id,
                            Some(NotificationAndFeedback::new(
                                data_notification.notification_id,
                                NotificationFeedback::InvalidPayloadData,
                            )),
                        )
                        .await
                        .unwrap();

                    // Fetch a new stream
                    stream_listener = streaming_client
                        .continuously_stream_transactions(
                            next_expected_version - 1,
                            next_expected_epoch,
                            true,
                            None,
                        )
                        .await
                        .unwrap();
                }

                // Check if we've reached the end
                if next_expected_epoch > end_epoch {
                    return;
                }
            },
            data_payload => unexpected_payload_type!(data_payload),
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_continuous_transactions_or_outputs() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request a continuous transaction or output stream and get a data stream listener
    let mut stream_listener = streaming_client
        .continuously_stream_transactions_or_outputs(
            MIN_ADVERTISED_TRANSACTION - 1,
            MIN_ADVERTISED_EPOCH_END,
            true,
            None,
        )
        .await
        .unwrap();

    // Verify that the stream listener receives all transaction or output notifications
    verify_continuous_transaction_or_output_notifications(&mut stream_listener, false).await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_continuous_transactions_or_outputs_limited_chunks() {
    // Create a new streaming client and service where chunks may be truncated
    let streaming_client = create_streaming_client_and_service_with_chunk_limits();

    // Request a continuous transaction or output stream and get a data stream listener
    let mut stream_listener = streaming_client
        .continuously_stream_transactions_or_outputs(
            MIN_ADVERTISED_TRANSACTION - 1,
            MIN_ADVERTISED_EPOCH_END,
            true,
            None,
        )
        .await
        .unwrap();

    // Verify that the stream listener receives all transaction or output notifications
    verify_continuous_transaction_or_output_notifications(&mut stream_listener, false).await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_continuous_transactions_or_outputs_target() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request a continuous transaction or output stream and get a data stream listener
    let target_version = MAX_ADVERTISED_TRANSACTION - 101;
    let target = create_ledger_info(target_version, MAX_ADVERTISED_EPOCH_END, true);
    let mut stream_listener = streaming_client
        .continuously_stream_transactions_or_outputs(
            MIN_ADVERTISED_TRANSACTION - 1,
            MIN_ADVERTISED_EPOCH_END,
            true,
            Some(target),
        )
        .await
        .unwrap();

    // Read the data notifications from the stream and verify the payloads
    let mut next_expected_epoch = MIN_ADVERTISED_EPOCH_END;
    let mut next_expected_version = MIN_ADVERTISED_TRANSACTION;
    loop {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();

        // Extract the ledger info and transactions or outputs with proof
        let (ledger_info_with_sigs, transactions_with_proof, outputs_with_proof) =
            match data_notification.data_payload {
                DataPayload::ContinuousTransactionsWithProof(
                    ledger_info_with_sigs,
                    transactions_with_proof,
                ) => (ledger_info_with_sigs, Some(transactions_with_proof), None),
                DataPayload::ContinuousTransactionOutputsWithProof(
                    ledger_info_with_sigs,
                    outputs_with_proof,
                ) => (ledger_info_with_sigs, None, Some(outputs_with_proof)),
                DataPayload::EndOfStream => {
                    return assert_eq!(next_expected_version, target_version + 1)
                },
                data_payload => unexpected_payload_type!(data_payload),
            };

        // Verify the epoch of the ledger info
        let ledger_info = ledger_info_with_sigs.ledger_info();
        assert_eq!(ledger_info.epoch(), next_expected_epoch);

        // Verify the transactions or outputs start version matches the expected version
        let first_version = if transactions_with_proof.is_some() {
            transactions_with_proof
                .clone()
                .unwrap()
                .get_first_transaction_version()
        } else {
            outputs_with_proof
                .clone()
                .unwrap()
                .get_first_output_version()
        };
        assert_eq!(Some(next_expected_version), first_version);

        // Verify the payload contains events
        if transactions_with_proof.is_some() {
            assert_some!(transactions_with_proof.clone().unwrap().events);
        }

        // Update the next expected version
        let num_transactions = if transactions_with_proof.is_some() {
            transactions_with_proof
                .clone()
                .unwrap()
                .get_num_transactions() as u64
        } else {
            outputs_with_proof.clone().unwrap().get_num_outputs() as u64
        };
        next_expected_version += num_transactions;

        // Update epochs if we've hit the epoch end
        let last_transaction_version = first_version.unwrap() + num_transactions - 1;
        if ledger_info.version() == last_transaction_version && ledger_info.ends_epoch() {
            next_expected_epoch += 1;
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_continuous_transactions_or_outputs_multiple_streams() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();
    let end_epoch = MIN_ADVERTISED_EPOCH_END + 5;

    // Request a continuous transaction or output stream starting at the next expected version
    let mut next_expected_version = MIN_ADVERTISED_TRANSACTION;
    let mut next_expected_epoch = MIN_ADVERTISED_EPOCH_END;
    let mut stream_listener = streaming_client
        .continuously_stream_transactions_or_outputs(
            next_expected_version - 1,
            next_expected_epoch,
            true,
            None,
        )
        .await
        .unwrap();

    // Terminate and request new transaction or output streams at increasing versions
    loop {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();

        // Extract the ledger info and transactions or outputs with proof
        let (ledger_info_with_sigs, transactions_with_proof, outputs_with_proof) =
            match data_notification.data_payload {
                DataPayload::ContinuousTransactionsWithProof(
                    ledger_info_with_sigs,
                    transactions_with_proof,
                ) => (ledger_info_with_sigs, Some(transactions_with_proof), None),
                DataPayload::ContinuousTransactionOutputsWithProof(
                    ledger_info_with_sigs,
                    outputs_with_proof,
                ) => (ledger_info_with_sigs, None, Some(outputs_with_proof)),
                data_payload => unexpected_payload_type!(data_payload),
            };

        // Verify the first transaction or output version
        let first_version = if transactions_with_proof.is_some() {
            transactions_with_proof
                .clone()
                .unwrap()
                .get_first_transaction_version()
        } else {
            outputs_with_proof
                .clone()
                .unwrap()
                .get_first_output_version()
        };
        assert_eq!(Some(next_expected_version), first_version);

        // Update the next expected version
        let num_transactions = if transactions_with_proof.is_some() {
            transactions_with_proof
                .clone()
                .unwrap()
                .get_num_transactions() as u64
        } else {
            outputs_with_proof.clone().unwrap().get_num_outputs() as u64
        };
        next_expected_version += num_transactions;

        // Update the next expected epoch if we've hit the epoch end
        let last_transaction_version = first_version.unwrap() + num_transactions - 1;
        let ledger_info = ledger_info_with_sigs.ledger_info();
        if ledger_info.version() == last_transaction_version && ledger_info.ends_epoch() {
            next_expected_epoch += 1;
        }

        // Terminate the stream if we haven't reached the end
        if next_expected_version < MAX_ADVERTISED_TRANSACTION_OUTPUT {
            // Terminate the stream
            streaming_client
                .terminate_stream_with_feedback(
                    stream_listener.data_stream_id,
                    Some(NotificationAndFeedback::new(
                        data_notification.notification_id,
                        NotificationFeedback::InvalidPayloadData,
                    )),
                )
                .await
                .unwrap();

            // Fetch a new stream
            stream_listener = streaming_client
                .continuously_stream_transactions_or_outputs(
                    next_expected_version - 1,
                    next_expected_epoch,
                    true,
                    None,
                )
                .await
                .unwrap();
        }

        // Check if we've reached the end
        if next_expected_epoch > end_epoch {
            return;
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_epoch_ending() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request an epoch ending stream and get a data stream listener
    let mut stream_listener = streaming_client
        .get_all_epoch_ending_ledger_infos(MIN_ADVERTISED_EPOCH_END)
        .await
        .unwrap();

    // Verify that the stream listener receives all epoch ending notifications
    verify_continuous_epoch_ending_notifications(&mut stream_listener).await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_epoch_ending_limited_chunks() {
    // Create a new streaming client and service where chunks may be truncated
    let streaming_client = create_streaming_client_and_service_with_chunk_limits();

    // Request a new epoch ending stream starting at the next expected index.
    let mut stream_listener = streaming_client
        .get_all_epoch_ending_ledger_infos(MIN_ADVERTISED_EPOCH_END)
        .await
        .unwrap();

    // Verify that the stream listener receives all epoch ending notifications
    verify_continuous_epoch_ending_notifications(&mut stream_listener).await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_epoch_ending_multiple_streams() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request a new epoch ending stream starting at the next expected index.
    let mut next_expected_epoch = MIN_ADVERTISED_EPOCH_END;
    let mut stream_listener = streaming_client
        .get_all_epoch_ending_ledger_infos(next_expected_epoch)
        .await
        .unwrap();

    // Terminate and request new epoch ending streams at increasing versions
    loop {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        match data_notification.data_payload {
            DataPayload::EpochEndingLedgerInfos(ledger_infos_with_sigs) => {
                // Update the next expected epoch
                next_expected_epoch += ledger_infos_with_sigs.len() as u64;

                // Terminate the stream if we haven't reached the end
                if next_expected_epoch < MAX_ADVERTISED_EPOCH_END {
                    // Terminate the stream
                    streaming_client
                        .terminate_stream_with_feedback(
                            stream_listener.data_stream_id,
                            Some(NotificationAndFeedback::new(
                                data_notification.notification_id,
                                NotificationFeedback::InvalidPayloadData,
                            )),
                        )
                        .await
                        .unwrap();

                    // Fetch a new stream
                    stream_listener = streaming_client
                        .get_all_epoch_ending_ledger_infos(next_expected_epoch)
                        .await
                        .unwrap();
                }
            },
            DataPayload::EndOfStream => {
                assert_eq!(next_expected_epoch, MAX_ADVERTISED_EPOCH_END + 1);
                return; // We've reached the end
            },
            data_payload => unexpected_payload_type!(data_payload),
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_optimistic_fetch_outputs() {
    // Create a new streaming client and service
    let enable_subscription_streaming = false;
    let streaming_client =
        create_streaming_client_and_service_with_data_delay(enable_subscription_streaming);

    // Request a continuous output stream and get a data stream listener
    let mut stream_listener = streaming_client
        .continuously_stream_transaction_outputs(
            MIN_ADVERTISED_TRANSACTION_OUTPUT - 1,
            MIN_ADVERTISED_EPOCH_END,
            None,
        )
        .await
        .unwrap();

    // Verify that the stream listener receives all output notifications
    verify_continuous_output_notifications(&mut stream_listener, true).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_optimistic_fetch_transactions() {
    // Create a new streaming client and service
    let enable_subscription_streaming = false;
    let streaming_client =
        create_streaming_client_and_service_with_data_delay(enable_subscription_streaming);

    // Request a continuous transaction stream and get a data stream listener
    let mut stream_listener = streaming_client
        .continuously_stream_transactions(
            MIN_ADVERTISED_TRANSACTION - 1,
            MIN_ADVERTISED_EPOCH_END,
            false,
            None,
        )
        .await
        .unwrap();

    // Verify that the stream listener receives all transaction notifications
    verify_continuous_transaction_notifications(&mut stream_listener, true).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_optimistic_fetch_transactions_or_outputs() {
    // Create a new streaming client and service
    let enable_subscription_streaming = false;
    let streaming_client =
        create_streaming_client_and_service_with_data_delay(enable_subscription_streaming);

    // Request a continuous transaction or output stream and get a data stream listener
    let mut stream_listener = streaming_client
        .continuously_stream_transactions_or_outputs(
            MIN_ADVERTISED_TRANSACTION - 1,
            MIN_ADVERTISED_EPOCH_END,
            false,
            None,
        )
        .await
        .unwrap();

    // Verify that the stream listener receives all transaction or output notifications
    verify_continuous_transaction_or_output_notifications(&mut stream_listener, true).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_subscribe_outputs() {
    // Create a new streaming client and service
    let enable_subscription_streaming = true;
    let streaming_client =
        create_streaming_client_and_service_with_data_delay(enable_subscription_streaming);

    // Request a continuous output stream and get a data stream listener
    let mut stream_listener = streaming_client
        .continuously_stream_transaction_outputs(
            MIN_ADVERTISED_TRANSACTION_OUTPUT - 1,
            MIN_ADVERTISED_EPOCH_END,
            None,
        )
        .await
        .unwrap();

    // Verify that the stream listener receives all output notifications
    verify_continuous_output_notifications(&mut stream_listener, true).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_subscribe_outputs_small_max() {
    // Create a data streaming service config with subscription
    // syncing enabled and a small max consecutive subscriptions.
    let enable_subscription_streaming = true;
    let streaming_service_config = DataStreamingServiceConfig {
        enable_subscription_streaming,
        max_num_consecutive_subscriptions: 2,
        ..Default::default()
    };

    // Create a new streaming client and service
    let data_beyond_highest_advertised = false;
    let streaming_client = create_streaming_client_and_spawn_server(
        Some(streaming_service_config),
        data_beyond_highest_advertised,
        false,
        false,
        enable_subscription_streaming,
    );

    // Request a continuous output stream and get a data stream listener
    let mut stream_listener = streaming_client
        .continuously_stream_transaction_outputs(
            MIN_ADVERTISED_TRANSACTION_OUTPUT - 1,
            MIN_ADVERTISED_EPOCH_END,
            None,
        )
        .await
        .unwrap();

    // Verify that the stream listener receives all output notifications
    verify_continuous_output_notifications(&mut stream_listener, data_beyond_highest_advertised)
        .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_subscribe_transactions() {
    // Create a new streaming client and service
    let enable_subscription_streaming = true;
    let streaming_client =
        create_streaming_client_and_service_with_data_delay(enable_subscription_streaming);

    // Request a continuous transaction stream and get a data stream listener
    let mut stream_listener = streaming_client
        .continuously_stream_transactions(
            MIN_ADVERTISED_TRANSACTION - 1,
            MIN_ADVERTISED_EPOCH_END,
            false,
            None,
        )
        .await
        .unwrap();

    // Verify that the stream listener receives all transaction notifications
    verify_continuous_transaction_notifications(&mut stream_listener, true).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_subscribe_transactions_small_max() {
    // Create a data streaming service config with subscription
    // syncing enabled and a small max consecutive subscriptions.
    let enable_subscription_streaming = true;
    let streaming_service_config = DataStreamingServiceConfig {
        enable_subscription_streaming,
        max_num_consecutive_subscriptions: 2,
        ..Default::default()
    };

    // Create a new streaming client and service
    let data_beyond_highest_advertised = true;
    let streaming_client = create_streaming_client_and_spawn_server(
        Some(streaming_service_config),
        true,
        false,
        false,
        enable_subscription_streaming,
    );

    // Request a continuous transaction stream and get a data stream listener
    let mut stream_listener = streaming_client
        .continuously_stream_transactions(
            MIN_ADVERTISED_TRANSACTION - 1,
            MIN_ADVERTISED_EPOCH_END,
            false,
            None,
        )
        .await
        .unwrap();

    // Verify that the stream listener receives all transaction notifications
    verify_continuous_transaction_notifications(
        &mut stream_listener,
        data_beyond_highest_advertised,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_subscribe_transactions_or_outputs() {
    // Create a new streaming client and service
    let enable_subscription_streaming = true;
    let streaming_client =
        create_streaming_client_and_service_with_data_delay(enable_subscription_streaming);

    // Request a continuous transaction or output stream and get a data stream listener
    let mut stream_listener = streaming_client
        .continuously_stream_transactions_or_outputs(
            MIN_ADVERTISED_TRANSACTION - 1,
            MIN_ADVERTISED_EPOCH_END,
            false,
            None,
        )
        .await
        .unwrap();

    // Verify that the stream listener receives all transaction or output notifications
    verify_continuous_transaction_or_output_notifications(&mut stream_listener, true).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_subscribe_transactions_or_outputs_small_max() {
    // Create a data streaming service config with subscription
    // syncing enabled and a small max consecutive subscriptions.
    let enable_subscription_streaming = true;
    let streaming_service_config = DataStreamingServiceConfig {
        enable_subscription_streaming,
        max_num_consecutive_subscriptions: 2,
        ..Default::default()
    };

    // Create a new streaming client and service
    let data_beyond_highest_advertised = true;
    let streaming_client = create_streaming_client_and_spawn_server(
        Some(streaming_service_config),
        data_beyond_highest_advertised,
        false,
        false,
        enable_subscription_streaming,
    );

    // Request a continuous transaction or output stream and get a data stream listener
    let mut stream_listener = streaming_client
        .continuously_stream_transactions_or_outputs(
            MIN_ADVERTISED_TRANSACTION - 1,
            MIN_ADVERTISED_EPOCH_END,
            false,
            None,
        )
        .await
        .unwrap();

    // Verify that the stream listener receives all transaction or output notifications
    verify_continuous_transaction_or_output_notifications(
        &mut stream_listener,
        data_beyond_highest_advertised,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_transaction_outputs() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request a transaction output stream and get a data stream listener
    let mut stream_listener = streaming_client
        .get_all_transaction_outputs(
            MIN_ADVERTISED_TRANSACTION_OUTPUT,
            MAX_ADVERTISED_TRANSACTION_OUTPUT,
            MAX_ADVERTISED_TRANSACTION_OUTPUT,
        )
        .await
        .unwrap();

    // Verify that the stream listener receives all output notifications
    verify_output_notifications(
        &mut stream_listener,
        MIN_ADVERTISED_TRANSACTION_OUTPUT,
        MAX_ADVERTISED_TRANSACTION_OUTPUT,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_transaction_outputs_limited_chunks() {
    // Create a new streaming client and service where chunks may be truncated
    let streaming_client = create_streaming_client_and_service_with_chunk_limits();

    // Request a transaction output stream starting at the next expected version
    let mut stream_listener = streaming_client
        .get_all_transaction_outputs(
            MIN_ADVERTISED_TRANSACTION_OUTPUT,
            MAX_ADVERTISED_TRANSACTION_OUTPUT,
            MAX_ADVERTISED_TRANSACTION_OUTPUT,
        )
        .await
        .unwrap();

    // Verify that the stream listener receives all output notifications
    verify_output_notifications(
        &mut stream_listener,
        MIN_ADVERTISED_TRANSACTION_OUTPUT,
        MAX_ADVERTISED_TRANSACTION_OUTPUT,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_transaction_outputs_multiple_streams() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request a transaction output stream starting at the next expected version
    let mut next_expected_version = MIN_ADVERTISED_TRANSACTION_OUTPUT;
    let mut stream_listener = streaming_client
        .get_all_transaction_outputs(
            next_expected_version,
            MAX_ADVERTISED_TRANSACTION_OUTPUT,
            MAX_ADVERTISED_TRANSACTION_OUTPUT,
        )
        .await
        .unwrap();

    // Terminate and request new output streams at increasing versions
    loop {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        match data_notification.data_payload {
            DataPayload::TransactionOutputsWithProof(outputs_with_proof) => {
                // Verify the first transaction output version
                let first_output_version = outputs_with_proof.get_first_output_version();
                assert_eq!(Some(next_expected_version), first_output_version);

                // Update the next expected version
                next_expected_version += outputs_with_proof.get_num_outputs() as u64;

                // Terminate the stream if we haven't reached the end
                if next_expected_version < MAX_ADVERTISED_TRANSACTION_OUTPUT {
                    // Terminate the stream
                    streaming_client
                        .terminate_stream_with_feedback(
                            stream_listener.data_stream_id,
                            Some(NotificationAndFeedback::new(
                                data_notification.notification_id,
                                NotificationFeedback::InvalidPayloadData,
                            )),
                        )
                        .await
                        .unwrap();

                    // Fetch a new stream
                    stream_listener = streaming_client
                        .get_all_transaction_outputs(
                            next_expected_version,
                            MAX_ADVERTISED_TRANSACTION_OUTPUT,
                            MAX_ADVERTISED_TRANSACTION_OUTPUT,
                        )
                        .await
                        .unwrap();
                }
            },
            DataPayload::EndOfStream => {
                assert_eq!(next_expected_version, MAX_ADVERTISED_TRANSACTION_OUTPUT + 1);
                return; // We've reached the end
            },
            data_payload => unexpected_payload_type!(data_payload),
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_transactions() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request a transaction stream (with events) and get a data stream listener
    let mut stream_listener = streaming_client
        .get_all_transactions(
            MIN_ADVERTISED_TRANSACTION,
            MAX_ADVERTISED_TRANSACTION,
            MAX_ADVERTISED_TRANSACTION,
            true,
        )
        .await
        .unwrap();

    // Verify that the stream listener receives all transaction notifications
    verify_transaction_notifications(
        &mut stream_listener,
        MIN_ADVERTISED_TRANSACTION,
        MAX_ADVERTISED_TRANSACTION,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_transactions_limited_chunks() {
    // Create a new streaming client and service where chunks may be truncated
    let streaming_client = create_streaming_client_and_service_with_chunk_limits();

    // Request a transaction stream (without events) and get a data stream listener
    let mut stream_listener = streaming_client
        .get_all_transactions(
            MIN_ADVERTISED_TRANSACTION,
            MAX_ADVERTISED_TRANSACTION,
            MAX_ADVERTISED_TRANSACTION,
            false,
        )
        .await
        .unwrap();

    // Verify that the stream listener receives all transaction notifications
    verify_transaction_notifications(
        &mut stream_listener,
        MIN_ADVERTISED_TRANSACTION,
        MAX_ADVERTISED_TRANSACTION,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_transactions_multiple_streams() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request a transaction stream starting at the next expected version
    let mut next_expected_version = MIN_ADVERTISED_TRANSACTION;
    let mut stream_listener = streaming_client
        .get_all_transactions(
            next_expected_version,
            MAX_ADVERTISED_TRANSACTION,
            MAX_ADVERTISED_TRANSACTION,
            true,
        )
        .await
        .unwrap();

    // Terminate and request new transaction streams at increasing versions
    loop {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        match data_notification.data_payload {
            DataPayload::TransactionsWithProof(transactions_with_proof) => {
                // Verify the first transaction version
                let first_transaction_version =
                    transactions_with_proof.get_first_transaction_version();
                assert_eq!(Some(next_expected_version), first_transaction_version);

                // Update the next expected version
                next_expected_version += transactions_with_proof.get_num_transactions() as u64;

                // Terminate the stream if we haven't reached the end
                if next_expected_version < MAX_ADVERTISED_TRANSACTION {
                    // Terminate the stream
                    streaming_client
                        .terminate_stream_with_feedback(
                            stream_listener.data_stream_id,
                            Some(NotificationAndFeedback::new(
                                data_notification.notification_id,
                                NotificationFeedback::InvalidPayloadData,
                            )),
                        )
                        .await
                        .unwrap();

                    // Fetch a new stream
                    stream_listener = streaming_client
                        .get_all_transactions(
                            next_expected_version,
                            MAX_ADVERTISED_TRANSACTION,
                            MAX_ADVERTISED_TRANSACTION,
                            true,
                        )
                        .await
                        .unwrap();
                }
            },
            DataPayload::EndOfStream => {
                assert_eq!(next_expected_version, MAX_ADVERTISED_TRANSACTION + 1);
                return; // We've reached the end
            },
            data_payload => unexpected_payload_type!(data_payload),
        }
    }
}

#[tokio::test]
async fn test_stream_states() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request a state value stream and verify we get a data stream listener
    let result = streaming_client
        .get_all_state_values(MAX_ADVERTISED_STATES - 1, None)
        .await;
    assert_ok!(result);

    // Request a stream where states are missing (we are lower than advertised)
    let result = streaming_client
        .get_all_state_values(MIN_ADVERTISED_STATES - 1, None)
        .await;
    assert_matches!(result, Err(Error::DataIsUnavailable(_)));

    // Request a stream where states are missing (we are higher than advertised)
    let result = streaming_client
        .get_all_state_values(MAX_ADVERTISED_EPOCH_END + 1, None)
        .await;
    assert_matches!(result, Err(Error::DataIsUnavailable(_)));
}

#[tokio::test]
async fn test_stream_continuous_outputs() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request a continuous output stream and verify we get a data stream listener
    let result = streaming_client
        .continuously_stream_transaction_outputs(
            MIN_ADVERTISED_TRANSACTION_OUTPUT - 1,
            MIN_ADVERTISED_EPOCH_END,
            None,
        )
        .await;
    assert_ok!(result);

    // Request a stream where data is missing (we are lower than advertised)
    let result = streaming_client
        .continuously_stream_transaction_outputs(
            MIN_ADVERTISED_TRANSACTION_OUTPUT - 2,
            MIN_ADVERTISED_EPOCH_END,
            None,
        )
        .await;
    assert_matches!(result, Err(Error::DataIsUnavailable(_)));

    // Request a stream where data is missing (we are higher than advertised)
    let result = streaming_client
        .continuously_stream_transaction_outputs(
            MAX_ADVERTISED_TRANSACTION_OUTPUT + 1,
            MIN_ADVERTISED_EPOCH_END,
            None,
        )
        .await;
    assert_matches!(result, Err(Error::DataIsUnavailable(_)));
}

#[tokio::test]
async fn test_stream_continuous_outputs_target() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request a continuous output stream and verify we get a data stream listener
    let result = streaming_client
        .continuously_stream_transaction_outputs(
            MIN_ADVERTISED_TRANSACTION_OUTPUT - 1,
            MIN_ADVERTISED_EPOCH_END,
            Some(create_ledger_info(
                MAX_ADVERTISED_TRANSACTION_OUTPUT,
                MAX_ADVERTISED_EPOCH_END,
                true,
            )),
        )
        .await;
    assert_ok!(result);

    // Request a stream where data is missing (the target version is higher than
    // advertised) and verify the stream is still created. This covers the case
    // where the advertised data is lagging behind the target requested by consensus.
    let result = streaming_client
        .continuously_stream_transaction_outputs(
            MIN_ADVERTISED_TRANSACTION_OUTPUT - 1,
            MIN_ADVERTISED_EPOCH_END,
            Some(create_ledger_info(
                MAX_ADVERTISED_TRANSACTION_OUTPUT + 1,
                MAX_ADVERTISED_EPOCH_END,
                true,
            )),
        )
        .await;
    assert_ok!(result);
}

#[tokio::test]
async fn test_stream_continuous_transactions() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request a continuous transaction stream and verify we get a data stream listener
    let result = streaming_client
        .continuously_stream_transactions(
            MIN_ADVERTISED_TRANSACTION - 1,
            MIN_ADVERTISED_EPOCH_END,
            true,
            None,
        )
        .await;
    assert_ok!(result);

    // Request a stream where data is missing (we are lower than advertised)
    let result = streaming_client
        .continuously_stream_transactions(
            MIN_ADVERTISED_TRANSACTION - 2,
            MIN_ADVERTISED_EPOCH_END,
            true,
            None,
        )
        .await;
    assert_matches!(result, Err(Error::DataIsUnavailable(_)));

    // Request a stream where data is missing (we are higher than advertised)
    let result = streaming_client
        .continuously_stream_transactions(
            MAX_ADVERTISED_TRANSACTION + 1,
            MIN_ADVERTISED_EPOCH_END,
            true,
            None,
        )
        .await;
    assert_matches!(result, Err(Error::DataIsUnavailable(_)));
}

#[tokio::test]
async fn test_stream_continuous_transactions_target() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request a continuous transaction stream and verify we get a data stream listener
    let result = streaming_client
        .continuously_stream_transactions(
            MIN_ADVERTISED_TRANSACTION - 1,
            MIN_ADVERTISED_EPOCH_END,
            true,
            Some(create_ledger_info(
                MAX_ADVERTISED_TRANSACTION,
                MAX_ADVERTISED_EPOCH_END,
                true,
            )),
        )
        .await;
    assert_ok!(result);

    // Request a stream where data is missing (the target version is higher than
    // advertised) and verify the stream is still created. This covers the case
    // where the advertised data is lagging behind the target requested by consensus.
    let result = streaming_client
        .continuously_stream_transactions(
            MIN_ADVERTISED_TRANSACTION - 1,
            MIN_ADVERTISED_EPOCH_END,
            true,
            Some(create_ledger_info(
                MAX_ADVERTISED_TRANSACTION + 1,
                MAX_ADVERTISED_EPOCH_END,
                false,
            )),
        )
        .await;
    assert_ok!(result);
}

#[tokio::test]
async fn test_stream_epoch_ending() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request an epoch ending stream and verify we get a data stream listener
    let result = streaming_client
        .get_all_epoch_ending_ledger_infos(MIN_ADVERTISED_EPOCH_END)
        .await;
    assert_ok!(result);

    // Request a stream where epoch data is missing (we are lower than advertised)
    let result = streaming_client
        .get_all_epoch_ending_ledger_infos(MIN_ADVERTISED_EPOCH_END - 1)
        .await;
    assert_matches!(result, Err(Error::DataIsUnavailable(_)));

    // Request a stream where epoch data is missing (we are higher than advertised)
    let result = streaming_client
        .get_all_epoch_ending_ledger_infos(MAX_ADVERTISED_EPOCH_END + 1)
        .await;
    assert_matches!(result, Err(Error::DataIsUnavailable(_)));
}

#[tokio::test]
async fn test_stream_transaction_outputs() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request a transaction output stream and verify we get a data stream listener
    let result = streaming_client
        .get_all_transaction_outputs(
            MIN_ADVERTISED_TRANSACTION_OUTPUT,
            MAX_ADVERTISED_TRANSACTION_OUTPUT,
            MAX_ADVERTISED_TRANSACTION_OUTPUT,
        )
        .await;
    assert_ok!(result);

    // Request a stream where outputs are missing (we are higher than advertised)
    let result = streaming_client
        .get_all_transaction_outputs(
            MIN_ADVERTISED_TRANSACTION_OUTPUT,
            MAX_ADVERTISED_TRANSACTION_OUTPUT + 1,
            MAX_ADVERTISED_TRANSACTION_OUTPUT + 1,
        )
        .await;
    assert_matches!(result, Err(Error::DataIsUnavailable(_)));

    // Request a stream where outputs are missing (we are lower than advertised)
    let result = streaming_client
        .get_all_transaction_outputs(
            MIN_ADVERTISED_TRANSACTION_OUTPUT - 1,
            MAX_ADVERTISED_TRANSACTION_OUTPUT,
            MAX_ADVERTISED_TRANSACTION_OUTPUT,
        )
        .await;
    assert_matches!(result, Err(Error::DataIsUnavailable(_)));
}

#[tokio::test]
async fn test_stream_transactions() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request a transaction stream and verify we get a data stream listener
    let result = streaming_client
        .get_all_transactions(
            MIN_ADVERTISED_TRANSACTION,
            MAX_ADVERTISED_TRANSACTION,
            MAX_ADVERTISED_TRANSACTION,
            true,
        )
        .await;
    assert_ok!(result);

    // Request a stream where transactions are missing (we are higher than advertised)
    let result = streaming_client
        .get_all_transactions(
            MIN_ADVERTISED_TRANSACTION,
            MAX_ADVERTISED_TRANSACTION + 1,
            MAX_ADVERTISED_TRANSACTION,
            true,
        )
        .await;
    assert_matches!(result, Err(Error::DataIsUnavailable(_)));

    // Request a stream where transactions is missing (we are lower than advertised)
    let result = streaming_client
        .get_all_transactions(
            MIN_ADVERTISED_TRANSACTION - 1,
            MAX_ADVERTISED_TRANSACTION,
            MAX_ADVERTISED_TRANSACTION,
            true,
        )
        .await;
    assert_matches!(result, Err(Error::DataIsUnavailable(_)));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_terminate_complete_stream() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request an output stream
    let mut stream_listener = streaming_client
        .get_all_transaction_outputs(
            MIN_ADVERTISED_TRANSACTION_OUTPUT,
            MAX_ADVERTISED_TRANSACTION_OUTPUT,
            MAX_ADVERTISED_TRANSACTION_OUTPUT,
        )
        .await
        .unwrap();

    // Wait until the stream is complete and then terminate it
    loop {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        match data_notification.data_payload {
            DataPayload::EndOfStream => {
                let result = streaming_client
                    .terminate_stream_with_feedback(
                        stream_listener.data_stream_id,
                        Some(NotificationAndFeedback::new(
                            data_notification.notification_id,
                            NotificationFeedback::InvalidPayloadData,
                        )),
                    )
                    .await;
                assert_ok!(result);
                return;
            },
            DataPayload::TransactionOutputsWithProof(_) => {},
            data_payload => unexpected_payload_type!(data_payload),
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
#[should_panic(expected = "SelectNextSome polled after terminated")]
async fn test_terminate_stream() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request a state value stream
    let mut stream_listener = streaming_client
        .get_all_state_values(MAX_ADVERTISED_STATES - 1, None)
        .await
        .unwrap();

    // Fetch the first state value notification and then terminate the stream
    let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
    match data_notification.data_payload {
        DataPayload::StateValuesWithProof(_) => {},
        data_payload => unexpected_payload_type!(data_payload),
    }

    // Terminate the stream
    let result = streaming_client
        .terminate_stream_with_feedback(
            stream_listener.data_stream_id,
            Some(NotificationAndFeedback::new(
                data_notification.notification_id,
                NotificationFeedback::InvalidPayloadData,
            )),
        )
        .await;
    assert_ok!(result);

    // Verify the streaming service has removed the stream (polling should panic)
    loop {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        match data_notification.data_payload {
            DataPayload::StateValuesWithProof(_) => {},
            DataPayload::EndOfStream => panic!("The stream should have terminated!"),
            data_payload => unexpected_payload_type!(data_payload),
        }
    }
}

fn create_streaming_client_and_service() -> StreamingServiceClient {
    create_streaming_client_and_spawn_server(None, false, false, false, false)
}

fn create_streaming_client_and_service_with_data_delay(
    enable_subscription_streaming: bool,
) -> StreamingServiceClient {
    create_streaming_client_and_spawn_server(
        None,
        true,
        false,
        false,
        enable_subscription_streaming,
    )
}

fn create_streaming_client_and_service_with_chunk_limits() -> StreamingServiceClient {
    create_streaming_client_and_spawn_server(None, false, true, true, false)
}

fn create_streaming_client_and_spawn_server(
    data_streaming_service_config: Option<DataStreamingServiceConfig>,
    data_beyond_highest_advertised: bool,
    limit_chunk_sizes: bool,
    skip_emulate_network_latencies: bool,
    enable_subscription_streaming: bool,
) -> StreamingServiceClient {
    let (client, service) = create_streaming_client_and_server(
        data_streaming_service_config,
        data_beyond_highest_advertised,
        limit_chunk_sizes,
        skip_emulate_network_latencies,
        enable_subscription_streaming,
    );
    tokio::spawn(service.start_service());
    client
}

pub fn create_streaming_client_and_server(
    data_streaming_service_config: Option<DataStreamingServiceConfig>,
    data_beyond_highest_advertised: bool,
    limit_chunk_sizes: bool,
    skip_emulate_network_latencies: bool,
    enable_subscription_streaming: bool,
) -> (
    StreamingServiceClient,
    DataStreamingService<MockAptosDataClient>,
) {
    initialize_logger();

    // Create a new streaming client and listener
    let (streaming_client, streaming_service_listener) =
        new_streaming_service_client_listener_pair();

    // Create a mock data client
    let aptos_data_client_config = AptosDataClientConfig::default();
    let aptos_data_client = MockAptosDataClient::new(
        aptos_data_client_config,
        data_beyond_highest_advertised,
        limit_chunk_sizes,
        skip_emulate_network_latencies,
        true,
    );

    // Create the data streaming service config
    let data_streaming_service_config =
        data_streaming_service_config.unwrap_or(DataStreamingServiceConfig {
            enable_subscription_streaming,
            max_concurrent_requests: 3,
            max_concurrent_state_requests: 6,
            ..Default::default()
        });

    // Create the streaming service and connect it to the listener
    let streaming_service = DataStreamingService::new(
        aptos_data_client_config,
        data_streaming_service_config,
        aptos_data_client,
        streaming_service_listener,
        TimeService::mock(),
    );

    (streaming_client, streaming_service)
}

/// Verifies that the stream listener receives all epoch ending
/// notifications and that the payloads are contiguous.
async fn verify_continuous_epoch_ending_notifications(stream_listener: &mut DataStreamListener) {
    let mut next_expected_epoch = MIN_ADVERTISED_EPOCH_END;

    // Read notifications until we reach the end of the stream
    loop {
        let data_notification = get_data_notification(stream_listener).await.unwrap();
        match data_notification.data_payload {
            DataPayload::EpochEndingLedgerInfos(ledger_infos_with_sigs) => {
                // Verify the epochs of the ledger infos are contiguous
                for ledger_info_with_sigs in ledger_infos_with_sigs {
                    let ledger_info = ledger_info_with_sigs.ledger_info();
                    let epoch = ledger_info.commit_info().epoch();
                    assert!(ledger_info.ends_epoch());
                    assert_eq!(next_expected_epoch, epoch);
                    assert_le!(epoch, MAX_ADVERTISED_EPOCH_END);
                    next_expected_epoch += 1;
                }
            },
            DataPayload::EndOfStream => {
                assert_eq!(next_expected_epoch, MAX_ADVERTISED_EPOCH_END + 1);
                return; // We've reached the end of the stream
            },
            data_payload => unexpected_payload_type!(data_payload),
        }
    }
}

/// Verifies that the stream listener receives all state value
/// notifications and that the payloads are contiguous.
async fn verify_continuous_state_value_notifications(stream_listener: &mut DataStreamListener) {
    let mut next_expected_index = 0;

    // Read notifications until we reach the end of the stream
    loop {
        let data_notification = get_data_notification(stream_listener).await.unwrap();
        match data_notification.data_payload {
            DataPayload::StateValuesWithProof(state_values_with_proof) => {
                // Verify the start index matches the expected index
                assert_eq!(state_values_with_proof.first_index, next_expected_index);

                // Verify the last index matches the state value list length
                let num_state_values = state_values_with_proof.raw_values.len() as u64;
                assert_eq!(
                    state_values_with_proof.last_index,
                    next_expected_index + num_state_values - 1,
                );

                // Verify the number of state values is as expected
                assert_eq!(
                    state_values_with_proof.raw_values.len() as u64,
                    num_state_values
                );

                next_expected_index += num_state_values;
            },
            DataPayload::EndOfStream => {
                assert_eq!(next_expected_index, TOTAL_NUM_STATE_VALUES);
                return; // We've reached the end of the stream
            },
            data_payload => unexpected_payload_type!(data_payload),
        }
    }
}

/// Verifies that the stream listener receives all transaction
/// output notifications and that the payloads are contiguous.
async fn verify_continuous_output_notifications(
    stream_listener: &mut DataStreamListener,
    data_beyond_advertised: bool,
) {
    let mut next_expected_epoch = MIN_ADVERTISED_EPOCH_END;
    let mut next_expected_version = MIN_ADVERTISED_TRANSACTION_OUTPUT;

    // Read notifications until we reach the end of the stream
    loop {
        if let Ok(data_notification) = get_data_notification(stream_listener).await {
            match data_notification.data_payload {
                DataPayload::ContinuousTransactionOutputsWithProof(
                    ledger_info_with_sigs,
                    outputs_with_proofs,
                ) => {
                    // Verify the continuous outputs payload
                    let (new_expected_version, new_expected_epoch) =
                        verify_continuous_outputs_with_proof(
                            next_expected_epoch,
                            next_expected_version,
                            ledger_info_with_sigs,
                            TransactionOutputListWithProofV2::new_from_v1(outputs_with_proofs),
                        );

                    // Update the next expected version and epoch
                    next_expected_version = new_expected_version;
                    next_expected_epoch = new_expected_epoch;
                },
                data_payload => unexpected_payload_type!(data_payload),
            }
        } else {
            // Verify the next expected version and epoch depending on data availability
            if data_beyond_advertised {
                assert_eq!(next_expected_epoch, MAX_REAL_EPOCH_END + 1);
                assert_eq!(next_expected_version, MAX_REAL_TRANSACTION_OUTPUT + 1);
            } else {
                assert_eq!(next_expected_epoch, MAX_ADVERTISED_EPOCH_END + 1);
                assert_eq!(next_expected_version, MAX_ADVERTISED_TRANSACTION_OUTPUT + 1);
            }

            return; // We've reached the end of the stream
        }
    }
}

/// Verifies that the stream listener receives all transaction
/// notifications and that the payloads are contiguous.
async fn verify_continuous_transaction_notifications(
    stream_listener: &mut DataStreamListener,
    data_beyond_advertised: bool,
) {
    let mut next_expected_epoch = MIN_ADVERTISED_EPOCH_END;
    let mut next_expected_version = MIN_ADVERTISED_TRANSACTION;

    // Read notifications until we reach the end of the stream
    loop {
        if let Ok(data_notification) = get_data_notification(stream_listener).await {
            match data_notification.data_payload {
                DataPayload::ContinuousTransactionsWithProof(
                    ledger_info_with_sigs,
                    transactions_with_proofs,
                ) => {
                    // Verify the continuous transactions payload
                    let (new_expected_version, new_expected_epoch) =
                        verify_continuous_transactions_with_proof(
                            next_expected_epoch,
                            next_expected_version,
                            ledger_info_with_sigs,
                            transactions_with_proofs,
                        );

                    // Update the next expected version and epoch
                    next_expected_version = new_expected_version;
                    next_expected_epoch = new_expected_epoch;
                },
                data_payload => unexpected_payload_type!(data_payload),
            }
        } else {
            // Verify the next expected version and epoch depending on data availability
            if data_beyond_advertised {
                assert_eq!(next_expected_epoch, MAX_REAL_EPOCH_END + 1);
                assert_eq!(next_expected_version, MAX_REAL_TRANSACTION + 1);
            } else {
                assert_eq!(next_expected_epoch, MAX_ADVERTISED_EPOCH_END + 1);
                assert_eq!(next_expected_version, MAX_ADVERTISED_TRANSACTION + 1);
            }

            return; // We've reached the end of the stream
        }
    }
}

/// Verifies that the stream listener receives all transaction
/// or output notifications and that the payloads are contiguous.
async fn verify_continuous_transaction_or_output_notifications(
    stream_listener: &mut DataStreamListener,
    data_beyond_advertised: bool,
) {
    let mut next_expected_epoch = MIN_ADVERTISED_EPOCH_END;
    let mut next_expected_version = MIN_ADVERTISED_TRANSACTION;

    // Read notifications until we reach the end of the stream
    loop {
        if let Ok(data_notification) = get_data_notification(stream_listener).await {
            match data_notification.data_payload {
                DataPayload::ContinuousTransactionsWithProof(
                    ledger_info_with_sigs,
                    transactions_with_proofs,
                ) => {
                    // Verify the continuous transactions payload
                    let (new_expected_version, new_expected_epoch) =
                        verify_continuous_transactions_with_proof(
                            next_expected_epoch,
                            next_expected_version,
                            ledger_info_with_sigs,
                            transactions_with_proofs,
                        );

                    // Update the next expected version and epoch
                    next_expected_version = new_expected_version;
                    next_expected_epoch = new_expected_epoch;
                },
                DataPayload::ContinuousTransactionOutputsWithProof(
                    ledger_info_with_sigs,
                    outputs_with_proofs,
                ) => {
                    // Verify the continuous outputs payload
                    let (new_expected_version, new_expected_epoch) =
                        verify_continuous_outputs_with_proof(
                            next_expected_epoch,
                            next_expected_version,
                            ledger_info_with_sigs,
                            TransactionOutputListWithProofV2::new_from_v1(outputs_with_proofs),
                        );

                    // Update the next expected version and epoch
                    next_expected_version = new_expected_version;
                    next_expected_epoch = new_expected_epoch;
                },
                data_payload => unexpected_payload_type!(data_payload),
            }
        } else {
            // Verify the next expected version and epoch depending on data availability
            if data_beyond_advertised {
                assert_eq!(next_expected_epoch, MAX_REAL_EPOCH_END + 1);
                assert_eq!(next_expected_version, MAX_REAL_TRANSACTION + 1);
            } else {
                assert_eq!(next_expected_epoch, MAX_ADVERTISED_EPOCH_END + 1);
                assert_eq!(next_expected_version, MAX_ADVERTISED_TRANSACTION + 1);
            }

            return; // We've reached the end of the stream
        }
    }
}

/// Verifies the continuous transaction outputs payload
/// and returns the new expected version and epoch.
fn verify_continuous_outputs_with_proof(
    expected_epoch: u64,
    expected_version: u64,
    ledger_info_with_sigs: LedgerInfoWithSignatures,
    outputs_with_proofs: TransactionOutputListWithProofV2,
) -> (u64, u64) {
    // Verify the ledger info epoch matches the expected epoch
    let ledger_info = ledger_info_with_sigs.ledger_info();
    assert_eq!(ledger_info.epoch(), expected_epoch);

    // Verify the output start version matches the expected version
    let first_output_version = outputs_with_proofs.get_first_output_version();
    assert_eq!(Some(expected_version), first_output_version);

    // Calculate the next expected version
    let num_outputs = outputs_with_proofs
        .get_output_list_with_proof()
        .get_num_outputs() as u64;
    let next_expected_version = expected_version + num_outputs;

    // Update epochs if we've hit the epoch end
    let last_output_version = first_output_version.unwrap() + num_outputs - 1;
    let next_expected_epoch =
        if ledger_info.version() == last_output_version && ledger_info.ends_epoch() {
            expected_epoch + 1
        } else {
            expected_epoch
        };

    // Return the new expected epoch and version
    (next_expected_version, next_expected_epoch)
}

/// Verifies the continuous transaction payload
/// and returns the new expected version and epoch.
fn verify_continuous_transactions_with_proof(
    expected_epoch: u64,
    expected_version: u64,
    ledger_info_with_sigs: LedgerInfoWithSignatures,
    transactions_with_proofs: TransactionListWithProof,
) -> (u64, u64) {
    // Verify the ledger info epoch matches the expected epoch
    let ledger_info = ledger_info_with_sigs.ledger_info();
    assert_eq!(ledger_info.epoch(), expected_epoch);

    // Verify the transaction start version matches the expected version
    let first_transaction_version = transactions_with_proofs.get_first_transaction_version();
    assert_eq!(Some(expected_version), first_transaction_version);

    // Calculate the next expected version
    let num_transactions = transactions_with_proofs.get_num_transactions() as u64;
    let next_expected_version = expected_version + num_transactions;

    // Update epochs if we've hit the epoch end
    let last_transaction_version = first_transaction_version.unwrap() + num_transactions - 1;
    let next_expected_epoch =
        if ledger_info.version() == last_transaction_version && ledger_info.ends_epoch() {
            expected_epoch + 1
        } else {
            expected_epoch
        };

    // Return the new expected epoch and version
    (next_expected_version, next_expected_epoch)
}

/// Verifies that the stream listener receives all output notifications
/// (for the specified range) and that the payloads are contiguous.
async fn verify_output_notifications(
    stream_listener: &mut DataStreamListener,
    first_output_version: u64,
    last_output_version: u64,
) {
    let mut next_expected_version = first_output_version;

    // Read notifications until we reach the end of the stream
    loop {
        let data_notification = get_data_notification(stream_listener).await.unwrap();
        match data_notification.data_payload {
            DataPayload::TransactionOutputsWithProof(outputs_with_proof) => {
                // Verify the transaction output start version matches the expected version
                let first_output_version = outputs_with_proof.get_first_output_version();
                assert_eq!(Some(next_expected_version), first_output_version);

                // Calculate the next expected version
                let num_outputs = outputs_with_proof.get_num_outputs();
                next_expected_version += num_outputs as u64;
            },
            DataPayload::EndOfStream => {
                return assert_eq!(next_expected_version, last_output_version + 1)
            },
            data_payload => unexpected_payload_type!(data_payload),
        }
    }
}

/// Verifies that the stream listener receives all transaction notifications
/// (for the specified range) and that the payloads are contiguous.
async fn verify_transaction_notifications(
    stream_listener: &mut DataStreamListener,
    first_transaction_version: u64,
    last_transaction_version: u64,
) {
    let mut next_expected_version = first_transaction_version;

    // Read notifications until we reach the end of the stream
    loop {
        let data_notification = get_data_notification(stream_listener).await.unwrap();
        match data_notification.data_payload {
            DataPayload::TransactionsWithProof(transactions_with_proof) => {
                // Verify the transaction start version matches the expected version
                let first_transaction_version =
                    transactions_with_proof.get_first_transaction_version();
                assert_eq!(Some(next_expected_version), first_transaction_version);

                // Calculate the next expected version
                let num_transactions = transactions_with_proof.get_num_transactions();
                next_expected_version += num_transactions as u64;
            },
            DataPayload::EndOfStream => {
                return assert_eq!(next_expected_version, last_transaction_version + 1)
            },
            data_payload => unexpected_payload_type!(data_payload),
        }
    }
}
