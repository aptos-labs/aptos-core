// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

// Maximum number of threads for the file store
pub const MAXIMUM_NUMBER_FILESTORE_THREADS: usize = 10;
// GRPC request metadata key for the token ID.
pub const GRPC_AUTH_TOKEN_HEADER: &str = "x-velor-data-authorization";
// GRPC request metadata key for the request name. This is used to identify the
// data destination.
pub const GRPC_REQUEST_NAME_HEADER: &str = "x-velor-request-name";
pub const GRPC_API_GATEWAY_API_KEY_HEADER: &str = "authorization";
// Limit the message size to 15MB. By default the downstream can receive up to 15MB.
pub const MESSAGE_SIZE_LIMIT: usize = 1024 * 1024 * 15;

// These come from API Gateway, see here:
// https://github.com/velor-chain/api-gateway/blob/0aae1c17fbd0f5e9b50bdb416f62b48d3d1d5e6b/src/common.rs

/// The type of the auth identity. For example, "anonymous IP" or "application" (API
/// key). For now all data service connections must be from an application, but we
/// include this for future-proofing.
pub const REQUEST_HEADER_VELOR_IDENTIFIER_TYPE: &str = "x-velor-identifier-type";
/// The identifier uniquely identifies the requester. For an application, this is the
/// application ID, a UUID4.
pub const REQUEST_HEADER_VELOR_IDENTIFIER: &str = "x-velor-identifier";
/// The email of the requester. For an application, this is the email of the user who
/// created the application. When looking at metrics based on this label, you should
/// also parallelize based on the application name. Or just use the identifier.
pub const REQUEST_HEADER_VELOR_EMAIL: &str = "x-velor-email";
/// The name of the application, e.g. something like "Graffio Testnet".
pub const REQUEST_HEADER_VELOR_APPLICATION_NAME: &str = "x-velor-application-name";

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct IndexerGrpcRequestMetadata {
    pub processor_name: String,
    /// See `REQUEST_HEADER_VELOR_IDENTIFIER_TYPE` for more information.
    pub request_identifier_type: String,
    /// See `REQUEST_HEADER_VELOR_IDENTIFIER` for more information.
    pub request_identifier: String,
    /// See `REQUEST_HEADER_VELOR_EMAIL` for more information.
    pub request_email: String,
    /// See `REQUEST_HEADER_VELOR_APPLICATION_NAME` for more information.
    pub request_application_name: String,
    pub request_connection_id: String,
    // Token is no longer needed behind api gateway.
    #[deprecated]
    pub request_token: String,
}

impl IndexerGrpcRequestMetadata {
    /// Get the label values for use with metrics that use these labels. Note, the
    /// order must match the order in metrics.rs.
    pub fn get_label_values(&self) -> Vec<&str> {
        vec![
            &self.request_identifier_type,
            &self.request_identifier,
            &self.request_email,
            &self.request_application_name,
            &self.processor_name,
        ]
    }
}
