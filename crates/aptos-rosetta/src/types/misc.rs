// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

/// [API Spec](https://www.rosetta-api.org/docs/models/Error.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Error {
    /// Error code
    pub code: u64,
    /// Message that always matches the error code
    pub message: String,
    /// Possible generic information about an error code
    pub description: Option<String>,
    /// Whether a call can retry on the error
    pub retriable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<ErrorDetails>,
}

/// Error details that are specific to the instance
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ErrorDetails {
    /// Detailed error message
    pub error: String,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/OperationStatus.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OperationStatus {
    pub status: String,
    pub successful: bool,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/Peer.html)
///
/// TODO: Metadata?
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Peer {
    peer_id: String,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/SyncStatus.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SyncStatus {
    current_index: Option<u64>,
    target_index: Option<u64>,
    stage: Option<String>,
    synced: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Version {
    pub rosetta_version: String,
    pub node_version: String,
    pub middleware_version: String,
}
