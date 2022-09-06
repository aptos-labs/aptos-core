// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{LoadDestination, NetworkLoadTest};
use forge::{NetworkContext, NetworkTest, Result, Swarm, Test};

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

    // fn finish(&self, swarm: &mut dyn Swarm) -> Result<()> {
    //     let runtime = Runtime::new().unwrap();
    //     // Threshold of more than 12 CPU cores for 30% of the time
    //     let cpu_threshold = MetricsThreshold::new(12, 30);
    //     // Threshold of more than 3 GB of memory for 30% of the time
    //     let memory_threshold = MetricsThreshold::new(3 * 1024 * 1024 * 1024, 30);
    //     runtime.block_on(ctx.swarm().ensure_healthy_system_metrics(
    //         start_timestamp as i64,
    //         end_timestamp as i64,
    //         SystemMetricsThreshold::new(cpu_threshold, memory_threshold),
    //     ))?;
    //     Ok(())
    // }
}

impl NetworkTest for PerformanceBenchmarkWithFN {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}
