// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Debug;
use thiserror::Error;

/// This traits defines something that is expected to output some kind of data.
/// Naturally that is pretty vague, and that's by design, this trait isn't meant
/// to be too constraining. Having the trait is still helpful though for
/// generalizing things such as memoization and retries.
#[async_trait::async_trait]
pub trait Provider: Debug + Sync + Send {
    type Output: Debug;

    /// This function is expected to return `Output`.
    async fn provide(&self) -> Result<Self::Output, ProviderError>;

    // This function is expected to return some explanation for why the Provider
    // couldn't be built. This is only for use in error messages.
    fn explanation() -> &'static str
    where
        Self: Sized;
}

/// For the sake of simplicity, this error type captures all different types
/// of errors that a Provider can return, in general terms. Really the only
/// thing that matters is whether the error is retryable or not.
#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("Something went wrong hitting endpoint {0}: {1:#}")]
    RetryableEndpointError(&'static str, #[source] anyhow::Error),

    #[error("Something went wrong hitting endpoint {0}: {1:#}")]
    NonRetryableEndpointError(&'static str, #[source] anyhow::Error),

    #[error("Something went wrong parsing the response from the node: {0:#}")]
    ParseError(#[from] anyhow::Error),
}

impl ProviderError {
    pub fn is_retryable(&self) -> bool {
        match self {
            ProviderError::RetryableEndpointError(_, _) => true,
            ProviderError::NonRetryableEndpointError(_, _) => false,
            ProviderError::ParseError(_) => false,
        }
    }
}
