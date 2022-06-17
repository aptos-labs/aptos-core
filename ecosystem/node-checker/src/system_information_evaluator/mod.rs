// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod build_evaluators;
mod build_version_evaluator;
mod traits;

use crate::{evaluator::EvaluationResult, metric_collector::SystemInformation};

pub use build_evaluators::build_evaluators;
pub use build_version_evaluator::{BuildVersionEvaluator, BuildVersionEvaluatorArgs};
pub use traits::{SystemInformationEvaluator, SystemInformationEvaluatorError};

pub const EVALUATOR_SOURCE: &str = "system_information";

/// This is a convenience function that returns the value if it was
/// found, or an Evaluation if not.
pub fn get_value<F>(
    metrics: &SystemInformation,
    metric_key: &str,
    evaluation_on_missing_fn: F,
) -> GetValueResult
where
    F: FnOnce() -> EvaluationResult,
{
    let metric_value = metrics.0.get(metric_key);
    match metric_value {
        Some(v) => GetValueResult::Present(v.to_string()),
        None => GetValueResult::Missing(evaluation_on_missing_fn()),
    }
}

pub enum GetValueResult {
    Present(String),
    Missing(EvaluationResult),
}
