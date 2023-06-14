// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::NetworkLoadTest;
use aptos_forge::{
    args::TransactionTypeArg,
    success_criteria::{SuccessCriteria, SuccessCriteriaChecker},
    EmitJobMode, EmitJobRequest, NetworkContext, NetworkTest, Result, Test, TxnStats,
};
use aptos_logger::info;
use rand::SeedableRng;
use std::{
    fmt::{self, Debug, Display},
    time::Duration,
};
use tokio::runtime::Runtime;

pub struct SingleRunStats {
    name: String,
    stats: TxnStats,
    ledger_transactions: u64,
    actual_duration: Duration,
}

pub enum Workloads {
    TPS(&'static [usize]),
    TRANSACTIONS(&'static [TransactionWorkload]),
}

impl Workloads {
    fn len(&self) -> usize {
        match self {
            Self::TPS(tpss) => tpss.len(),
            Self::TRANSACTIONS(workloads) => workloads.len(),
        }
    }

    fn name(&self, index: usize) -> String {
        match self {
            Self::TPS(tpss) => tpss[index].to_string(),
            Self::TRANSACTIONS(workloads) => workloads[index].to_string(),
        }
    }

    fn configure(&self, index: usize, request: EmitJobRequest) -> EmitJobRequest {
        match self {
            Self::TPS(tpss) => request.mode(EmitJobMode::ConstTps { tps: tpss[index] }),
            Self::TRANSACTIONS(workloads) => workloads[index].configure(request),
        }
    }
}

#[derive(Debug)]
pub struct TransactionWorkload {
    pub transaction_type: TransactionTypeArg,
    pub num_modules: usize,
    pub unique_senders: bool,
}

impl TransactionWorkload {
    fn configure(&self, request: EmitJobRequest) -> EmitJobRequest {
        let account_creation_type =
            TransactionTypeArg::AccountGenerationLargePool.materialize(1, false);

        if self.unique_senders {
            request.transaction_type(self.transaction_type.materialize(self.num_modules, false))
        } else {
            let write_type = self.transaction_type.materialize(self.num_modules, true);
            request.transaction_mix_per_phase(vec![
                // warmup
                vec![(account_creation_type, 1)],
                vec![(account_creation_type, 1)],
                vec![(write_type, 1)],
                // cooldown
                vec![(write_type, 1)],
            ])
        }
    }
}

impl Display for TransactionWorkload {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

pub struct LoadVsPerfBenchmark {
    pub test: Box<dyn NetworkLoadTest>,
    pub workloads: Workloads,
    pub criteria: Vec<SuccessCriteria>,
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
        workloads: &Workloads,
        index: usize,
        duration: Duration,
    ) -> Result<Vec<SingleRunStats>> {
        let rng = SeedableRng::from_rng(ctx.core().rng())?;
        let emit_job_request = workloads.configure(index, ctx.emit_job.clone());
        let (stats, actual_duration, ledger_transactions, stats_by_phase) =
            self.test.network_load_test(
                ctx,
                emit_job_request,
                duration,
                // add larger warmup, as when we are exceeding the max load,
                // it takes more time to fill mempool.
                0.2,
                0.05,
                rng,
            )?;

        let mut result = vec![SingleRunStats {
            name: workloads.name(index),
            stats,
            ledger_transactions,
            actual_duration,
        }];

        if stats_by_phase.len() > 1 {
            for (i, (phase_stats, phase_duration)) in stats_by_phase.into_iter().enumerate() {
                result.push(SingleRunStats {
                    name: format!("{}_phase_{}", workloads.name(index), i),
                    stats: phase_stats,
                    ledger_transactions,
                    actual_duration: phase_duration,
                });
            }
        }

        Ok(result)
    }
}

impl NetworkTest for LoadVsPerfBenchmark {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        assert!(
            self.criteria.is_empty() || self.criteria.len() == self.workloads.len(),
            "Invalid config, {} criteria and {} workloads given",
            self.criteria.len(),
            self.workloads.len(),
        );

        let _runtime = Runtime::new().unwrap();
        let individual_with_buffer = ctx
            .global_duration
            .checked_div(self.workloads.len() as u32)
            .unwrap();
        let individual_duration = individual_with_buffer.mul_f32(0.8);
        let buffer = individual_with_buffer - individual_duration;

        let mut results = Vec::new();
        for index in 0..self.workloads.len() {
            if index != 0 {
                info!("Sleeping in between loadtests, for {}s", buffer.as_secs());
                std::thread::sleep(buffer);
            }

            info!("Starting for {}", self.workloads.name(index));
            results.append(&mut self.evaluate_single(
                ctx,
                &self.workloads,
                index,
                individual_duration,
            )?);

            // Note: uncomment below to perform reconfig during a test
            // let mut aptos_info = ctx.swarm().aptos_public_info();
            // runtime.block_on(aptos_info.reconfig());

            let table = to_table(&results);
            for line in table {
                info!("{}", line);
            }
        }

        let table = to_table(&results);
        for line in table {
            ctx.report.report_text(line);
        }
        for (index, result) in results.iter().enumerate() {
            let rate = result.stats.rate();
            if let Some(criteria) = self.criteria.get(index) {
                SuccessCriteriaChecker::check_core_for_success(
                    criteria,
                    ctx.report,
                    &rate,
                    Some(result.name.clone()),
                )?;
            }
        }
        Ok(())
    }
}

fn to_table(results: &[SingleRunStats]) -> Vec<String> {
    let mut table = Vec::new();
    table.push(format!(
        "{: <30} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12}",
        "workload",
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
    ));

    for result in results {
        let rate = result.stats.rate();
        table.push(format!(
            "{: <30} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12}",
            result.name,
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
        ));
    }

    table
}
