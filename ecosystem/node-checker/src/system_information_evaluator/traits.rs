// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)]

use crate::{evaluator::EvaluationResult, metric_collector::SystemInformation};
use anyhow::Result;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum SystemInformationEvaluatorError {
    /// The key we're looking for is missing from the baseline. Args:
    ///   - The key name.
    ///   - Explanation.
    /// When the target node is missing a key, we return an Evaluation
    /// indicating that something is wrong with the target node, but if the
    /// baseline node is missing a key, it implies that something is wrong
    /// with our node checker configuration, so we return an error here.
    #[error("A key was unexpectedly missing from the baseline system information. Key name: {0}, Explanation: {1}")]
    BaselineMissingKey(String, String),
}

/// This trait defines evaluators that operate on the data from /system_information
/// on the metrics port.
pub trait SystemInformationEvaluator: Sync + Send {
    fn evaluate_system_information(
        &self,
        baseline_system_information: &SystemInformation,
        target_system_information: &SystemInformation,
    ) -> Result<Vec<EvaluationResult>, SystemInformationEvaluatorError>;

    fn get_name(&self) -> String;
}

impl std::fmt::Debug for dyn SystemInformationEvaluator {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            fmt,
            "SystemInformationEvaluator {{ name: {:?} }}",
            self.get_name()
        )
    }
}
