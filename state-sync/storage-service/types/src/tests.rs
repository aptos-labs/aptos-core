// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    requests::{
        DataRequest, EpochEndingLedgerInfoRequest, NewTransactionOutputsWithProofRequest,
        NewTransactionsOrOutputsWithProofRequest, NewTransactionsWithProofRequest,
        StateValuesWithProofRequest, TransactionOutputsWithProofRequest,
        TransactionsOrOutputsWithProofRequest, TransactionsWithProofRequest,
    },
    responses::{CompleteDataRange, DataSummary, ProtocolMetadata},
    Epoch, StorageServiceRequest,
};
use aptos_config::config::AptosDataClientConfig;
use aptos_crypto::hash::HashValue;
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
    // Create a data client config with the specified max optimistic fetch lag
    let max_optimistic_fetch_version_lag = 1000;
    let data_client_config = AptosDataClientConfig {
        max_optimistic_fetch_version_lag,
        ..Default::default()
    };

    // Create a data summary with the specified synced ledger info version
    let highest_synced_version = 50_000;
    let data_summary = DataSummary {
        synced_ledger_info: Some(create_ledger_info_at_version(highest_synced_version)),
        ..Default::default()
    };

    // Verify the different requests that can be serviced
    for compression in [true, false] {
        // Test the known versions that are within the optimistic fetch lag
        let known_versions = vec![
            highest_synced_version,
            highest_synced_version + (max_optimistic_fetch_version_lag / 2),
            highest_synced_version + max_optimistic_fetch_version_lag - 1,
        ];
        verify_can_service_optimistic_fetch_requests(
            &data_client_config,
            &data_summary,
            compression,
            known_versions,
            true,
        );

        // Test the known versions that are outside the optimistic fetch lag
        let known_versions = vec![
            highest_synced_version + max_optimistic_fetch_version_lag,
            highest_synced_version + max_optimistic_fetch_version_lag + 1,
            highest_synced_version + (max_optimistic_fetch_version_lag * 2),
        ];
        verify_can_service_optimistic_fetch_requests(
            &data_client_config,
            &data_summary,
            compression,
            known_versions,
            false,
        );
    }
}

#[test]
fn test_data_summary_service_transactions() {
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
            &data_client_config,
            &data_summary,
            compression,
            invalid_proof_versions,
            false,
        );
    }
}

#[test]
fn test_data_summary_service_transaction_outputs() {
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
            &data_client_config,
            &data_summary,
            compression,
            invalid_proof_versions,
            false,
        );

        // Test the invalid data ranges and proofs
        let invalid_ranges = vec![(175, 125, 225), (201, 200, 201), (202, 200, 200)];
        verify_can_service_output_requests(
            &data_client_config,
            &data_summary,
            compression,
            invalid_ranges,
            false,
        );
    }
}

#[test]
fn test_data_summary_service_transactions_or_outputs() {
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
            &data_client_config,
            &data_summary,
            compression,
            valid_ranges_and_proofs,
            true,
        );

        // Test the missing output ranges and proofs
        let missing_output_ranges = vec![(51, 200, 225), (99, 100, 225), (51, 71, 225)];
        verify_can_service_transaction_or_output_requests(
            &data_client_config,
            &data_summary,
            compression,
            missing_output_ranges,
            false,
        );

        // Test the missing transaction ranges and proofs
        let missing_transaction_ranges = vec![(200, 202, 225), (150, 201, 225), (201, 225, 225)];
        verify_can_service_transaction_or_output_requests(
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
            &data_client_config,
            &data_summary,
            compression,
            invalid_proof_versions,
            false,
        );

        // Test the invalid data ranges and proofs
        let invalid_ranges = vec![(175, 125, 225), (201, 200, 201), (202, 200, 200)];
        verify_can_service_transaction_or_output_requests(
            &data_client_config,
            &data_summary,
            compression,
            invalid_ranges,
            false,
        );
    }
}

