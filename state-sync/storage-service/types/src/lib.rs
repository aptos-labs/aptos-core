// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use requests::StorageServiceRequest;
use responses::StorageServiceResponse;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod requests;
pub mod responses;

#[cfg(test)]
mod tests;

/// A type alias for different epochs.
pub type Epoch = u64;

pub type Result<T, E = StorageServiceError> = ::std::result::Result<T, E>;

/// A storage service error that can be returned to the client on a failure
/// to process a service request.
#[derive(Clone, Debug, Deserialize, Eq, Error, PartialEq, Serialize)]
pub enum StorageServiceError {
    #[error("Internal service error: {0}")]
    InternalError(String),
    #[error("Invalid storage request: {0}")]
    InvalidRequest(String),
}

/// A single storage service message sent or received over AptosNet.
#[derive(Clone, Debug, Deserialize, Serialize)]
// TODO(philiphayes): do something about this without making it ugly :(
#[allow(clippy::large_enum_variant)]
pub enum StorageServiceMessage {
    /// A request to the storage service.
    Request(StorageServiceRequest),
    /// A response from the storage service. If there was an error while handling
    /// the request, the service will return an [`StorageServiceError`] error.
    Response(Result<StorageServiceResponse>),
}
