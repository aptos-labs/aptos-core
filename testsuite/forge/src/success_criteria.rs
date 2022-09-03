// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use serde::Serialize;
use std::time::Duration;
use transaction_emitter_lib::emitter::stats::TxnStats;

use crate::{Swarm, SwarmExt};

#[derive(Default, Clone, Debug, Serialize)]
pub struct SuccessCriteria {
    pub avg_tps: usize,
    pub max_latency_ms: usize,
    check_no_restarts: bool,
    wait_for_all_nodes_to_catchup: Option<Duration>,
}

impl SuccessCriteria {
    pub fn new(
        tps: usize,
        max_latency_ms: usize,
        check_no_restarts: bool,
        wait_for_all_nodes_to_catchup: Option<Duration>,
    ) -> Self {
        Self {
            avg_tps: tps,
            max_latency_ms,
            check_no_restarts,
            wait_for_all_nodes_to_catchup,
        }
    }

    pub async fn check_for_success(
        &self,
        stats: &TxnStats,
        window: &Duration,
        swarm: &dyn Swarm,
    ) -> anyhow::Result<()> {
        // TODO: Add more success criteria like expired transactions, CPU, memory usage etc
        let avg_tps = stats.committed / window.as_secs();
        if avg_tps < self.avg_tps as u64 {
            let error_message = format!(
                "TPS requirement failed. Average TPS {}, minimum TPS requirement {}",
                avg_tps, self.avg_tps
            );
            bail!(error_message)
        }

        if let Some(timeout) = self.wait_for_all_nodes_to_catchup {
            swarm.wait_for_all_nodes_to_catchup(timeout).await?;
        }

        if self.check_no_restarts {
            swarm.ensure_no_validator_restart().await?;
            swarm.ensure_no_fullnode_restart().await?;
        }

        // TODO(skedia) Add latency success criteria after we have support for querying prometheus
        // latency
        Ok(())
    }
}
