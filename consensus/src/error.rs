// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::pipeline;
use thiserror::Error;

#[derive(Debug, Error)]
#[error(transparent)]
pub struct DbError {
    #[from]
    inner: anyhow::Error,
}

impl From<velor_storage_interface::VelorDbError> for DbError {
    fn from(e: velor_storage_interface::VelorDbError) -> Self {
        DbError { inner: e.into() }
    }
}

#[derive(Debug, Error)]
#[error(transparent)]
pub struct StateSyncError {
    #[from]
    inner: anyhow::Error,
}

impl From<pipeline::errors::Error> for StateSyncError {
    fn from(e: pipeline::errors::Error) -> Self {
        StateSyncError { inner: e.into() }
    }
}

impl From<velor_executor_types::ExecutorError> for StateSyncError {
    fn from(e: velor_executor_types::ExecutorError) -> Self {
        StateSyncError { inner: e.into() }
    }
}

#[derive(Debug, Error)]
#[error(transparent)]
pub struct MempoolError {
    #[from]
    inner: anyhow::Error,
}

#[derive(Debug, Error)]
#[error(transparent)]
pub struct QuorumStoreError {
    #[from]
    inner: anyhow::Error,
}

#[derive(Debug, Error)]
#[error(transparent)]
pub struct VerifyError {
    #[from]
    inner: anyhow::Error,
}

pub fn error_kind(e: &anyhow::Error) -> &'static str {
    if e.downcast_ref::<velor_executor_types::ExecutorError>()
        .is_some()
    {
        return "Execution";
    }
    if let Some(e) = e.downcast_ref::<StateSyncError>() {
        if e.inner
            .downcast_ref::<velor_executor_types::ExecutorError>()
            .is_some()
        {
            return "Execution";
        }
        return "StateSync";
    }
    if e.downcast_ref::<MempoolError>().is_some() {
        return "Mempool";
    }
    if e.downcast_ref::<QuorumStoreError>().is_some() {
        return "QuorumStore";
    }
    if e.downcast_ref::<DbError>().is_some() {
        return "ConsensusDb";
    }
    if e.downcast_ref::<velor_safety_rules::Error>().is_some() {
        return "SafetyRules";
    }
    if e.downcast_ref::<VerifyError>().is_some() {
        return "VerifyError";
    }
    "InternalError"
}

#[cfg(test)]
mod tests {
    use crate::error::{error_kind, StateSyncError};
    use anyhow::Context;

    #[test]
    fn conversion_and_downcast() {
        let error = velor_executor_types::ExecutorError::InternalError {
            error: "lalala".to_string(),
        };
        let typed_error: StateSyncError = error.into();
        let upper: anyhow::Result<()> = Err(typed_error).context("Context!");
        assert_eq!(error_kind(&upper.unwrap_err()), "Execution");
    }
}
