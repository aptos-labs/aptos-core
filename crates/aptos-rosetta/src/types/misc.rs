// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

///
///
/// [API Spec](https://www.rosetta-api.org/docs/models/Error.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Error {
    /// Error code
    pub code: u64,
    /// Message that always matches the error code
    pub message: String,
    /// Possible generic information about an error code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether a call can retry on the error
    pub retriable: bool,
    /// Specific details of the error e.g. stack trace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<ErrorDetails>,
}

/// Error details that are specific to the instance
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ErrorDetails {
    /// Detailed error message
    pub error: String,
}

/// Status of an operation
///
/// [API Spec](https://www.rosetta-api.org/docs/models/OperationStatus.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OperationStatus {
    pub status: String,
    pub successful: bool,
}

/// Represents a Peer, used for discovery
///
/// [API Spec](https://www.rosetta-api.org/docs/models/Peer.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Peer {
    peer_id: String,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/SyncStatus.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SyncStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    current_index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stage: Option<String>,
    synced: bool,
}

/// Version information for the current deployment to handle software version matching
///
/// [API Spec](https://www.rosetta-api.org/docs/models/Version.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Version {
    /// Rosetta version, this should be hardcoded
    pub rosetta_version: String,
    /// Node version, this should come from the node
    pub node_version: String,
    /// Middleware version, this should be the version of this software
    pub middleware_version: String,
}
