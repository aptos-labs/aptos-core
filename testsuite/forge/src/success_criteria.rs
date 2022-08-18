// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use serde::Serialize;
use std::time::{Duration, Instant};
use transaction_emitter_lib::emitter::stats::TxnStats;

use crate::{Swarm, SwarmExt};

#[derive(Default, Clone, Debug, Serialize)]
pub struct SuccessCriteria {
    avg_tps: usize,
    max_latency_ms: usize,
    wait_for_all_nodes_to_catchup: Option<Duration>,
}

impl SuccessCriteria {
    pub fn new(
        tps: usize,
        max_latency_ms: usize,
        wait_for_all_nodes_to_catchup: Option<Duration>,
    ) -> Self {
        Self {
            avg_tps: tps,
            max_latency_ms,
            wait_for_all_nodes_to_catchup,
        }
    }

    pub fn check_for_success(
        &self,
        stats: &TxnStats,
        window: &Duration,
        swarm: &dyn Swarm,
    ) -> anyhow::Result<()> {
        // TODO: Add more success criteria like expired transactions, CPU, memory usage etc
        let avg_tps = stats.committed / window.as_secs();
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

        if let Some(duration) = self.wait_for_all_nodes_to_catchup {
            futures::executor::block_on(async {
                swarm
                    .wait_for_all_nodes_to_catchup(Instant::now() + duration)
                    .await
            })?;
        }

        // TODO(skedia) Add latency success criteria after we have support for querying prometheus
        // latency
        Ok(())
    }
}
