// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
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

/// The suffix to append to data request and responses labels
/// (if the request/response requires compression).
const COMPRESSION_SUFFIX_LABEL: &str = "_compressed";

/// A type alias for different epochs.
pub type Epoch = u64;

/// Shorthand error typing
pub type Result<T, E = StorageServiceError> = ::std::result::Result<T, E>;

/// A storage service error that can be returned to the client on a failure
/// to process a service request.
#[derive(Clone, Debug, Deserialize, Error, PartialEq, Eq, Serialize)]
pub enum StorageServiceError {
    #[error("Internal service error: {0}")]
    InternalError(String),
    #[error("Invalid storage request: {0}")]
    InvalidRequest(String),
    #[error("Too many invalid requests! Back off required: {0}")]
    TooManyInvalidRequests(String),
}

/// A single storage service message sent or received over VelorNet.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[allow(clippy::large_enum_variant)]
pub enum StorageServiceMessage {
    /// A request to the storage service.
    Request(StorageServiceRequest),
    /// A response from the storage service. If there was an error while handling
    /// the request, the service will return an [`StorageServiceError`] error.
    Response(Result<StorageServiceResponse>),
}
