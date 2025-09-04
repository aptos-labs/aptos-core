// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::tests::{mock::MockClient, utils};
use velor_storage_service_types::{
    requests::DataRequest,
    responses::{DataResponse, ServerProtocolVersion, StorageServiceResponse},
};
use claims::assert_matches;

// Useful test constants
const PROTOCOL_VERSION: u64 = 1;

#[tokio::test]
async fn test_get_server_protocol_version() {
    // Create the storage client and server
    let (mut mock_client, service, _, _, _) = MockClient::new(None, None);
    tokio::spawn(service.start());

    // Process a request to fetch the protocol version
    let response = get_protocol_version(&mut mock_client, true).await;

    // Verify the response is correct
    let expected_data_response = DataResponse::ServerProtocolVersion(ServerProtocolVersion {
        protocol_version: PROTOCOL_VERSION,
    });
    assert_matches!(response, StorageServiceResponse::CompressedResponse(_, _));
    assert_eq!(
        response.get_data_response().unwrap(),
        expected_data_response
    );
}

/// Sends a protocol version request and processes the response
async fn get_protocol_version(
    mock_client: &mut MockClient,
    use_compression: bool,
) -> StorageServiceResponse {
    let data_request = DataRequest::GetServerProtocolVersion;
    utils::send_storage_request(mock_client, use_compression, data_request)
        .await
        .unwrap()
}
