// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_transaction_emitter_lib::emitter::stats::TxnStats;
use log::info;
use serde::Serialize;
use std::fmt;

#[derive(Default, Debug, Serialize)]
pub struct TestReport {
    metrics: Vec<ReportedMetric>,
    text: String,
}

#[derive(Debug, Serialize)]
pub struct ReportedMetric {
    pub test_name: String,
    pub metric: String,
    pub value: f64,
}

impl TestReport {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn report_metric<E: ToString, M: ToString>(&mut self, test: E, metric: M, value: f64) {
        self.metrics.push(ReportedMetric {
            test_name: test.to_string(),
            metric: metric.to_string(),
            value,
        });
    }

    pub fn report_text(&mut self, text: String) {
        if !self.text.is_empty() {
            self.text.push('\n');
        }
        self.text.push_str(&text);
        info!("{}", text);
    }

    pub fn report_txn_stats(&mut self, test_name: String, stats: &TxnStats) {
        let rate = stats.rate();
        self.report_metric(test_name.clone(), "submitted_txn", stats.submitted as f64);
        self.report_metric(test_name.clone(), "expired_txn", stats.expired as f64);
        self.report_metric(test_name.clone(), "avg_tps", rate.committed);
        self.report_metric(test_name.clone(), "avg_latency", rate.latency);
        self.report_metric(test_name.clone(), "p50_latency", rate.p50_latency as f64);
        self.report_metric(test_name.clone(), "p90_latency", rate.p90_latency as f64);
        self.report_metric(test_name.clone(), "p99_latency", rate.p99_latency as f64);
        self.report_text(format!("{} : {}", test_name, rate));
    }

    pub fn print_report(&self) {
        println!("Test Statistics: ");
        println!("{}", self);
        let json_report =
            serde_json::to_string_pretty(&self).expect("Failed to serialize report to json");
        println!(
            "\n====json-report-begin===\n{}\n====json-report-end===",
            json_report
        );
    }
}

impl fmt::Display for TestReport {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}
