// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod config;
mod grpc_response_stream;
mod metrics;
mod response_dispatcher;
mod service;

pub use config::{IndexerGrpcDataServiceConfig, NonTlsConfig, SERVER_NAME};
use serde::{Deserialize, Serialize};

pub const SERVICE_TYPE: &str = "data_service";

const REQUEST_HEADER_APTOS_EMAIL_HEADER: &str = "x-aptos-email";
const REQUEST_HEADER_APTOS_USER_CLASSIFICATION_HEADER: &str = "x-aptos-user-classification";
const REQUEST_HEADER_APTOS_API_KEY_NAME: &str = "x-aptos-api-key-name";
const REQUEST_HEADER_APTOS_REQUEST_NAME: &str = "x-aptos-request-name";

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RequestMetadata {
    pub request_api_key_name: String,
    pub request_email: String,
    pub user_classification: String,
    pub processor_name: String,
    pub connection_id: String,
}

impl RequestMetadata {
    pub fn new(
        request: &tonic::Request<aptos_protos::indexer::v1::GetTransactionsRequest>,
    ) -> Self {
        let request_metadata_pairs = vec![
            ("request_api_key_name", REQUEST_HEADER_APTOS_API_KEY_NAME),
            ("request_email", REQUEST_HEADER_APTOS_EMAIL_HEADER),
            (
                "user_classification",
                REQUEST_HEADER_APTOS_USER_CLASSIFICATION_HEADER,
            ),
            ("processor_name", REQUEST_HEADER_APTOS_REQUEST_NAME),
        ];
        let mut request_metadata_map: std::collections::HashMap<String, String> =
            request_metadata_pairs
                .into_iter()
                .map(|(key, value)| {
                    (
                        key.to_string(),
                        request
                            .metadata()
                            .get(value)
                            .map(|value| value.to_str().unwrap_or("unspecified").to_string())
                            .unwrap_or("unspecified".to_string()),
                    )
                })
                .collect();
        request_metadata_map.insert(
            "connection_id".to_string(),
            uuid::Uuid::new_v4().to_string(),
        );
        let request_metadata: RequestMetadata =
            serde_json::from_str(&serde_json::to_string(&request_metadata_map).unwrap()).unwrap();
        request_metadata
    }
}
