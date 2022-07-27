// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use serde::Serialize;
use std::time::Duration;
use transaction_emitter_lib::emitter::stats::TxnStats;

#[derive(Default, Clone, Debug, Serialize)]
pub struct SuccessCriteria {
    avg_tps: usize,
    max_latency_ms: usize,
}

impl SuccessCriteria {
    pub fn new(tps: usize, max_latency_ms: usize) -> Self {
        Self {
            avg_tps: tps,
            max_latency_ms,
        }
    }

    pub fn check_for_success(&self, stats: &TxnStats, window: &Duration) -> anyhow::Result<()> {
        // TODO: Add more success criteria like expired transactions, CPU, memory usage etc
        let avg_tps = stats.committed / window.as_secs();
        let p99_latency = stats.latency_buckets.percentile(99, 100);
        let is_triggerd_by_github_actions =
            std::env::var("FORGE_TRIGGERED_BY").unwrap_or_default() == "github-actions";
        if avg_tps < self.avg_tps as u64 {
            let error_message = format!(
                "TPS requirement failed. Average TPS {}, minimum TPS requirement {}",
                avg_tps, self.avg_tps
            );
            if is_triggerd_by_github_actions {
                // ::error:: is github specific syntax to set an error on the job that is highlighted as described here https://docs.github.com/en/actions/using-workflows/workflow-commands-for-github-actions#setting-an-error-message
                println!("::error::{error_message}");
            }
            bail!(error_message)
        }
        if p99_latency > self.max_latency_ms as u64 {
            let error_message = format!(
                "Latency requirement failed. P99 latency {}, maximum latency requirement {}",
                p99_latency, self.max_latency_ms
            );
            if is_triggerd_by_github_actions {
                println!("::error::{error_message}");
            }
            bail!(error_message)
        }
        Ok(())
    }
}
