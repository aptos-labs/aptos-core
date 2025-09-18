// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    requests::{
        DataRequest, EpochEndingLedgerInfoRequest, NewTransactionOutputsWithProofRequest,
        NewTransactionsOrOutputsWithProofRequest, NewTransactionsWithProofRequest,
        StateValuesWithProofRequest, SubscribeTransactionOutputsWithProofRequest,
        SubscribeTransactionsOrOutputsWithProofRequest, SubscribeTransactionsWithProofRequest,
        SubscriptionStreamMetadata, TransactionOutputsWithProofRequest,
        TransactionsOrOutputsWithProofRequest, TransactionsWithProofRequest,
    },
    responses::{CompleteDataRange, DataSummary, ProtocolMetadata},
    Epoch, StorageServiceRequest,
};
use aptos_config::config::AptosDataClientConfig;
use aptos_crypto::hash::HashValue;
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::{
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    transaction::Version,
};
use claims::{assert_err, assert_ok};
use proptest::{arbitrary::any, prelude::*};
use rand::{thread_rng, Rng};

#[test]
fn test_complete_data_ranges() {
    // Test valid data ranges
    assert_ok!(CompleteDataRange::new(0, 0));
    assert_ok!(CompleteDataRange::new(10, 10));
    assert_ok!(CompleteDataRange::new(10, 20));
    assert_ok!(CompleteDataRange::new(u64::MAX, u64::MAX));

    // Test degenerate data ranges
    assert_err!(CompleteDataRange::new(1, 0));
    assert_err!(CompleteDataRange::new(20, 10));
    assert_err!(CompleteDataRange::new(u64::MAX, 0));
    assert_err!(CompleteDataRange::new(u64::MAX, 1));

    // Test the overflow edge cases
    assert_ok!(CompleteDataRange::new(1, u64::MAX));
    assert_ok!(CompleteDataRange::new(0, u64::MAX - 1));
    assert_err!(CompleteDataRange::new(0, u64::MAX));
}

#[test]
fn test_data_summary_service_epoch_ending_ledger_infos() {
    // Create a data client config and data summary
    let data_client_config = AptosDataClientConfig::default();
    let data_summary = DataSummary {
        epoch_ending_ledger_infos: Some(create_data_range(100, 200)),
        ..Default::default()
    };

    // Verify the different requests that can be serviced
    for compression in [true, false] {
        // Test the valid data ranges
        let valid_ranges = vec![(100, 200), (125, 175), (100, 100), (150, 150), (200, 200)];
        verify_can_service_epoch_ending_requests(
            &data_client_config,
            &data_summary,
            compression,
            valid_ranges,
            true,
        );

        // Test the missing data ranges
        let invalid_ranges = vec![(99, 200), (100, 201), (50, 250), (50, 150), (150, 250)];
        verify_can_service_epoch_ending_requests(
            &data_client_config,
            &data_summary,
            compression,
            invalid_ranges,
            false,
        );

        // Test degenerate data ranges
        let degenerate_ranges = vec![(200, 199), (200, 100), (150, 149)];
        verify_can_service_epoch_ending_requests(
            &data_client_config,
            &data_summary,
            compression,
            degenerate_ranges,
            false,
        );
    }
}

