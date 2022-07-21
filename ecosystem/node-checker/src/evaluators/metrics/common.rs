// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::evaluator::EvaluationResult;
use anyhow::{anyhow, Result};
use log::warn;
use prometheus_parse::{Scrape as PrometheusScrape, Value as PrometheusValue};

pub struct Label<'a> {
    pub key: &'a str,
    pub value: &'a str,
}

/// This function searches through the given set of metrics and searches for
/// a metric with the given metric name. If no label was given, we return that
/// metric immediately. If a label was given, we search for a metric that
/// has that label.
fn get_metric_value(
    metrics: &PrometheusScrape,
    metric_name: &str,
    expected_label: Option<&Label>,
) -> Option<u64> {
    let mut discovered_sample = None;
    for sample in &metrics.samples {
        if sample.metric == metric_name {
            match &expected_label {
                Some(expected_label) => {
                    let label_value = sample.labels.get(expected_label.key);
                    if let Some(label_value) = label_value {
                        if label_value == expected_label.value {
                            discovered_sample = Some(sample);
                            break;
                        }
                    }
                }
                None => {
                    discovered_sample = Some(sample);
                    break;
                }
            }
        }
    }
    match discovered_sample {
        Some(sample) => match &sample.value {
            PrometheusValue::Counter(v) => Some(v.round() as u64),
            PrometheusValue::Gauge(v) => Some(v.round() as u64),
            PrometheusValue::Untyped(v) => Some(v.round() as u64),
            wildcard => {
                warn!("Found unexpected metric type: {:?}", wildcard);
                None
            }
        },
        None => None,
    }
}

/// This is a convenience function that returns the metric value if it was
/// found, or an Evaluation if not.
pub fn get_metric<F>(
    metrics: &PrometheusScrape,
    metric_name: &str,
    expected_label: Option<&Label>,
    evaluation_on_missing_fn: F,
) -> GetMetricResult
where
    F: FnOnce() -> EvaluationResult,
{
    let metric_value = get_metric_value(metrics, metric_name, expected_label);
    match metric_value {
        Some(v) => GetMetricResult::Present(v),
        None => GetMetricResult::Missing(evaluation_on_missing_fn()),
    }
}

#[derive(Debug)]
pub enum GetMetricResult {
    Present(u64),
    Missing(EvaluationResult),
}

impl GetMetricResult {
    pub fn unwrap(self, evaluation_results: &mut Vec<EvaluationResult>) -> Option<u64> {
        match self {
            GetMetricResult::Present(value) => Some(value),
            GetMetricResult::Missing(evaluation_result) => {
                evaluation_results.push(evaluation_result);
                None
            }
        }
    }
}

pub fn parse_metrics(metrics: Vec<String>) -> Result<PrometheusScrape> {
    PrometheusScrape::parse(metrics.iter().map(|l| Ok(l.to_string()))).map_err(|e| anyhow!(e))
}
