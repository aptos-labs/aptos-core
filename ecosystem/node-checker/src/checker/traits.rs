// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{tps::TpsCheckerError, CheckResult};
use crate::{
    common::get_type_name,
    provider::{ProviderCollection, ProviderError},
};
use std::fmt::Debug;
use thiserror::Error;

/// A Checker is a component of NHC that is responsible for checking a
/// particular aspect of the node under investigation, be that metrics,
/// system information, API checks, load tests, etc.
#[async_trait::async_trait]
pub trait Checker: Debug + Sync + Send {
    /// This function is expected to take in a ProviderCollection
    /// and return a vec of evaluation results. It should only return
    /// errors when there is something wrong with NHC itself or the
    /// baseline node. If something is unexpected with the target,
    /// we expect this function to return an EvaluationResult indicating
    /// as such.
    async fn check(
        &self,
        providers: &ProviderCollection,
    ) -> anyhow::Result<Vec<CheckResult>, CheckerError>;

    /// Helper to build a CheckResult where it sets the name of the checker
    /// that returned that result.
    fn build_result(headline: String, score: u8, explanation: String) -> CheckResult
    where
        Self: Sized,
    {
        CheckResult::new(
            get_type_name::<Self>().to_string(),
            headline,
            score,
            explanation,
        )
    }
}

/// This error is used by Checkers to indicate that something went wrong
/// with NHC or the baseline. If something went wrong with the target, we
/// expect the Checker to return a CheckResult indicating as such instead.
#[derive(Error, Debug)]
pub enum CheckerError {
    #[error("Provider failed to return data: {0:#}")]
    ProviderError(#[from] ProviderError),

    #[error("Something went wrong hitting endpoint {0}: {1:#}")]
    RetryableEndpointError(&'static str, #[source] anyhow::Error),

    #[error("Something went wrong hitting endpoint {0}: {1:#}")]
    NonRetryableEndpointError(&'static str, #[source] anyhow::Error),

    #[error("The necessary data ({0}) was mising: {1:#}")]
    MissingDataError(&'static str, #[source] anyhow::Error),

    // The TPS checker is pretty complex, we give it its own errors here.
    #[error("Something went wrong with the TPS checker: {0:#}")]
    TpsCheckerError(#[from] TpsCheckerError),
}

impl CheckerError {
    pub fn is_retryable(&self) -> bool {
        match self {
            CheckerError::ProviderError(error) => error.is_retryable(),
            CheckerError::RetryableEndpointError(_, _) => true,
            CheckerError::NonRetryableEndpointError(_, _) => false,
            CheckerError::MissingDataError(_, _) => false,
            CheckerError::TpsCheckerError(error) => error.is_retryable(),
        }
    }
}