#[test]
fn test_data_summary_service_optimistic_fetch() {
    // Test both v1 and v2 transaction requests
    for use_request_v2 in [false, true] {
        // Create a data client config with the specified max optimistic fetch lag
        let max_optimistic_fetch_lag_secs = 50;
        let data_client_config = AptosDataClientConfig {
            max_optimistic_fetch_lag_secs,
            ..Default::default()
        };

        // Create a mock time service and get the current timestamp
        let time_service = TimeService::mock();
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;

        // Create a data summary with the specified synced ledger info
        let highest_synced_version = 10_000;
        let data_summary = DataSummary {
            synced_ledger_info: Some(create_ledger_info_at_version_and_timestamp(
                highest_synced_version,
                timestamp_usecs,
            )),
            ..Default::default()
        };

        // Elapse the time service by half the max optimistic fetch lag
        time_service
            .clone()
            .into_mock()
            .advance_secs(max_optimistic_fetch_lag_secs / 2);

        // Verify that optimistic fetch requests can be serviced
        for compression in [true, false] {
            let known_versions = vec![0, 1, highest_synced_version, highest_synced_version * 2];
            verify_can_service_optimistic_fetch_requests(
                use_request_v2,
                &data_client_config,
                &data_summary,
                time_service.clone(),
                compression,
                known_versions,
                true,
            );
        }

        // Elapse the time service by the max optimistic fetch lag
        time_service
            .clone()
            .into_mock()
            .advance_secs(max_optimistic_fetch_lag_secs);

        // Verify that optimistic fetch requests can no longer be serviced
        // (as the max lag has been exceeded for the given data summary).
        for compression in [true, false] {
            let known_versions = vec![0, 1, highest_synced_version, highest_synced_version * 2];
            verify_can_service_optimistic_fetch_requests(
                use_request_v2,
                &data_client_config,
                &data_summary,
                time_service.clone(),
                compression,
                known_versions,
                false,
            );
        }
    }
}

#[test]
fn test_data_summary_service_subscription() {
    // Test both v1 and v2 transaction requests
    for use_request_v2 in [false, true] {
        // Create a data client config with the specified max subscription lag
        let max_subscription_lag_secs = 100;
        let data_client_config = AptosDataClientConfig {
            max_subscription_lag_secs,
            ..Default::default()
        };

        // Create a mock time service and get the current timestamp
        let time_service = TimeService::mock();
        let timestamp_usecs = time_service.now_unix_time().as_micros() as u64;

        // Create a data summary with the specified synced ledger info
        let highest_synced_version = 50_000;
        let data_summary = DataSummary {
            synced_ledger_info: Some(create_ledger_info_at_version_and_timestamp(
                highest_synced_version,
                timestamp_usecs,
            )),
            ..Default::default()
        };

        // Elapse the time service by half the max subscription lag
        time_service
            .clone()
            .into_mock()
            .advance_secs(max_subscription_lag_secs / 2);

        // Verify that subscription requests can be serviced
        for compression in [true, false] {
            let known_versions = vec![0, 1, highest_synced_version, highest_synced_version * 2];
            verify_can_service_subscription_requests(
                use_request_v2,
                &data_client_config,
                &data_summary,
                time_service.clone(),
                compression,
                known_versions,
                true,
            );
        }

        // Elapse the time service by the max subscription lag
        time_service
            .clone()
            .into_mock()
            .advance_secs(max_subscription_lag_secs);

        // Verify that subscription requests can no longer be serviced
        // (as the max lag has been exceeded for the given data summary).
        for compression in [true, false] {
            let known_versions = vec![0, 1, highest_synced_version, highest_synced_version * 2];
            verify_can_service_subscription_requests(
                use_request_v2,
                &data_client_config,
                &data_summary,
                time_service.clone(),
                compression,
                known_versions,
                false,
            );
        }
    }
}

#[test]
fn test_data_summary_service_transactions() {
    // Test both v1 and v2 transaction requests
    for use_request_v2 in [false, true] {
        // Create a data client config and data summary
        let data_client_config = AptosDataClientConfig::default();
        let data_summary = DataSummary {
            synced_ledger_info: Some(create_ledger_info_at_version(250)),
            transactions: Some(create_data_range(100, 200)),
            ..Default::default()
        };

        // Verify the different requests that can be serviced
        for compression in [true, false] {
            // Test the valid data ranges and proofs
            let valid_ranges_and_proofs = vec![
                (100, 200, 225),
                (125, 175, 225),
                (100, 100, 225),
                (150, 150, 225),
                (200, 200, 225),
                (200, 200, 250),
            ];
            verify_can_service_transaction_requests(
                use_request_v2,
                &data_client_config,
                &data_summary,
                compression,
                valid_ranges_and_proofs,
                true,
            );

            // Test the missing data ranges and proofs
            let missing_data_ranges = vec![
                (99, 200, 225),
                (100, 201, 225),
                (50, 250, 225),
                (50, 150, 225),
                (150, 250, 225),
            ];
            verify_can_service_transaction_requests(
                use_request_v2,
                &data_client_config,
                &data_summary,
                compression,
                missing_data_ranges,
                false,
            );

            // Test the invalid data ranges and proofs
            let invalid_proof_versions = vec![
                (100, 200, 300),
                (125, 175, 300),
                (100, 100, 300),
                (150, 150, 300),
                (200, 200, 300),
                (200, 200, 251),
            ];
            verify_can_service_transaction_requests(
                use_request_v2,
                &data_client_config,
                &data_summary,
                compression,
                invalid_proof_versions,
                false,
            );
        }
    }
}

