// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::tests::{mock, mock::MockClient, utils};
use anyhow::format_err;
use velor_storage_interface::VelorDbError;
use velor_storage_service_types::{
    responses::{DataResponse, StorageServiceResponse},
    StorageServiceError,
};
use claims::assert_matches;
use mockall::predicate::eq;

#[tokio::test]
async fn test_get_number_of_states_at_version() {
    // Create test data
    let version = 101;
    let number_of_states: u64 = 560;

    // Create the mock db reader
    let mut db_reader = mock::create_mock_db_reader();
    db_reader
        .expect_get_state_item_count()
        .times(1)
        .with(eq(version))
        .returning(move |_| Ok(number_of_states as usize));

    // Create the storage client and server
    let (mut mock_client, mut service, _, _, _) = MockClient::new(Some(db_reader), None);
    utils::update_storage_server_summary(&mut service, version, 10);
    tokio::spawn(service.start());

    // Process a request to fetch the number of states at a version
    let response = utils::get_number_of_states(&mut mock_client, version, false)
        .await
        .unwrap();

    // Verify the response is correct
    assert_matches!(response, StorageServiceResponse::RawResponse(_));
    assert_eq!(
        response.get_data_response().unwrap(),
        DataResponse::NumberOfStatesAtVersion(number_of_states)
    );
}

#[tokio::test]
async fn test_get_number_of_states_at_version_not_serviceable() {
    // Create test data
    let version = 101;

    // Create the storage client and server (that cannot service the request)
    let (mut mock_client, mut service, _, _, _) = MockClient::new(None, None);
    utils::update_storage_server_summary(&mut service, version - 1, 10);
    tokio::spawn(service.start());

    // Process a request to fetch the number of states at a version
    let response = utils::get_number_of_states(&mut mock_client, version, false)
        .await
        .unwrap_err();

    // Verify the request is not serviceable
    assert_matches!(response, StorageServiceError::InvalidRequest(_));
}

#[tokio::test]
async fn test_get_number_of_states_at_version_invalid() {
    // Create test data
    let version = 1;

    // Create the mock db reader
    let mut db_reader = mock::create_mock_db_reader();
    db_reader
        .expect_get_state_item_count()
        .times(1)
        .with(eq(version))
        .returning(move |_| {
            Err(VelorDbError::NotFound(
                format_err!("Version does not exist!").to_string(),
            ))
        });

    // Create the storage client and server
    let (mut mock_client, mut service, _, _, _) = MockClient::new(Some(db_reader), None);
    utils::update_storage_server_summary(&mut service, version, 10);
    tokio::spawn(service.start());

    // Process a request to fetch the number of states at a version
    let response = utils::get_number_of_states(&mut mock_client, version, false)
        .await
        .unwrap_err();

    // Verify the response is correct
    assert_matches!(response, StorageServiceError::InternalError(_));
}
