// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_crypto::HashValue;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Error, PartialEq, Eq, Serialize)]
/// Different reasons for proposal rejection
pub enum Error {
    #[error("Provided epoch, {0}, does not match expected epoch, {1}")]
    IncorrectEpoch(u64, u64),
    #[error("block has next round that wraps around: {0}")]
    IncorrectRound(u64),
    #[error("Provided round, {0}, is incompatible with last voted round, {1}")]
    IncorrectLastVotedRound(u64, u64),
    #[error("Provided round, {0}, is incompatible with preferred round, {1}")]
    IncorrectPreferredRound(u64, u64),
    #[error("Unable to verify that the new tree extends the parent: {0}")]
    InvalidAccumulatorExtension(String),
    #[error("Invalid EpochChangeProof: {0}")]
    InvalidEpochChangeProof(String),
    #[error("Internal error: {0}")]
    InternalError(String),
    #[error("No next_epoch_state specified in the provided Ledger Info")]
    InvalidLedgerInfo,
    #[error("Invalid proposal: {0}")]
    InvalidProposal(String),
    #[error("Invalid QC: {0}")]
    InvalidQuorumCertificate(String),
    #[error("{0} is not set, SafetyRules is not initialized")]
    NotInitialized(String),
    #[error("Does not satisfy order vote rule. Block Round {0}, Highest Timeout Round {1}")]
    NotSafeForOrderVote(u64, u64),
    #[error("Data not found in secure storage: {0}")]
    SecureStorageMissingDataError(String),
    #[error("Unexpected error returned by secure storage: {0}")]
    SecureStorageUnexpectedError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Validator key not found: {0}")]
    ValidatorKeyNotFound(String),
    #[error("The validator is not in the validator set. Address not in set: {0}")]
    ValidatorNotInSet(String),
    #[error("Vote proposal missing expected signature")]
    VoteProposalSignatureNotFound,
    #[error("Does not satisfy 2-chain voting rule. Round {0}, Quorum round {1}, TC round {2},  HQC round in TC {3}")]
    NotSafeToVote(u64, u64, u64, u64),
    #[error("Does not satisfy 2-chain timeout rule. Round {0}, Quorum round {1}, TC round {2}, one-chain round {3}")]
    NotSafeToTimeout(u64, u64, u64, u64),
    #[error("Invalid TC: {0}")]
    InvalidTimeoutCertificate(String),
    #[error("Inconsistent Execution Result: Ordered BlockInfo doesn't match executed BlockInfo. Ordered: {0}, Executed: {1}")]
    InconsistentExecutionResult(String, String),
    #[error("Invalid Ordered LedgerInfoWithSignatures: Empty or at least one of executed_state_id, version, or epoch_state are not dummy value: {0}")]
    InvalidOrderedLedgerInfo(String),
    #[error("Waypoint out of date: Previous waypoint version {0}, updated version {1}, current epoch {2}, provided epoch {3}")]
    WaypointOutOfDate(u64, u64, u64, u64),
    #[error("Invalid Timeout: {0}")]
    InvalidTimeout(String),
    #[error("Incorrect 1-chain Quorum Certificate provided for signing order votes. Quorum Certificate: {0}, block id: {1}")]
    InvalidOneChainQuorumCertificate(HashValue, HashValue),
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::SerializationError(format!("{}", error))
    }
}

impl From<velor_secure_net::Error> for Error {
    #[allow(clippy::fallible_impl_from)]
    fn from(error: velor_secure_net::Error) -> Self {
        Self::InternalError(error.to_string())
    }
}

impl From<velor_secure_storage::Error> for Error {
    fn from(error: velor_secure_storage::Error) -> Self {
        match error {
            velor_secure_storage::Error::PermissionDenied => {
                // If a storage error is thrown that indicates a permission failure, we
                // want to panic immediately to alert an operator that something has gone
                // wrong. For example, this error is thrown when a storage (e.g., vault)
                // token has expired, so it makes sense to fail fast and require a token
                // renewal!
                panic!(
                    "A permission error was thrown: {:?}. Maybe the storage token needs to be renewed?",
                    error
                );
            },
            velor_secure_storage::Error::KeyVersionNotFound(_, _)
            | velor_secure_storage::Error::KeyNotSet(_) => {
                Self::SecureStorageMissingDataError(error.to_string())
            },
            _ => Self::SecureStorageUnexpectedError(error.to_string()),
        }
    }
}
