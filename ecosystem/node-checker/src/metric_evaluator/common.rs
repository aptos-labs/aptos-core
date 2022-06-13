// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use log::warn;
use prometheus_parse::{Scrape as PrometheusScrape, Value as PrometheusValue};

pub fn get_metric_value(
    metrics: &PrometheusScrape,
    metric_name: &str,
    expected_label_key: &str,
    expected_label_value: &str,
) -> Option<u64> {
    for sample in &metrics.samples {
        if sample.metric == metric_name {
            let label_value = sample.labels.get(expected_label_key);
            if let Some(label_value) = label_value {
                if label_value == expected_label_value {
                    match &sample.value {
                        PrometheusValue::Counter(v) => return Some(v.round() as u64),
                        PrometheusValue::Gauge(v) => return Some(v.round() as u64),
                        PrometheusValue::Untyped(v) => return Some(v.round() as u64),
                        wildcard => {
                            warn!("Found unexpected metric type: {:?}", wildcard);
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn parse_metrics(metrics: Vec<String>) -> Result<PrometheusScrape> {
    PrometheusScrape::parse(metrics.iter().map(|l| Ok(l.to_string()))).map_err(|e| anyhow!(e))
}
