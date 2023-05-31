// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::tests::{
    mock,
    mock::{MockClient, MockDatabaseReader},
    utils,
};
use aptos_config::config::StorageServiceConfig;
use aptos_storage_service_notifications::StorageServiceNotificationSender;
use aptos_storage_service_types::{
    requests::DataRequest,
    responses::{
        CompleteDataRange, DataResponse, DataSummary, ProtocolMetadata, StorageServerSummary,
        StorageServiceResponse,
    },
    StorageServiceError,
};
use aptos_types::{ledger_info::LedgerInfoWithSignatures, transaction::Version};

#[tokio::test]
async fn test_get_storage_server_summary_advance_time() {
    // Create test data
    let highest_version = 506;
    let highest_epoch = 30;
    let lowest_version = 101;
    let state_prune_window = 50;
    let highest_ledger_info =
        utils::create_test_ledger_info_with_sigs(highest_epoch, highest_version);

    // Create the mock db reader
    let db_reader = create_db_reader_with_expectations(
        lowest_version,
        state_prune_window,
        highest_ledger_info.clone(),
    );

    // Create the storage client and server
    let (mut mock_client, service, _, mock_time, _) = MockClient::new(Some(db_reader), None);
    let storage_summary_cache = service.cached_storage_server_summary.clone();
    tokio::spawn(service.start());

    // Test multiple updates to the storage summary cache
    for _ in 0..100 {
        // Fetch the storage summary and verify we get a default summary response
        let response = get_storage_server_summary(&mut mock_client, true)
            .await
            .unwrap();
        let default_response = StorageServiceResponse::new(
            DataResponse::StorageServerSummary(StorageServerSummary::default()),
            true,
        )
        .unwrap();
        assert_eq!(response, default_response);

        // Elapse enough time to force a cache update
        utils::advance_storage_refresh_time(&mock_time).await;

        // Process another request to fetch the storage summary
        let response = get_storage_server_summary(&mut mock_client, true)
            .await
            .unwrap();

        // Verify the response is correct (after the cache update)
        verify_server_summary_response(
            highest_version,
            highest_epoch,
            lowest_version,
            state_prune_window,
            highest_ledger_info.clone(),
            response,
        );

        // Manually overwrite the storage summary cache
        *storage_summary_cache.write() = StorageServerSummary::default();
    }
}

#[tokio::test]
async fn test_get_storage_server_summary_notification() {
    // Create test data
    let highest_version = 1000;
    let highest_epoch = 430;
    let lowest_version = 11;
    let state_prune_window = 200;
    let highest_ledger_info =
        utils::create_test_ledger_info_with_sigs(highest_epoch, highest_version);

    // Create the mock db reader
    let db_reader = create_db_reader_with_expectations(
        lowest_version,
        state_prune_window,
        highest_ledger_info.clone(),
    );

    // Create the storage client and server
    let (mut mock_client, service, storage_service_notifier, _, _) =
        MockClient::new(Some(db_reader), None);
    let storage_summary_cache = service.cached_storage_server_summary.clone();
    tokio::spawn(service.start());

    // Test multiple updates to the storage summary cache
    for _ in 0..100 {
        // Fetch the storage summary and verify we get a default summary response
        let response = get_storage_server_summary(&mut mock_client, true)
            .await
            .unwrap();
        let default_response = StorageServiceResponse::new(
            DataResponse::StorageServerSummary(StorageServerSummary::default()),
            true,
        )
        .unwrap();
        assert_eq!(response, default_response);

        // Send a notification to the storage service. This will cause the cache to be updated.
        storage_service_notifier.notify_new_commit(1).await.unwrap();

        // Process another request to fetch the storage summary
        let response = get_storage_server_summary(&mut mock_client, true)
            .await
            .unwrap();

        // Verify the response is correct (after the cache update)
        verify_server_summary_response(
            highest_version,
            highest_epoch,
            lowest_version,
            state_prune_window,
            highest_ledger_info.clone(),
            response,
        );

        // Manually overwrite the storage summary cache
        *storage_summary_cache.write() = StorageServerSummary::default();
    }
}

/// Creates a mock database reader with the necessary
/// expectations to satisfy the storage server summary request.
fn create_db_reader_with_expectations(
    lowest_version: Version,
    state_prune_window: usize,
    highest_ledger_info: LedgerInfoWithSignatures,
) -> MockDatabaseReader {
    // Create the mock reader
    let mut db_reader = mock::create_mock_db_reader();

    // Set the read call expectations
    db_reader
        .expect_get_latest_ledger_info()
        .returning(move || Ok(highest_ledger_info.clone()));
    db_reader
        .expect_get_first_txn_version()
        .returning(move || Ok(Some(lowest_version)));
    db_reader
        .expect_get_first_write_set_version()
        .returning(move || Ok(Some(lowest_version)));
    db_reader
        .expect_get_epoch_snapshot_prune_window()
        .returning(move || Ok(state_prune_window));
    db_reader
        .expect_is_state_merkle_pruner_enabled()
        .returning(move || Ok(true));
    db_reader
}

/// Sends a storage summary request and processes the response
async fn get_storage_server_summary(
    mock_client: &mut MockClient,
    use_compression: bool,
) -> Result<StorageServiceResponse, StorageServiceError> {
    let data_request = DataRequest::GetStorageServerSummary;
    utils::send_storage_request(mock_client, use_compression, data_request).await
}

/// Verifies that the given storage server summary response is valid
fn verify_server_summary_response(
    highest_version: u64,
    highest_epoch: u64,
    lowest_version: Version,
    state_prune_window: usize,
    highest_ledger_info: LedgerInfoWithSignatures,
    response: StorageServiceResponse,
) {
    // Create the expected response
    let default_storage_config = StorageServiceConfig::default();
    let expected_server_summary = StorageServerSummary {
        protocol_metadata: ProtocolMetadata {
            max_epoch_chunk_size: default_storage_config.max_epoch_chunk_size,
            max_state_chunk_size: default_storage_config.max_state_chunk_size,
            max_transaction_chunk_size: default_storage_config.max_transaction_chunk_size,
            max_transaction_output_chunk_size: default_storage_config
                .max_transaction_output_chunk_size,
        },
        data_summary: DataSummary {
            synced_ledger_info: Some(highest_ledger_info),
            epoch_ending_ledger_infos: Some(CompleteDataRange::from_genesis(highest_epoch - 1)),
            transactions: Some(CompleteDataRange::new(lowest_version, highest_version).unwrap()),
            transaction_outputs: Some(
                CompleteDataRange::new(lowest_version, highest_version).unwrap(),
            ),
            states: Some(
                CompleteDataRange::new(
                    highest_version - state_prune_window as u64 + 1,
                    highest_version,
                )
                .unwrap(),
            ),
        },
    };

    // Verify the response matches the expected response
    assert_eq!(
        response,
        StorageServiceResponse::new(
            DataResponse::StorageServerSummary(expected_server_summary),
            true,
        )
        .unwrap()
    );
}
