// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Error types for IDL operations

use thiserror::Error;

/// Error types for ValidatorInfo IDL operations
#[derive(Debug, Error)]
pub enum ValidatorInfoIdlError {
    #[error("JSON deserialization error: {0}")]
    JsonDeserializationError(String),
    #[error("Account address parsing error: {0}")]
    AccountAddressError(String),
    #[error("Consensus public key parsing error: {0}")]
    ConsensusPublicKeyError(String),
    #[error("Network addresses BCS deserialization error: {0}")]
    NetworkAddressesError(String),
    #[error("Base64 decoding error: {0}")]
    Base64Error(String),
    #[error("Hex decoding error: {0}")]
    HexError(String),
}

/// Error types for general IDL operations
#[derive(Debug, Error)]
pub enum IdlError {
    #[error("ValidatorInfo IDL error: {0}")]
    ValidatorInfo(#[from] ValidatorInfoIdlError),
    #[error("JSON serialization error: {0}")]
    JsonSerializationError(String),
    #[error("JSON deserialization error: {0}")]
    JsonDeserializationError(String),
    #[error("Unsupported type: {0}")]
    UnsupportedType(String),
} 