#[test]
fn test_data_summary_can_service_state_chunk_request() {
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
    LedgerInfoWithSignatures::new(
        LedgerInfo::new(
            BlockInfo::new(0, 0, HashValue::zero(), HashValue::zero(), version, 0, None),
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
    let random_number: u64 = thread_rng().gen();

    // Determine the data request type based on the random number
    let data_request = if random_number % 3 == 0 {
        DataRequest::GetNewTransactionsWithProof(NewTransactionsWithProofRequest {
            known_version,
            known_epoch: 1,
            include_events: false,
        })
    } else if random_number % 3 == 1 {
        DataRequest::GetNewTransactionOutputsWithProof(NewTransactionOutputsWithProofRequest {
            known_version,
            known_epoch: 1,
        })
    } else {
        DataRequest::GetNewTransactionsOrOutputsWithProof(
            NewTransactionsOrOutputsWithProofRequest {
                known_version,
                known_epoch: 1,
                include_events: false,
                max_num_output_reductions: 0,
            },
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

/// Creates a request for transactions
fn create_transactions_request(
    proof: Version,
    start: Version,
    end: Version,
    use_compression: bool,
) -> StorageServiceRequest {
    let data_request = DataRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
        proof_version: proof,
        start_version: start,
        end_version: end,
        include_events: true,
    });
    StorageServiceRequest::new(data_request, use_compression)
}

/// Creates a request for transactions or outputs
fn create_transactions_or_outputs_request(
    proof: Version,
    start: Version,
    end: Version,
    use_compression: bool,
) -> StorageServiceRequest {
    let data_request =
        DataRequest::GetTransactionsOrOutputsWithProof(TransactionsOrOutputsWithProofRequest {
            proof_version: proof,
            start_version: start,
            end_version: end,
            include_events: true,
            max_num_output_reductions: 3,
        });
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
        verify_serviceability(data_client_config, data_summary, request, expect_service);
    }
}

/// Verifies the serviceability of the optimistic fetch versions against
/// the specified data summary. If `expect_service` is true, then the
/// request should be serviceable.
fn verify_can_service_optimistic_fetch_requests(
    data_client_config: &AptosDataClientConfig,
    data_summary: &DataSummary,
    compression: bool,
    known_versions: Vec<Version>,
    expect_service: bool,
) {
    for known_version in known_versions {
        // Create the optimistic fetch request
        let request = create_optimistic_fetch_request(known_version, compression);

        // Verify the serviceability of the request
        verify_serviceability(data_client_config, data_summary, request, expect_service);
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
        verify_serviceability(data_client_config, data_summary, request, expect_service);
    }
}

/// Verifies the serviceability of the transaction request ranges against
/// the specified data summary. If `expect_service` is true, then the
/// request should be serviceable.
fn verify_can_service_transaction_requests(
    data_client_config: &AptosDataClientConfig,
    data_summary: &DataSummary,
    use_compression: bool,
    transaction_ranges: Vec<(u64, u64, u64)>,
    expect_service: bool,
) {
    for (start_version, end_version, proof_version) in transaction_ranges {
        // Create the transaction request
        let request =
            create_transactions_request(proof_version, start_version, end_version, use_compression);

        // Verify the serviceability of the request
        verify_serviceability(data_client_config, data_summary, request, expect_service);
    }
}

/// Verifies the serviceability of the transaction or output request
/// ranges against the specified data summary. If `expect_service` is
/// true, then the request should be serviceable.
fn verify_can_service_transaction_or_output_requests(
    data_client_config: &AptosDataClientConfig,
    data_summary: &DataSummary,
    use_compression: bool,
    transaction_ranges: Vec<(u64, u64, u64)>,
    expect_service: bool,
) {
    for (start_version, end_version, proof_version) in transaction_ranges {
        // Create the transaction or output request
        let request = create_transactions_or_outputs_request(
            proof_version,
            start_version,
            end_version,
            use_compression,
        );

        // Verify the serviceability of the request
        verify_serviceability(data_client_config, data_summary, request, expect_service);
    }
}

/// Verifies the serviceability of the output request ranges against
/// the specified data summary. If `expect_service` is true, then the
/// request should be serviceable.
fn verify_can_service_output_requests(
    data_client_config: &AptosDataClientConfig,
    data_summary: &DataSummary,
    use_compression: bool,
    output_ranges: Vec<(u64, u64, u64)>,
    expect_service: bool,
) {
    for (start_version, end_version, proof_version) in output_ranges {
        // Create the output request
        let request =
            create_outputs_request(proof_version, start_version, end_version, use_compression);

        // Verify the serviceability of the request
        verify_serviceability(data_client_config, data_summary, request, expect_service);
    }
}

/// A simple helper method to verify the serviceability of a request
fn verify_serviceability(
    data_client_config: &AptosDataClientConfig,
    data_summary: &DataSummary,
    request: StorageServiceRequest,
    expect_service: bool,
) {
    let can_service = data_summary.can_service(data_client_config, &request);

    // Assert that the serviceability matches the expectation
    if expect_service {
        assert!(can_service);
    } else {
        assert!(!can_service);
    }
}
