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
        if avg_tps < self.avg_tps as u64 {
            bail!(
                "TPS requirement failed. Average TPS {}, minimum TPS requirement {}",
                avg_tps,
                self.avg_tps
            )
        }
        if p99_latency > self.max_latency_ms as u64 {
            bail!(
                "Latency requirement failed. P99 latency {}, maximum latency requirement {}",
                p99_latency,
                self.max_latency_ms
            )
        }
        Ok(())
    }
}