#[test]
fn test_data_summary_service_transaction_outputs() {
    // Test both v1 and v2 transaction requests
    for use_request_v2 in [false, true] {
        // Create a data client config and data summary
        let data_client_config = AptosDataClientConfig::default();
        let data_summary = DataSummary {
            synced_ledger_info: Some(create_ledger_info_at_version(250)),
            transaction_outputs: Some(create_data_range(100, 200)),
            ..Default::default()
        };

        // Verify the different requests that can be serviced
        for compression in [true, false] {
            // Test the valid data ranges and proofs
            let valid_ranges_and_proofs = vec![
                (100, 200, 225),
                (125, 175, 225),
                (100, 100, 225),
                (150, 150, 225),
                (200, 200, 225),
                (200, 200, 250),
            ];
            verify_can_service_output_requests(
                use_request_v2,
                &data_client_config,
                &data_summary,
                compression,
                valid_ranges_and_proofs,
                true,
            );

            // Test the missing data ranges and proofs
            let missing_data_ranges = vec![
                (99, 200, 225),
                (100, 201, 225),
                (50, 250, 225),
                (50, 150, 225),
                (150, 250, 225),
            ];
            verify_can_service_output_requests(
                use_request_v2,
                &data_client_config,
                &data_summary,
                compression,
                missing_data_ranges,
                false,
            );

            // Test the valid data ranges and invalid proofs
            let invalid_proof_versions = vec![
                (100, 200, 300),
                (125, 175, 300),
                (100, 100, 300),
                (150, 150, 300),
                (200, 200, 300),
                (200, 200, 251),
            ];
            verify_can_service_output_requests(
                use_request_v2,
                &data_client_config,
                &data_summary,
                compression,
                invalid_proof_versions,
                false,
            );

            // Test the invalid data ranges and proofs
            let invalid_ranges = vec![(175, 125, 225), (201, 200, 201), (202, 200, 200)];
            verify_can_service_output_requests(
                use_request_v2,
                &data_client_config,
                &data_summary,
                compression,
                invalid_ranges,
                false,
            );
        }
    }
}

