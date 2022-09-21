// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::error::ApiError;
use serde::{Deserialize, Serialize};
use std::{
    convert::TryFrom,
    fmt::{Display, Formatter},
    str::FromStr,
};

/// Errors that can be returned by the API
///
/// [API Spec](https://www.rosetta-api.org/docs/models/Error.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Error {
    /// Error code
    pub code: u32,
    /// Message that always matches the error code
    pub message: String,
    /// Whether a call can retry on the error
    pub retriable: bool,
    /// Specific details of the error e.g. stack trace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<ErrorDetails>,
}

/// Error details that are specific to the instance
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ErrorDetails {
    /// Related error details
    pub details: String,
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

/// An internal enum to support Operation typing
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum OperationType {
    // Create must always be first for ordering
    CreateAccount,
    // Withdraw must come before deposit
    Withdraw,
    Deposit,
    SetOperator,
    SetVoter,
    // Fee must always be last for ordering
    Fee,
}

impl OperationType {
    const CREATE_ACCOUNT: &'static str = "create_account";
    const DEPOSIT: &'static str = "deposit";
    const WITHDRAW: &'static str = "withdraw";
    const FEE: &'static str = "fee";
    const SET_OPERATOR: &'static str = "set_operator";
    const SET_VOTER: &'static str = "set_voter";

    pub fn all() -> Vec<OperationType> {
        vec![
            OperationType::CreateAccount,
            OperationType::Withdraw,
            OperationType::Deposit,
            OperationType::Fee,
            OperationType::SetOperator,
            OperationType::SetVoter,
        ]
    }
}

impl FromStr for OperationType {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            Self::CREATE_ACCOUNT => Ok(OperationType::CreateAccount),
            Self::DEPOSIT => Ok(OperationType::Deposit),
            Self::WITHDRAW => Ok(OperationType::Withdraw),
            Self::FEE => Ok(OperationType::Fee),
            Self::SET_OPERATOR => Ok(OperationType::SetOperator),
            Self::SET_VOTER => Ok(OperationType::SetVoter),
            _ => Err(ApiError::DeserializationFailed(Some(format!(
                "Invalid OperationType: {}",
                s
            )))),
        }
    }
}

impl Display for OperationType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            OperationType::CreateAccount => Self::CREATE_ACCOUNT,
            OperationType::Deposit => Self::DEPOSIT,
            OperationType::Withdraw => Self::WITHDRAW,
            OperationType::SetOperator => Self::SET_OPERATOR,
            OperationType::SetVoter => Self::SET_VOTER,
            OperationType::Fee => Self::FEE,
        })
    }
}

/// An internal type to support typing of Operation statuses
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum OperationStatusType {
    /// Operation was part of a successfully committed transaction
    Success,
    /// Operation was not part of a successfully committed transaction
    Failure,
}

impl OperationStatusType {
    const SUCCESS: &'static str = "success";
    const FAILURE: &'static str = "failure";

    pub fn all() -> Vec<OperationStatusType> {
        vec![OperationStatusType::Success, OperationStatusType::Failure]
    }
}

impl From<OperationStatusType> for OperationStatus {
    fn from(status: OperationStatusType) -> Self {
        let successful = match status {
            OperationStatusType::Success => true,
            OperationStatusType::Failure => false,
        };

        OperationStatus {
            status: status.to_string(),
            successful,
        }
    }
}

impl TryFrom<OperationStatus> for OperationStatusType {
    type Error = ApiError;

    fn try_from(status: OperationStatus) -> Result<Self, Self::Error> {
        OperationStatusType::from_str(&status.status)
    }
}

impl FromStr for OperationStatusType {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            Self::SUCCESS => Ok(OperationStatusType::Success),
            Self::FAILURE => Ok(OperationStatusType::Failure),
            _ => Err(ApiError::DeserializationFailed(Some(format!(
                "Invalid OperationStatusType: {}",
                s
            )))),
        }
    }
}

impl Display for OperationStatusType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            OperationStatusType::Success => Self::SUCCESS,
            OperationStatusType::Failure => Self::FAILURE,
        })
    }
}
