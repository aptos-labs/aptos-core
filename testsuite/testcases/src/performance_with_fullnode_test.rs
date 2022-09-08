// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{LoadDestination, NetworkLoadTest};
use forge::system_metrics::{MetricsThreshold, SystemMetricsThreshold};
use forge::{NetworkContext, NetworkTest, Result, Swarm, Test};
use tokio::runtime::Runtime;

pub struct PerformanceBenchmarkWithFN;

impl Test for PerformanceBenchmarkWithFN {
    fn name(&self) -> &'static str {
        "performance benchmark with full nodes"
    }
}

impl NetworkLoadTest for PerformanceBenchmarkWithFN {
    fn setup(&self, _swarm: &mut dyn Swarm) -> Result<LoadDestination> {
        Ok(LoadDestination::AllFullnodes)
    }

    fn finish(&self, swarm: &mut dyn Swarm, start_time: u64, end_time: u64) -> Result<()> {
        let runtime = Runtime::new().unwrap();
        // Threshold of more than 12 CPU cores for 30% of the time
        let cpu_threshold = MetricsThreshold::new(12, 30);
        // Threshold of more than 3 GB of memory for 30% of the time
        let memory_threshold = MetricsThreshold::new(3 * 1024 * 1024 * 1024, 30);
        runtime.block_on(swarm.ensure_healthy_system_metrics(
            start_time as i64,
            end_time as i64,
            SystemMetricsThreshold::new(cpu_threshold, memory_threshold),
        ))?;
        Ok(())
    }
}

impl NetworkTest for PerformanceBenchmarkWithFN {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}
