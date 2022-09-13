// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::NetworkLoadTest;
use aptos_logger::info;
use forge::{EmitJobMode, NetworkContext, NetworkTest, Result, Test, TxnStats};
use rand::SeedableRng;
use std::time::Duration;
use tokio::runtime::Runtime;

pub struct SingleRunStats {
    tps: usize,
    stats: TxnStats,
    ledger_transactions: u64,
    actual_duration: Duration,
}

pub struct LoadVsPerfBenchmark {
    pub test: &'static dyn NetworkLoadTest,
    pub tps: &'static [usize],
}

impl Test for LoadVsPerfBenchmark {
    fn name(&self) -> &'static str {
        "continuous progress test"
    }
}

impl LoadVsPerfBenchmark {
    fn evaluate_single(
        &self,
        ctx: &mut NetworkContext<'_>,
        tps: usize,
        duration: Duration,
    ) -> Result<SingleRunStats> {
        let rng = SeedableRng::from_rng(ctx.core().rng())?;
        let emit_job_request = ctx.emit_job.clone().mode(EmitJobMode::ConstTps { tps });
        let (stats, actual_duration, ledger_transactions) = self.test.network_load_test(
            ctx,
            emit_job_request,
            duration,
            // add larger warmup, as we are exceeding the max load,
            // and for that it takes more time to fill mempool.
            0.2,
            0.05,
            rng,
        )?;

        Ok(SingleRunStats {
            tps,
            stats,
            ledger_transactions,
            actual_duration,
        })
    }
}

impl NetworkTest for LoadVsPerfBenchmark {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        let runtime = Runtime::new().unwrap();
        let individual_with_buffer = ctx
            .global_duration
            .checked_div(self.tps.len() as u32)
            .unwrap();
        let individual_duration = individual_with_buffer.mul_f32(0.8);
        let buffer = individual_with_buffer - individual_duration;

        let mut results = Vec::new();
        for (i, tps) in self.tps.iter().enumerate() {
            if i != 0 {
                info!("Sleeping in between loadtests, for {}s", buffer.as_secs());
                std::thread::sleep(buffer);
            }

            info!("Starting for {}", tps);
            let result = self.evaluate_single(ctx, *tps, individual_duration)?;
            results.push(result);

            let mut aptos_info = ctx.swarm().aptos_public_info();
            runtime.block_on(aptos_info.reconfig());

            println!(
                "{: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12}",
                "wanted/s",
                "submitted/s",
                "committed/s",
                "expired/s",
                "rejected/s",
                "chain txn/s",
                "latency",
                "p50 lat",
                "p90 lat",
                "p99 lat",
                "actual dur"
            );
            for result in &results {
                let rate = result.stats.rate(result.actual_duration);
                println!(
                    "{: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12}",
                    result.tps,
                    rate.submitted,
                    rate.committed,
                    rate.expired,
                    rate.failed_submission,
                    result.ledger_transactions / result.actual_duration.as_secs(),
                    rate.latency,
                    rate.p50_latency,
                    rate.p90_latency,
                    rate.p99_latency,
                    result.actual_duration.as_secs()
                )
            }
        }
        Ok(())
    }
}
