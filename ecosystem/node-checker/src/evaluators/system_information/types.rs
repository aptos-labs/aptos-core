// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)]

use crate::metric_collector::SystemInformation;
use thiserror::Error as ThisError;

#[derive(Debug)]
pub struct SystemInformationEvaluatorInput {
    pub baseline_system_information: SystemInformation,
    pub target_system_information: SystemInformation,
}

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