#[test]
fn test_data_summary_service_transactions_or_outputs() {
    // Test both v1 and v2 transaction requests
    for use_request_v2 in [false, true] {
        // Create a data client config and data summary
        let data_client_config = AptosDataClientConfig::default();
        let data_summary = DataSummary {
            synced_ledger_info: Some(create_ledger_info_at_version(250)),
            transactions: Some(create_data_range(50, 200)),
            transaction_outputs: Some(create_data_range(100, 250)),
            ..Default::default()
        };

        // Verify the different requests that can be serviced
        for compression in [true, false] {
            // Test the valid data ranges and proofs
            let valid_ranges_and_proofs = vec![
                (100, 200, 225),
                (125, 175, 225),
                (100, 100, 225),
                (150, 150, 225),
                (200, 200, 225),
                (200, 200, 250),
            ];
            verify_can_service_transaction_or_output_requests(
                use_request_v2,
                &data_client_config,
                &data_summary,
                compression,
                valid_ranges_and_proofs,
                true,
            );

            // Test the missing output ranges and proofs
            let missing_output_ranges = vec![(51, 200, 225), (99, 100, 225), (51, 71, 225)];
            verify_can_service_transaction_or_output_requests(
                use_request_v2,
                &data_client_config,
                &data_summary,
                compression,
                missing_output_ranges,
                false,
            );

            // Test the missing transaction ranges and proofs
            let missing_transaction_ranges =
                vec![(200, 202, 225), (150, 201, 225), (201, 225, 225)];
            verify_can_service_transaction_or_output_requests(
                use_request_v2,
                &data_client_config,
                &data_summary,
                compression,
                missing_transaction_ranges,
                false,
            );

            // Test the valid data ranges and invalid proofs
            let invalid_proof_versions = vec![
                (100, 200, 300),
                (125, 175, 300),
                (100, 100, 300),
                (150, 150, 300),
                (200, 200, 300),
                (200, 200, 251),
            ];
            verify_can_service_transaction_or_output_requests(
                use_request_v2,
                &data_client_config,
                &data_summary,
                compression,
                invalid_proof_versions,
                false,
            );

            // Test the invalid data ranges and proofs
            let invalid_ranges = vec![(175, 125, 225), (201, 200, 201), (202, 200, 200)];
            verify_can_service_transaction_or_output_requests(
                use_request_v2,
                &data_client_config,
                &data_summary,
                compression,
                invalid_ranges,
                false,
            );
        }
    }
}

#[test]
fn test_data_summary_service_state_chunk_request() {
    // Create a data client config and data summary
    let data_client_config = AptosDataClientConfig::default();
    let data_summary = DataSummary {
        synced_ledger_info: Some(create_ledger_info_at_version(250)),
        states: Some(create_data_range(100, 300)),
        ..Default::default()
    };

    // Verify the different requests that can be serviced
    for compression in [true, false] {
        // Test the valid request versions
        let valid_request_versions = vec![100, 200, 250];
        verify_can_service_state_chunk_requests(
            &data_client_config,
            &data_summary,
            compression,
            valid_request_versions,
            true,
        );

        // Test invalid request versions
        let invalid_request_versions = vec![50, 99, 251, 300];
        verify_can_service_state_chunk_requests(
            &data_client_config,
            &data_summary,
            compression,
            invalid_request_versions,
            false,
        );
    }
}

#[test]
fn test_protocol_metadata_service() {
    // Create the protocol metadata
    let metadata = ProtocolMetadata {
        max_transaction_chunk_size: 100,
        max_epoch_chunk_size: 100,
        max_transaction_output_chunk_size: 100,
        max_state_chunk_size: 100,
    };

    // Verify the different requests that can be serviced
    for compression in [true, false] {
        // Requests with smaller chunk sizes can be serviced
        assert!(metadata.can_service(&create_transactions_request(200, 100, 101, compression)));
        assert!(metadata.can_service(&create_epoch_ending_request(100, 199, compression)));
        assert!(metadata.can_service(&create_outputs_request(200, 100, 100, compression)));
        assert!(metadata.can_service(&create_state_values_request(200, 100, 199, compression)));

        // Requests with larger chunk sizes (beyond the max) can also be serviced
        assert!(metadata.can_service(&create_transactions_request(200, 100, 1000, compression)));
        assert!(metadata.can_service(&create_epoch_ending_request(100, 10000, compression)));
        assert!(metadata.can_service(&create_outputs_request(200, 100, 9999989, compression)));
        assert!(metadata.can_service(&create_state_values_request(200, 100, 200, compression)));
    }
}

