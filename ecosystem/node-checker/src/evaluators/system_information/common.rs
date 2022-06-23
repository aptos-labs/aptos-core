// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{evaluator::EvaluationResult, metric_collector::SystemInformation};

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
