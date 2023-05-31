// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::tests::{mock, mock::MockClient, utils};
use aptos_config::config::StorageServiceConfig;
use aptos_storage_service_types::{
    requests::DataRequest,
    responses::{
        CompleteDataRange, DataResponse, DataSummary, ProtocolMetadata, StorageServerSummary,
        StorageServiceResponse,
    },
    StorageServiceError,
};

#[tokio::test]
async fn test_get_storage_server_summary() {
    // Create test data
    let highest_version = 506;
    let highest_epoch = 30;
    let lowest_version = 101;
    let state_prune_window = 50;
    let highest_ledger_info =
        utils::create_test_ledger_info_with_sigs(highest_epoch, highest_version);

    // Create the mock db reader
    let mut db_reader = mock::create_mock_db_reader();
    let highest_ledger_info_clone = highest_ledger_info.clone();
    db_reader
        .expect_get_latest_ledger_info()
        .times(1)
        .returning(move || Ok(highest_ledger_info_clone.clone()));
    db_reader
        .expect_get_first_txn_version()
        .times(1)
        .returning(move || Ok(Some(lowest_version)));
    db_reader
        .expect_get_first_write_set_version()
        .times(1)
        .returning(move || Ok(Some(lowest_version)));
    db_reader
        .expect_get_epoch_snapshot_prune_window()
        .times(1)
        .returning(move || Ok(state_prune_window));
    db_reader
        .expect_is_state_merkle_pruner_enabled()
        .returning(move || Ok(true));

    // Create the storage client and server
    let (mut mock_client, service, mock_time, _) = MockClient::new(Some(db_reader), None);
    tokio::spawn(service.start());

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
    assert_eq!(
        response,
        StorageServiceResponse::new(
            DataResponse::StorageServerSummary(expected_server_summary),
            true
        )
        .unwrap()
    );
}

/// Sends a storage summary request and processes the response
async fn get_storage_server_summary(
    mock_client: &mut MockClient,
    use_compression: bool,
) -> Result<StorageServiceResponse, StorageServiceError> {
    let data_request = DataRequest::GetStorageServerSummary;
    utils::send_storage_request(mock_client, use_compression, data_request).await
}