#[test]
fn test_is_transaction_data_v2_request() {
    // Create transaction data v1 requests
    let transactions_request = create_transactions_request(200, 100, 101, false);
    let outputs_request = create_outputs_request(200, 100, 101, false);
    let transactions_or_outputs_request =
        create_transactions_or_outputs_request(200, 100, 101, false);

    // Verify that none of them are v2 requests
    for request in [
        transactions_request,
        outputs_request,
        transactions_or_outputs_request,
    ] {
        assert!(!request.data_request.is_transaction_data_v2_request());
    }

    // Create transaction data v2 requests
    let transactions_request_v2 = create_transactions_request_v2(200, 100, 101, false);
    let outputs_request_v2 = create_outputs_request_v2(200, 100, 101, false);
    let transactions_or_outputs_request_v2 =
        create_transactions_or_outputs_request_v2(200, 100, 101, false);

    // Verify that all of them are v2 requests
    for request in [
        transactions_request_v2,
        outputs_request_v2,
        transactions_or_outputs_request_v2,
    ] {
        assert!(request.data_request.is_transaction_data_v2_request());
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn test_data_summary_length_invariant(range in any::<CompleteDataRange<u64>>()) {
        let _ = range.len(); // This should not panic
    }
}

/// Creates a new data range using the specified bounds
fn create_data_range(lowest: u64, highest: u64) -> CompleteDataRange<u64> {
    CompleteDataRange::new(lowest, highest).unwrap()
}

/// Creates a request for epoch ending ledger infos
fn create_epoch_ending_request(
    start: Epoch,
    end: Epoch,
    use_compression: bool,
) -> StorageServiceRequest {
    let data_request = DataRequest::GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest {
        start_epoch: start,
        expected_end_epoch: end,
    });
    StorageServiceRequest::new(data_request, use_compression)
}

/// Creates a new ledger info at the given version
fn create_ledger_info_at_version(version: Version) -> LedgerInfoWithSignatures {
    create_ledger_info_at_version_and_timestamp(version, 0)
}

/// Creates a new ledger info at the given version and timestamp
fn create_ledger_info_at_version_and_timestamp(
    version: Version,
    timestamp_usecs: u64,
) -> LedgerInfoWithSignatures {
    LedgerInfoWithSignatures::new(
        LedgerInfo::new(
            BlockInfo::new(
                0,
                0,
                HashValue::zero(),
                HashValue::zero(),
                version,
                timestamp_usecs,
                None,
            ),
            HashValue::zero(),
        ),
        AggregateSignature::empty(),
    )
}

/// Creates a new optimistic request
fn create_optimistic_fetch_request(
    known_version: u64,
    use_compression: bool,
) -> StorageServiceRequest {
    // Generate a random number
    let random_number = get_random_u64();

    // Determine the data request type based on the random number
    let data_request = if random_number % 3 == 0 {
        DataRequest::GetNewTransactionsWithProof(NewTransactionsWithProofRequest {
            known_version,
            known_epoch: get_random_u64(),
            include_events: false,
        })
    } else if random_number % 3 == 1 {
        DataRequest::GetNewTransactionOutputsWithProof(NewTransactionOutputsWithProofRequest {
            known_version,
            known_epoch: get_random_u64(),
        })
    } else {
        DataRequest::GetNewTransactionsOrOutputsWithProof(
            NewTransactionsOrOutputsWithProofRequest {
                known_version,
                known_epoch: get_random_u64(),
                include_events: false,
                max_num_output_reductions: get_random_u64(),
            },
        )
    };
    StorageServiceRequest::new(data_request, use_compression)
}

/// Creates a new optimistic request (v2)
fn create_optimistic_fetch_request_v2(
    known_version: u64,
    use_compression: bool,
) -> StorageServiceRequest {
    // Generate a random number
    let random_number = get_random_u64();

    // Determine the data request type based on the random number
    let data_request = if random_number % 3 == 0 {
        DataRequest::get_new_transaction_data_with_proof(
            known_version,
            get_random_u64(),
            false,
            get_random_u64(),
        )
    } else if random_number % 3 == 1 {
        DataRequest::get_new_transaction_output_data_with_proof(
            known_version,
            get_random_u64(),
            get_random_u64(),
        )
    } else {
        DataRequest::get_new_transaction_or_output_data_with_proof(
            known_version,
            get_random_u64(),
            false,
            get_random_u64(),
        )
    };
    StorageServiceRequest::new(data_request, use_compression)
}

