// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_notification::DataPayload,
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
use aptos_config::config::DataStreamingServiceConfig;
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

    // Read the data notifications from the stream and verify index ordering
    let mut next_expected_index = 0;
    loop {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
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
            }
            DataPayload::EndOfStream => {
                return assert_eq!(next_expected_index, TOTAL_NUM_STATE_VALUES)
            }
            data_payload => unexpected_payload_type!(data_payload),
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_state_values_limited_chunks() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service_with_chunk_limits();

    // Request a new state value stream starting at the next expected index
    let mut next_expected_index = 0;
    let mut stream_listener = streaming_client
        .get_all_state_values(MAX_ADVERTISED_STATES, Some(next_expected_index))
        .await
        .unwrap();

    // Terminate and request streams when the chunks are no longer contiguous
    loop {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        let reset_stream = match data_notification.data_payload {
            DataPayload::StateValuesWithProof(state_values_with_proof) => {
                if state_values_with_proof.first_index == next_expected_index {
                    next_expected_index += state_values_with_proof.raw_values.len() as u64;
                    false
                } else {
                    true // We hit a non-contiguous chunk
                }
            }
            DataPayload::EndOfStream => {
                if next_expected_index != TOTAL_NUM_STATE_VALUES {
                    true // The stream thought it had completed, but the chunk was incomplete
                } else {
                    return; // All data was received!
                }
            }
            data_payload => unexpected_payload_type!(data_payload),
        };

        if reset_stream {
            // Terminate the stream and fetch a new one (we hit non-contiguous data)
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

            stream_listener = streaming_client
                .get_all_state_values(MAX_ADVERTISED_STATES, Some(next_expected_index))
                .await
                .unwrap();
        }
    }
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
                next_expected_index += state_values_with_proof.raw_values.len() as u64;

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
            }
            DataPayload::EndOfStream => {
                return assert_eq!(next_expected_index, TOTAL_NUM_STATE_VALUES)
            }
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

    // Read the data notifications from the stream and verify the payloads
    let mut next_expected_epoch = MIN_ADVERTISED_EPOCH_END;
    let mut next_expected_version = MIN_ADVERTISED_TRANSACTION_OUTPUT;
    loop {
        if let Ok(data_notification) = get_data_notification(&mut stream_listener).await {
            match data_notification.data_payload {
                DataPayload::ContinuousTransactionOutputsWithProof(
                    ledger_info_with_sigs,
                    outputs_with_proofs,
                ) => {
                    let ledger_info = ledger_info_with_sigs.ledger_info();
                    // Verify the epoch of the ledger info
                    assert_eq!(ledger_info.epoch(), next_expected_epoch);

                    // Verify the output start version matches the expected version
                    let first_output_version = outputs_with_proofs.first_transaction_output_version;
                    assert_eq!(Some(next_expected_version), first_output_version);

                    let num_outputs = outputs_with_proofs.transactions_and_outputs.len() as u64;
                    next_expected_version += num_outputs;

                    // Update epochs if we've hit the epoch end
                    let last_output_version = first_output_version.unwrap() + num_outputs - 1;
                    if ledger_info.version() == last_output_version && ledger_info.ends_epoch() {
                        next_expected_epoch += 1;
                    }
                }
                data_payload => unexpected_payload_type!(data_payload),
            }
        } else {
            assert_eq!(next_expected_epoch, MAX_ADVERTISED_EPOCH_END + 1);
            return assert_eq!(next_expected_version, MAX_ADVERTISED_TRANSACTION_OUTPUT + 1);
        }
    }
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
                let ledger_info = ledger_info_with_sigs.ledger_info();
                // Verify the epoch of the ledger info
                assert_eq!(ledger_info.epoch(), next_expected_epoch);

                // Verify the output start version matches the expected version
                let first_output_version = outputs_with_proofs.first_transaction_output_version;
                assert_eq!(Some(next_expected_version), first_output_version);

                let num_outputs = outputs_with_proofs.transactions_and_outputs.len() as u64;
                next_expected_version += num_outputs;

                // Update epochs if we've hit the epoch end
                let last_output_version = first_output_version.unwrap() + num_outputs - 1;
                if ledger_info.version() == last_output_version && ledger_info.ends_epoch() {
                    next_expected_epoch += 1;
                }
            }
            DataPayload::EndOfStream => {
                return assert_eq!(next_expected_version, target_version + 1)
            }
            data_payload => unexpected_payload_type!(data_payload),
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_continuous_outputs_limited_chunks() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service_with_chunk_limits();
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

    // Terminate and request new streams when the chunks are no longer contiguous
    loop {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        let reset_stream = match data_notification.data_payload {
            DataPayload::ContinuousTransactionOutputsWithProof(
                ledger_info_with_sigs,
                outputs_with_proofs,
            ) => {
                let first_output_version = outputs_with_proofs
                    .first_transaction_output_version
                    .unwrap();
                let num_outputs = outputs_with_proofs.transactions_and_outputs.len() as u64;
                let last_output_version = first_output_version + num_outputs - 1;

                if first_output_version == next_expected_version {
                    // Update the next version and epoch (if applicable)
                    next_expected_version += num_outputs;
                    let ledger_info = ledger_info_with_sigs.ledger_info();
                    if ledger_info.version() == last_output_version && ledger_info.ends_epoch() {
                        next_expected_epoch += 1;
                    }

                    // Check if we've hit the target epoch
                    if next_expected_epoch > end_epoch {
                        return; // All data was received!
                    }

                    false
                } else {
                    true // We hit a non-contiguous chunk
                }
            }
            data_payload => unexpected_payload_type!(data_payload),
        };

        if reset_stream {
            // Terminate the stream and fetch a new one (we hit non-contiguous data)
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

            stream_listener = streaming_client
                .continuously_stream_transaction_outputs(
                    next_expected_version - 1,
                    next_expected_epoch,
                    None,
                )
                .await
                .unwrap();
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
                let first_output_version = outputs_with_proofs.first_transaction_output_version;
                assert_eq!(Some(next_expected_version), first_output_version);

                let num_outputs = outputs_with_proofs.transactions_and_outputs.len() as u64;
                next_expected_version += num_outputs;

                let last_output_version = first_output_version.unwrap() + num_outputs - 1;
                let ledger_info = ledger_info_with_sigs.ledger_info();
                if ledger_info.version() == last_output_version && ledger_info.ends_epoch() {
                    next_expected_epoch += 1;
                }
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
                if next_expected_epoch > end_epoch {
                    return;
                }
            }
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

    // Read the data notifications from the stream and verify the payloads
    let mut next_expected_epoch = MIN_ADVERTISED_EPOCH_END;
    let mut next_expected_version = MIN_ADVERTISED_TRANSACTION;
    loop {
        if let Ok(data_notification) = get_data_notification(&mut stream_listener).await {
            match data_notification.data_payload {
                DataPayload::ContinuousTransactionsWithProof(
                    ledger_info_with_sigs,
                    transactions_with_proof,
                ) => {
                    let ledger_info = ledger_info_with_sigs.ledger_info();
                    // Verify the epoch of the ledger info
                    assert_eq!(ledger_info.epoch(), next_expected_epoch);

                    // Verify the transaction start version matches the expected version
                    let first_transaction_version =
                        transactions_with_proof.first_transaction_version;
                    assert_eq!(Some(next_expected_version), first_transaction_version);

                    // Verify the payload contains events
                    assert_some!(transactions_with_proof.events);

                    let num_transactions = transactions_with_proof.transactions.len() as u64;
                    next_expected_version += num_transactions;

                    // Update epochs if we've hit the epoch end
                    let last_transaction_version =
                        first_transaction_version.unwrap() + num_transactions - 1;
                    if ledger_info.version() == last_transaction_version && ledger_info.ends_epoch()
                    {
                        next_expected_epoch += 1;
                    }
                }
                data_payload => unexpected_payload_type!(data_payload),
            }
        } else {
            assert_eq!(next_expected_epoch, MAX_ADVERTISED_EPOCH_END + 1);
            return assert_eq!(next_expected_version, MAX_ADVERTISED_TRANSACTION + 1);
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_continuous_transactions_limited_chunks() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service_with_chunk_limits();
    let end_epoch = MIN_ADVERTISED_EPOCH_END + 5;

    // Request a continuous transaction stream and get a data stream listener
    let mut next_expected_epoch = MIN_ADVERTISED_EPOCH_END;
    let mut next_expected_version = MIN_ADVERTISED_TRANSACTION;
    let mut stream_listener = streaming_client
        .continuously_stream_transactions(
            next_expected_version - 1,
            next_expected_epoch,
            true,
            None,
        )
        .await
        .unwrap();

    // Terminate and request new streams when the chunks are no longer contiguous
    loop {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        let reset_stream = match data_notification.data_payload {
            DataPayload::ContinuousTransactionsWithProof(
                ledger_info_with_sigs,
                transactions_with_proofs,
            ) => {
                let first_transaction_version =
                    transactions_with_proofs.first_transaction_version.unwrap();
                let num_transactions = transactions_with_proofs.transactions.len() as u64;
                let last_transaction_version = first_transaction_version + num_transactions - 1;

                if first_transaction_version == next_expected_version {
                    // Update the next version and epoch (if applicable)
                    next_expected_version += num_transactions;
                    let ledger_info = ledger_info_with_sigs.ledger_info();
                    if ledger_info.version() == last_transaction_version && ledger_info.ends_epoch()
                    {
                        next_expected_epoch += 1;
                    }

                    // Check if we've hit the target epoch
                    if next_expected_epoch > end_epoch {
                        return; // All data was received!
                    }

                    false
                } else {
                    true // We hit a non-contiguous chunk
                }
            }
            data_payload => unexpected_payload_type!(data_payload),
        };

        if reset_stream {
            // Terminate the stream and fetch a new one (we hit non-contiguous data)
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
    }
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
                let ledger_info = ledger_info_with_sigs.ledger_info();
                // Verify the epoch of the ledger info
                assert_eq!(ledger_info.epoch(), next_expected_epoch);

                // Verify the transaction start version matches the expected version
                let first_transaction_version = transactions_with_proof.first_transaction_version;
                assert_eq!(Some(next_expected_version), first_transaction_version);

                // Verify the payload contains events
                assert_some!(transactions_with_proof.events);

                let num_transactions = transactions_with_proof.transactions.len() as u64;
                next_expected_version += num_transactions;

                // Update epochs if we've hit the epoch end
                let last_transaction_version =
                    first_transaction_version.unwrap() + num_transactions - 1;
                if ledger_info.version() == last_transaction_version && ledger_info.ends_epoch() {
                    next_expected_epoch += 1;
                }
            }
            DataPayload::EndOfStream => {
                return assert_eq!(next_expected_version, target_version + 1)
            }
            data_payload => unexpected_payload_type!(data_payload),
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

    // Read the data notifications from the stream and verify epoch ordering
    let mut next_expected_epoch = MIN_ADVERTISED_EPOCH_END;
    loop {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        match data_notification.data_payload {
            DataPayload::EpochEndingLedgerInfos(ledger_infos_with_sigs) => {
                // Verify the epochs of the ledger infos are contiguous
                for ledger_info_with_sigs in ledger_infos_with_sigs {
                    let epoch = ledger_info_with_sigs.ledger_info().commit_info().epoch();
                    assert_eq!(next_expected_epoch, epoch);
                    assert_le!(epoch, MAX_ADVERTISED_EPOCH_END);
                    next_expected_epoch += 1;
                }
            }
            DataPayload::EndOfStream => {
                return assert_eq!(next_expected_epoch, MAX_ADVERTISED_EPOCH_END + 1)
            }
            data_payload => unexpected_payload_type!(data_payload),
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_epoch_ending_limited_chunks() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service_with_chunk_limits();

    // Request a new epoch ending stream starting at the next expected index.
    let mut next_expected_epoch = MIN_ADVERTISED_EPOCH_END;
    let mut stream_listener = streaming_client
        .get_all_epoch_ending_ledger_infos(next_expected_epoch)
        .await
        .unwrap();

    // Terminate and request streams when the chunks are no longer contiguous
    loop {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        let reset_stream = match data_notification.data_payload {
            DataPayload::EpochEndingLedgerInfos(ledger_infos_with_sigs) => {
                let first_ledger_info_epoch = ledger_infos_with_sigs[0].ledger_info().epoch();
                if first_ledger_info_epoch == next_expected_epoch {
                    next_expected_epoch += ledger_infos_with_sigs.len() as u64;
                    false
                } else {
                    true // We hit a non-contiguous chunk
                }
            }
            DataPayload::EndOfStream => {
                if next_expected_epoch != MAX_ADVERTISED_EPOCH_END + 1 {
                    true // The stream thought it had completed, but the chunk was incomplete
                } else {
                    return; // All data was received!
                }
            }
            data_payload => unexpected_payload_type!(data_payload),
        };

        if reset_stream {
            // Terminate the stream and fetch a new one (we hit non-contiguous data)
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

            stream_listener = streaming_client
                .get_all_epoch_ending_ledger_infos(next_expected_epoch)
                .await
                .unwrap();
        }
    }
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
                next_expected_epoch += ledger_infos_with_sigs.len() as u64;
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
            }
            DataPayload::EndOfStream => {
                return assert_eq!(next_expected_epoch, MAX_ADVERTISED_EPOCH_END + 1)
            }
            data_payload => unexpected_payload_type!(data_payload),
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_subscribe_outputs() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service_with_data_delay();

    // Request a continuous output stream and get a data stream listener
    let mut stream_listener = streaming_client
        .continuously_stream_transaction_outputs(
            MIN_ADVERTISED_TRANSACTION_OUTPUT - 1,
            MIN_ADVERTISED_EPOCH_END,
            None,
        )
        .await
        .unwrap();

    // Read the data notifications from the stream and verify the payloads
    let mut next_expected_epoch = MIN_ADVERTISED_EPOCH_END;
    let mut next_expected_version = MIN_ADVERTISED_TRANSACTION_OUTPUT;
    loop {
        if let Ok(data_notification) = get_data_notification(&mut stream_listener).await {
            match data_notification.data_payload {
                DataPayload::ContinuousTransactionOutputsWithProof(
                    ledger_info_with_sigs,
                    outputs_with_proofs,
                ) => {
                    let ledger_info = ledger_info_with_sigs.ledger_info();
                    // Verify the epoch of the ledger info
                    assert_eq!(ledger_info.epoch(), next_expected_epoch);

                    // Verify the output start version matches the expected version
                    let first_output_version = outputs_with_proofs.first_transaction_output_version;
                    assert_eq!(Some(next_expected_version), first_output_version);

                    let num_outputs = outputs_with_proofs.transactions_and_outputs.len() as u64;
                    next_expected_version += num_outputs;

                    // Update epochs if we've hit the epoch end
                    let last_output_version = first_output_version.unwrap() + num_outputs - 1;
                    if ledger_info.version() == last_output_version && ledger_info.ends_epoch() {
                        next_expected_epoch += 1;
                    }
                }
                data_payload => unexpected_payload_type!(data_payload),
            }
        } else {
            assert_eq!(next_expected_epoch, MAX_REAL_EPOCH_END + 1);
            return assert_eq!(next_expected_version, MAX_REAL_TRANSACTION_OUTPUT + 1);
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_subscribe_transactions() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service_with_data_delay();

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

    // Read the data notifications from the stream and verify the payloads
    let mut next_expected_epoch = MIN_ADVERTISED_EPOCH_END;
    let mut next_expected_version = MIN_ADVERTISED_TRANSACTION;
    loop {
        if let Ok(data_notification) = get_data_notification(&mut stream_listener).await {
            match data_notification.data_payload {
                DataPayload::ContinuousTransactionsWithProof(
                    ledger_info_with_sigs,
                    transactions_with_proofs,
                ) => {
                    let ledger_info = ledger_info_with_sigs.ledger_info();
                    // Verify the epoch of the ledger info
                    assert_eq!(ledger_info.epoch(), next_expected_epoch);

                    // Verify the transaction start version matches the expected version
                    let first_transaction_version =
                        transactions_with_proofs.first_transaction_version;
                    assert_eq!(Some(next_expected_version), first_transaction_version);

                    let num_transactions = transactions_with_proofs.transactions.len() as u64;
                    next_expected_version += num_transactions;

                    // Update epochs if we've hit the epoch end
                    let last_transaction_version =
                        first_transaction_version.unwrap() + num_transactions - 1;
                    if ledger_info.version() == last_transaction_version && ledger_info.ends_epoch()
                    {
                        next_expected_epoch += 1;
                    }
                }
                data_payload => unexpected_payload_type!(data_payload),
            }
        } else {
            assert_eq!(next_expected_epoch, MAX_REAL_EPOCH_END + 1);
            return assert_eq!(next_expected_version, MAX_REAL_TRANSACTION + 1);
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_transaction_outputs() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service();

    // Request a transaction output stream and get a data stream listener
    let mut next_expected_version = MIN_ADVERTISED_TRANSACTION_OUTPUT;
    let mut stream_listener = streaming_client
        .get_all_transaction_outputs(
            next_expected_version,
            MAX_ADVERTISED_TRANSACTION_OUTPUT,
            MAX_ADVERTISED_TRANSACTION_OUTPUT,
        )
        .await
        .unwrap();

    // Read the data notifications from the stream and verify the payloads
    loop {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        match data_notification.data_payload {
            DataPayload::TransactionOutputsWithProof(outputs_with_proof) => {
                // Verify the transaction output start version matches the expected version
                let first_output_version = outputs_with_proof.first_transaction_output_version;
                assert_eq!(Some(next_expected_version), first_output_version);

                let num_outputs = outputs_with_proof.transactions_and_outputs.len();
                next_expected_version += num_outputs as u64;
            }
            DataPayload::EndOfStream => {
                return assert_eq!(next_expected_version, MAX_ADVERTISED_TRANSACTION_OUTPUT + 1)
            }
            data_payload => unexpected_payload_type!(data_payload),
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_transaction_outputs_limited_chunks() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service_with_chunk_limits();

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

    // Terminate and request streams when the chunks are no longer contiguous
    loop {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        let reset_stream = match data_notification.data_payload {
            DataPayload::TransactionOutputsWithProof(outputs_with_proof) => {
                let first_output_version =
                    outputs_with_proof.first_transaction_output_version.unwrap();
                if first_output_version == next_expected_version {
                    next_expected_version +=
                        outputs_with_proof.transactions_and_outputs.len() as u64;
                    false
                } else {
                    true // We hit a non-contiguous chunk
                }
            }
            DataPayload::EndOfStream => {
                if next_expected_version != MAX_ADVERTISED_TRANSACTION_OUTPUT + 1 {
                    true // The stream thought it had completed, but the chunk was incomplete
                } else {
                    return; // All data was received!
                }
            }
            data_payload => unexpected_payload_type!(data_payload),
        };

        if reset_stream {
            // Terminate the stream and fetch a new one (we hit non-contiguous data)
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

            stream_listener = streaming_client
                .get_all_transaction_outputs(
                    next_expected_version,
                    MAX_ADVERTISED_TRANSACTION_OUTPUT,
                    MAX_ADVERTISED_TRANSACTION_OUTPUT,
                )
                .await
                .unwrap();
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

    // Read the data notifications from the stream and verify the payloads
    let mut next_expected_transaction = MIN_ADVERTISED_TRANSACTION;
    loop {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        match data_notification.data_payload {
            DataPayload::TransactionsWithProof(transactions_with_proof) => {
                // Verify the transaction start version matches the expected version
                let first_transaction_version = transactions_with_proof.first_transaction_version;
                assert_eq!(Some(next_expected_transaction), first_transaction_version);

                // Verify the payload contains events
                assert_some!(transactions_with_proof.events);

                let num_transactions = transactions_with_proof.transactions.len();
                next_expected_transaction += num_transactions as u64;
            }
            DataPayload::EndOfStream => {
                return assert_eq!(next_expected_transaction, MAX_ADVERTISED_TRANSACTION + 1)
            }
            data_payload => unexpected_payload_type!(data_payload),
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_notifications_transactions_limited_chunks() {
    // Create a new streaming client and service
    let streaming_client = create_streaming_client_and_service_with_chunk_limits();

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

    // Terminate and request streams when the chunks are no longer contiguous
    loop {
        let data_notification = get_data_notification(&mut stream_listener).await.unwrap();
        let reset_stream = match data_notification.data_payload {
            DataPayload::TransactionsWithProof(transactions_with_proof) => {
                let first_transaction_version =
                    transactions_with_proof.first_transaction_version.unwrap();
                if first_transaction_version == next_expected_version {
                    next_expected_version += transactions_with_proof.transactions.len() as u64;
                    false
                } else {
                    true // We hit a non-contiguous chunk
                }
            }
            DataPayload::EndOfStream => {
                if next_expected_version != MAX_ADVERTISED_TRANSACTION + 1 {
                    true // The stream thought it had completed, but the chunk was incomplete
                } else {
                    return; // All data was received!
                }
            }
            data_payload => unexpected_payload_type!(data_payload),
        };

        if reset_stream {
            // Terminate the stream and fetch a new one (we hit non-contiguous data)
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
    }
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
                let first_transaction_version = transactions_with_proof.first_transaction_version;
                assert_eq!(Some(next_expected_version), first_transaction_version);

                next_expected_version += transactions_with_proof.transactions.len() as u64;
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
            }
            DataPayload::EndOfStream => {
                return assert_eq!(next_expected_version, MAX_ADVERTISED_TRANSACTION + 1)
            }
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
            }
            DataPayload::TransactionOutputsWithProof(_) => {}
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
        DataPayload::StateValuesWithProof(_) => {}
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
            DataPayload::StateValuesWithProof(_) => {}
            DataPayload::EndOfStream => panic!("The stream should have terminated!"),
            data_payload => unexpected_payload_type!(data_payload),
        }
    }
}

fn create_streaming_client_and_service() -> StreamingServiceClient {
    create_streaming_client_and_spawn_server(false, false, false)
}

fn create_streaming_client_and_service_with_data_delay() -> StreamingServiceClient {
    create_streaming_client_and_spawn_server(true, false, false)
}

fn create_streaming_client_and_service_with_chunk_limits() -> StreamingServiceClient {
    create_streaming_client_and_spawn_server(false, true, true)
}

fn create_streaming_client_and_spawn_server(
    data_beyond_highest_advertised: bool,
    limit_chunk_sizes: bool,
    skip_emulate_network_latencies: bool,
) -> StreamingServiceClient {
    let (client, service) = create_streaming_client_and_server(
        data_beyond_highest_advertised,
        limit_chunk_sizes,
        skip_emulate_network_latencies,
    );
    tokio::spawn(service.start_service());
    client
}

pub fn create_streaming_client_and_server(
    data_beyond_highest_advertised: bool,
    limit_chunk_sizes: bool,
    skip_emulate_network_latencies: bool,
) -> (
    StreamingServiceClient,
    DataStreamingService<MockAptosDataClient>,
) {
    initialize_logger();

    // Create a new streaming client and listener
    let (streaming_client, streaming_service_listener) =
        new_streaming_service_client_listener_pair();

    // Create a mock data client
    let aptos_data_client = MockAptosDataClient::new(
        data_beyond_highest_advertised,
        limit_chunk_sizes,
        skip_emulate_network_latencies,
    );

    // Create the data streaming service config
    let data_streaming_service_config = DataStreamingServiceConfig {
        max_concurrent_requests: 3,
        max_concurrent_state_requests: 6,
        ..Default::default()
    };

    // Create the streaming service and connect it to the listener
    let streaming_service = DataStreamingService::new(
        data_streaming_service_config,
        aptos_data_client,
        streaming_service_listener,
    );

    (streaming_client, streaming_service)
}
