// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::NetworkLoadTest;
use aptos_forge::{
    args::TransactionTypeArg,
    prometheus_metrics::{LatencyBreakdown, LatencyBreakdownSlice},
    success_criteria::{SuccessCriteria, SuccessCriteriaChecker},
    EmitJobMode, EmitJobRequest, NetworkContext, NetworkTest, Result, Test, TxnStats,
};
use aptos_logger::info;
use rand::SeedableRng;
use std::{fmt::Debug, time::Duration};
use tokio::runtime::Runtime;

pub struct SingleRunStats {
    name: String,
    stats: TxnStats,
    latency_breakdown: LatencyBreakdown,
    ledger_transactions: u64,
    actual_duration: Duration,
}

#[derive(Debug)]
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

    fn type_name(&self) -> String {
        match self {
            Self::TPS(_) => "Load (TPS)".to_string(),
            Self::TRANSACTIONS(_) => "Workload".to_string(),
        }
    }

    fn phase_name(&self, index: usize, phase: usize) -> String {
        match self {
            Self::TPS(tpss) => {
                assert_eq!(phase, 0);
                format!("{}", tpss[index])
            },
            Self::TRANSACTIONS(workloads) => format!(
                "{}{}: {}",
                index,
                if workloads[index].is_phased() {
                    format!(": ph{}", phase)
                } else {
                    "".to_string()
                },
                workloads[index].phase_name(phase)
            ),
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
    pub mempool_backlog: usize,
}

impl TransactionWorkload {
    fn is_phased(&self) -> bool {
        self.unique_senders
    }

    fn configure(&self, request: EmitJobRequest) -> EmitJobRequest {
        let account_creation_type =
            TransactionTypeArg::AccountGenerationLargePool.materialize(1, false);

        let request = request.mode(EmitJobMode::MaxLoad {
            mempool_backlog: self.mempool_backlog,
        });

        if self.is_phased() {
            let write_type = self.transaction_type.materialize(self.num_modules, true);
            request.transaction_mix_per_phase(vec![
                // warmup
                vec![(account_creation_type, 1)],
                vec![(account_creation_type, 1)],
                vec![(write_type, 1)],
                // cooldown
                vec![(write_type, 1)],
            ])
        } else {
            request.transaction_type(self.transaction_type.materialize(self.num_modules, false))
        }
    }

    fn phase_name(&self, phase: usize) -> String {
        format!(
            "{}{}[{:.1}k]",
            match (self.is_phased(), phase) {
                (true, 0) => "CreateBurnerAccounts".to_string(),
                (true, 1) => format!("{:?}", self.transaction_type),
                (false, 0) => format!("{:?}", self.transaction_type),
                _ => unreachable!(),
            },
            if self.num_modules > 1 {
                format!("({} modules)", self.num_modules)
            } else {
                "".to_string()
            },
            self.mempool_backlog as f32 / 1000.0,
        )
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
        let stats_by_phase = self.test.network_load_test(
            ctx,
            emit_job_request,
            duration,
            // add larger warmup, as when we are exceeding the max load,
            // it takes more time to fill mempool.
            0.2,
            0.05,
            rng,
        )?;

        let mut result = vec![];
        for (phase, phase_stats) in stats_by_phase.into_iter().enumerate() {
            result.push(SingleRunStats {
                name: workloads.phase_name(index, phase),
                stats: phase_stats.emitter_stats,
                latency_breakdown: phase_stats.latency_breakdown,
                ledger_transactions: phase_stats.ledger_transactions,
                actual_duration: phase_stats.actual_duration,
            });
        }

        Ok(result)
    }
}

impl NetworkTest for LoadVsPerfBenchmark {
    fn run(&self, ctx: &mut NetworkContext<'_>) -> Result<()> {
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

            info!("Starting for {:?}", self.workloads);
            results.push(self.evaluate_single(ctx, &self.workloads, index, individual_duration)?);

            // Note: uncomment below to perform reconfig during a test
            // let mut aptos_info = ctx.swarm().aptos_public_info();
            // runtime.block_on(aptos_info.reconfig());

            let table = to_table(self.workloads.type_name(), &results);
            for line in table {
                info!("{}", line);
            }
        }

        let table = to_table(self.workloads.type_name(), &results);
        for line in table {
            ctx.report.report_text(line);
        }
        for (index, result) in results.iter().enumerate() {
            // always take last phase for success criteria
            let target_result = &result[result.len() - 1];
            let rate = target_result.stats.rate();
            if let Some(criteria) = self.criteria.get(index) {
                SuccessCriteriaChecker::check_core_for_success(
                    criteria,
                    ctx.report,
                    &rate,
                    Some(&target_result.latency_breakdown),
                    Some(target_result.name.clone()),
                )?;
            }
        }
        Ok(())
    }
}

fn to_table(type_name: String, results: &[Vec<SingleRunStats>]) -> Vec<String> {
    let mut table = Vec::new();
    table.push(format!(
        "{: <40} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12}",
        type_name,
        "submitted/s",
        "committed/s",
        "expired/s",
        "rejected/s",
        "chain txn/s",
        "latency",
        "p50 lat",
        "p90 lat",
        "p99 lat",
        "batch->pos",
        "pos->prop",
        "prop->order",
        "order->commit",
        "actual dur"
    ));

    for run_results in results {
        for result in run_results {
            let rate = result.stats.rate();
            table.push(format!(
                "{: <40} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12.3} | {: <12.3} | {: <12.3} | {: <12.3} | {: <12}",
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
                result.latency_breakdown.get_samples(&LatencyBreakdownSlice::QsBatchToPos).max_sample(),
                result.latency_breakdown.get_samples(&LatencyBreakdownSlice::QsPosToProposal).max_sample(),
                result.latency_breakdown.get_samples(&LatencyBreakdownSlice::ConsensusProposalToOrdered).max_sample(),
                result.latency_breakdown.get_samples(&LatencyBreakdownSlice::ConsensusOrderedToCommit).max_sample(),
                result.actual_duration.as_secs()
            ));
        }
    }

    table
}