/// Creates a request for transaction outputs
fn create_outputs_request(
    proof_version: Version,
    start_version: Version,
    end_version: Version,
    use_compression: bool,
) -> StorageServiceRequest {
    let data_request =
        DataRequest::GetTransactionOutputsWithProof(TransactionOutputsWithProofRequest {
            proof_version,
            start_version,
            end_version,
        });
    StorageServiceRequest::new(data_request, use_compression)
}

/// Creates a request for transaction outputs (v2)
fn create_outputs_request_v2(
    proof_version: Version,
    start_version: Version,
    end_version: Version,
    use_compression: bool,
) -> StorageServiceRequest {
    let data_request = DataRequest::get_transaction_output_data_with_proof(
        proof_version,
        start_version,
        end_version,
        get_random_u64(),
    );
    StorageServiceRequest::new(data_request, use_compression)
}

/// Creates a new subscription request
fn create_subscription_request(known_version: u64, use_compression: bool) -> StorageServiceRequest {
    // Create a new subscription stream metadata
    let subscription_stream_metadata = SubscriptionStreamMetadata {
        known_version_at_stream_start: known_version,
        known_epoch_at_stream_start: get_random_u64(),
        subscription_stream_id: get_random_u64(),
    };

    // Generate a random number
    let random_number = get_random_u64();

    // Determine the data request type based on the random number
    let data_request = if random_number % 3 == 0 {
        DataRequest::SubscribeTransactionsWithProof(SubscribeTransactionsWithProofRequest {
            subscription_stream_metadata,
            include_events: false,
            subscription_stream_index: get_random_u64(),
        })
    } else if random_number % 3 == 1 {
        DataRequest::SubscribeTransactionOutputsWithProof(
            SubscribeTransactionOutputsWithProofRequest {
                subscription_stream_metadata,
                subscription_stream_index: get_random_u64(),
            },
        )
    } else {
        DataRequest::SubscribeTransactionsOrOutputsWithProof(
            SubscribeTransactionsOrOutputsWithProofRequest {
                subscription_stream_metadata,
                include_events: false,
                max_num_output_reductions: get_random_u64(),
                subscription_stream_index: get_random_u64(),
            },
        )
    };
    StorageServiceRequest::new(data_request, use_compression)
}

/// Creates a new subscription request (v2)
fn create_subscription_request_v2(
    known_version: u64,
    use_compression: bool,
) -> StorageServiceRequest {
    // Create a new subscription stream metadata
    let subscription_stream_metadata = SubscriptionStreamMetadata {
        known_version_at_stream_start: known_version,
        known_epoch_at_stream_start: get_random_u64(),
        subscription_stream_id: get_random_u64(),
    };

    // Generate a random number
    let random_number = get_random_u64();

    // Determine the data request type based on the random number
    let data_request = if random_number % 3 == 0 {
        DataRequest::subscribe_transaction_data_with_proof(
            subscription_stream_metadata,
            get_random_u64(),
            false,
            get_random_u64(),
        )
    } else if random_number % 3 == 1 {
        DataRequest::subscribe_transaction_output_data_with_proof(
            subscription_stream_metadata,
            get_random_u64(),
            get_random_u64(),
        )
    } else {
        DataRequest::subscribe_transaction_or_output_data_with_proof(
            subscription_stream_metadata,
            get_random_u64(),
            false,
            get_random_u64(),
        )
    };
    StorageServiceRequest::new(data_request, use_compression)
}

/// Creates a request for transactions
fn create_transactions_request(
    proof_version: Version,
    start_version: Version,
    end_version: Version,
    use_compression: bool,
) -> StorageServiceRequest {
    let data_request = DataRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
        proof_version,
        start_version,
        end_version,
        include_events: true,
    });
    StorageServiceRequest::new(data_request, use_compression)
}

/// Creates a request for transactions (v2)
fn create_transactions_request_v2(
    proof_version: Version,
    start_version: Version,
    end_version: Version,
    use_compression: bool,
) -> StorageServiceRequest {
    let data_request = DataRequest::get_transaction_data_with_proof(
        proof_version,
        start_version,
        end_version,
        true,
        get_random_u64(),
    );
    StorageServiceRequest::new(data_request, use_compression)
}

