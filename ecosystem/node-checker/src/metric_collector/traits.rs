// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Error, Result};
use async_trait::async_trait;
use thiserror::Error as ThisError;

// TODO: Consider using thiserror.

#[derive(Debug, ThisError)]
pub enum MetricCollectorError {
    /// We were unable to get data from the node.
    #[error("Failed to pull data from the node")]
    GetDataError(Error),

    /// We could not perform basic parsing on the response.
    #[error("Failed to parse the response from the node")]
    ResponseParseError(Error),
}

/// todo describe the trait
/// todo assert these trait constraints are necessary
/// todo consider whether we need Clone if we need to spawn multiple handlers ourselves.
///
/// Note:
///  - Sync + Send is required because this will be a member of the todo which needs
///      to be used across async boundaries
///
///  - 'static is required because this will be stored on the todo which needs to be 'static
#[async_trait]
pub trait MetricCollector: Sync + Send + 'static {
    async fn collect_metrics(&self) -> Result<Vec<String>, MetricCollectorError>;
}