/// Creates a request for transactions or outputs
fn create_transactions_or_outputs_request(
    proof_version: Version,
    start_version: Version,
    end_version: Version,
    use_compression: bool,
) -> StorageServiceRequest {
    let data_request =
        DataRequest::GetTransactionsOrOutputsWithProof(TransactionsOrOutputsWithProofRequest {
            proof_version,
            start_version,
            end_version,
            include_events: true,
            max_num_output_reductions: 3,
        });
    StorageServiceRequest::new(data_request, use_compression)
}

/// Creates a request for transactions or outputs (v2)
fn create_transactions_or_outputs_request_v2(
    proof_version: Version,
    start_version: Version,
    end_version: Version,
    use_compression: bool,
) -> StorageServiceRequest {
    let data_request = DataRequest::get_transaction_or_output_data_with_proof(
        proof_version,
        start_version,
        end_version,
        false,
        get_random_u64(),
    );
    StorageServiceRequest::new(data_request, use_compression)
}

/// Creates a request for state values
fn create_state_values_request(
    version: Version,
    start_index: u64,
    end_index: u64,
    use_compression: bool,
) -> StorageServiceRequest {
    let data_request = DataRequest::GetStateValuesWithProof(StateValuesWithProofRequest {
        version,
        start_index,
        end_index,
    });
    StorageServiceRequest::new(data_request, use_compression)
}

/// Creates a request for state values at a given version
fn create_state_values_request_at_version(
    version: Version,
    use_compression: bool,
) -> StorageServiceRequest {
    create_state_values_request(version, 0, 1000, use_compression)
}

/// Generates a random u64
fn get_random_u64() -> u64 {
    thread_rng().r#gen()
}

/// Verifies the serviceability of the epoch ending request ranges against
/// the specified data summary. If `expect_service` is true, then the
/// request should be serviceable.
fn verify_can_service_epoch_ending_requests(
    data_client_config: &AptosDataClientConfig,
    data_summary: &DataSummary,
    compression: bool,
    epoch_ranges: Vec<(Epoch, Epoch)>,
    expect_service: bool,
) {
    for (start_epoch, end_epoch) in epoch_ranges {
        // Create the epoch ending request
        let request = create_epoch_ending_request(start_epoch, end_epoch, compression);

        // Verify the serviceability of the request
        verify_serviceability(
            data_client_config,
            data_summary,
            None,
            request,
            expect_service,
        );
    }
}

/// Verifies the serviceability of the optimistic fetch versions against
/// the specified data summary. If `expect_service` is true, then the
/// request should be serviceable.
fn verify_can_service_optimistic_fetch_requests(
    use_request_v2: bool,
    data_client_config: &AptosDataClientConfig,
    data_summary: &DataSummary,
    time_service: TimeService,
    compression: bool,
    known_versions: Vec<Version>,
    expect_service: bool,
) {
    for known_version in known_versions {
        // Create the optimistic fetch request
        let request = if use_request_v2 {
            create_optimistic_fetch_request_v2(known_version, compression)
        } else {
            create_optimistic_fetch_request(known_version, compression)
        };

        // Verify the serviceability of the request
        verify_serviceability(
            data_client_config,
            data_summary,
            Some(time_service.clone()),
            request,
            expect_service,
        );
    }
}

/// Verifies the serviceability of the state chunk request versions
/// against the specified data summary. If `expect_service` is true,
/// then the request should be serviceable.
fn verify_can_service_state_chunk_requests(
    data_client_config: &AptosDataClientConfig,
    data_summary: &DataSummary,
    use_compression: bool,
    versions: Vec<u64>,
    expect_service: bool,
) {
    for version in versions {
        // Create the state chunk request
        let request = create_state_values_request_at_version(version, use_compression);

        // Verify the serviceability of the request
        verify_serviceability(
            data_client_config,
            data_summary,
            None,
            request,
            expect_service,
        );
    }
}

/// Verifies the serviceability of the subscription versions against
/// the specified data summary. If `expect_service` is true, then the
/// request should be serviceable.
fn verify_can_service_subscription_requests(
    use_request_v2: bool,
    data_client_config: &AptosDataClientConfig,
    data_summary: &DataSummary,
    time_service: TimeService,
    compression: bool,
    known_versions: Vec<Version>,
    expect_service: bool,
) {
    for known_version in known_versions {
        // Create the subscription request
        let request = if use_request_v2 {
            create_subscription_request_v2(known_version, compression)
        } else {
            create_subscription_request(known_version, compression)
        };

        // Verify the serviceability of the request
        verify_serviceability(
            data_client_config,
            data_summary,
            Some(time_service.clone()),
            request,
            expect_service,
        );
    }
}

/// Verifies the serviceability of the transaction request ranges against
/// the specified data summary. If `expect_service` is true, then the
/// request should be serviceable.
fn verify_can_service_transaction_requests(
    use_request_v2: bool,
    data_client_config: &AptosDataClientConfig,
    data_summary: &DataSummary,
    use_compression: bool,
    transaction_ranges: Vec<(u64, u64, u64)>,
    expect_service: bool,
) {
    for (start_version, end_version, proof_version) in transaction_ranges {
        // Create the transaction request
        let request = if use_request_v2 {
            create_transactions_request_v2(
                proof_version,
                start_version,
                end_version,
                use_compression,
            )
        } else {
            create_transactions_request(proof_version, start_version, end_version, use_compression)
        };

        // Verify the serviceability of the request
        verify_serviceability(
            data_client_config,
            data_summary,
            None,
            request,
            expect_service,
        );
    }
}

/// Verifies the serviceability of the transaction or output request
/// ranges against the specified data summary. If `expect_service` is
/// true, then the request should be serviceable.
fn verify_can_service_transaction_or_output_requests(
    use_request_v2: bool,
    data_client_config: &AptosDataClientConfig,
    data_summary: &DataSummary,
    use_compression: bool,
    transaction_ranges: Vec<(u64, u64, u64)>,
    expect_service: bool,
) {
    for (start_version, end_version, proof_version) in transaction_ranges {
        // Create the transaction or output request
        let request = if use_request_v2 {
            create_transactions_or_outputs_request_v2(
                proof_version,
                start_version,
                end_version,
                use_compression,
            )
        } else {
            create_transactions_or_outputs_request(
                proof_version,
                start_version,
                end_version,
                use_compression,
            )
        };

        // Verify the serviceability of the request
        verify_serviceability(
            data_client_config,
            data_summary,
            None,
            request,
            expect_service,
        );
    }
}

/// Verifies the serviceability of the output request ranges against
/// the specified data summary. If `expect_service` is true, then the
/// request should be serviceable.
fn verify_can_service_output_requests(
    use_request_v2: bool,
    data_client_config: &AptosDataClientConfig,
    data_summary: &DataSummary,
    use_compression: bool,
    output_ranges: Vec<(u64, u64, u64)>,
    expect_service: bool,
) {
    for (start_version, end_version, proof_version) in output_ranges {
        // Create the output request
        let request = if use_request_v2 {
            create_outputs_request_v2(proof_version, start_version, end_version, use_compression)
        } else {
            create_outputs_request(proof_version, start_version, end_version, use_compression)
        };

        // Verify the serviceability of the request
        verify_serviceability(
            data_client_config,
            data_summary,
            None,
            request,
            expect_service,
        );
    }
}

/// A simple helper method to verify the serviceability of a request
fn verify_serviceability(
    data_client_config: &AptosDataClientConfig,
    data_summary: &DataSummary,
    time_service: Option<TimeService>,
    request: StorageServiceRequest,
    expect_service: bool,
) {
    let time_service = time_service.unwrap_or(TimeService::mock());
    let can_service = data_summary.can_service(data_client_config, time_service, &request);

    // Assert that the serviceability matches the expectation
    if expect_service {
        assert!(can_service);
    } else {
        assert!(!can_service);
    }
}
