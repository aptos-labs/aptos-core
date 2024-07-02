// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{create_emitter_and_request, LoadDestination, NetworkLoadTest};
use anyhow::Context;
use aptos_forge::{
    args::TransactionTypeArg,
    prometheus_metrics::{LatencyBreakdown, LatencyBreakdownSlice},
    success_criteria::{SuccessCriteria, SuccessCriteriaChecker},
    EmitJobMode, EmitJobRequest, NetworkContext, NetworkContextSynchronizer, NetworkTest, Result,
    Test, TxnStats, WorkflowProgress,
};
use aptos_logger::info;
use async_trait::async_trait;
use rand::SeedableRng;
use std::{fmt::Debug, ops::DerefMut, time::Duration};

// add larger warmup, as when we are exceeding the max load,
// it takes more time to fill mempool.
const PER_TEST_WARMUP_DURATION_FRACTION: f32 = 0.2;
const PER_TEST_COOLDOWN_DURATION_FRACTION: f32 = 0.05;

pub struct SingleRunStats {
    name: String,
    stats: TxnStats,
    latency_breakdown: LatencyBreakdown,
    ledger_transactions: u64,
    actual_duration: Duration,
}

#[derive(Debug)]
pub enum Workloads {
    TPS(Vec<usize>),
    TRANSACTIONS(Vec<TransactionWorkload>),
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

    fn num_phases(&self, index: usize) -> usize {
        match self {
            Self::TPS(_) => 1,
            Self::TRANSACTIONS(workloads) => {
                if workloads[index].is_phased() {
                    2
                } else {
                    1
                }
            },
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

    fn split_duration(&self, global_duration: Duration) -> (Duration, Duration) {
        let total_phases: usize = (0..self.len()).map(|index| self.num_phases(index)).sum();
        let phase_duration = global_duration.div_f32(
            total_phases as f32 + PER_TEST_WARMUP_DURATION_FRACTION * (self.len() - 1) as f32,
        );
        let buffer = phase_duration.mul_f32(PER_TEST_WARMUP_DURATION_FRACTION);
        (phase_duration, buffer)
    }
}

#[derive(Debug, Copy, Clone)]
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
            TransactionTypeArg::AccountGenerationLargePool.materialize_default();

        let request = request.mode(EmitJobMode::MaxLoad {
            mempool_backlog: self.mempool_backlog,
        });

        if self.is_phased() {
            let write_type = self.transaction_type.materialize(
                self.num_modules,
                true,
                WorkflowProgress::when_done_default(),
            );
            request.transaction_mix_per_phase(vec![
                // warmup
                vec![(account_creation_type, 1)],
                vec![(account_creation_type, 1)],
                vec![(write_type, 1)],
                // cooldown
                vec![(write_type, 1)],
            ])
        } else {
            request.transaction_type(self.transaction_type.materialize(
                self.num_modules,
                false,
                WorkflowProgress::when_done_default(),
            ))
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

pub struct ContinuousTraffic {
    pub traffic: EmitJobRequest,
    pub criteria: Option<SuccessCriteria>,
}

pub struct LoadVsPerfBenchmark {
    pub test: Box<dyn NetworkLoadTest>,
    pub workloads: Workloads,
    pub criteria: Vec<SuccessCriteria>,

    pub continuous_traffic: Option<ContinuousTraffic>,
}

impl Test for LoadVsPerfBenchmark {
    fn name(&self) -> &'static str {
        "continuous progress test"
    }
}

impl LoadVsPerfBenchmark {
    async fn evaluate_single(
        &self,
        ctx: &mut NetworkContext<'_>,
        workloads: &Workloads,
        index: usize,
        duration: Duration,
    ) -> Result<Vec<SingleRunStats>> {
        let rng = SeedableRng::from_rng(ctx.core().rng())?;
        let emit_job_request = workloads.configure(index, ctx.emit_job.clone());
        let stats_by_phase = self
            .test
            .network_load_test(
                ctx,
                emit_job_request,
                duration,
                PER_TEST_WARMUP_DURATION_FRACTION,
                PER_TEST_COOLDOWN_DURATION_FRACTION,
                rng,
            )
            .await?;

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

#[async_trait]
impl NetworkTest for LoadVsPerfBenchmark {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        assert!(
            self.criteria.is_empty() || self.criteria.len() == self.workloads.len(),
            "Invalid config, {} criteria and {} workloads given",
            self.criteria.len(),
            self.workloads.len(),
        );

        let mut ctx_locker = ctx.ctx.lock().await;
        let ctx = ctx_locker.deref_mut();

        let mut continous_job = if let Some(continuous_traffic) = &self.continuous_traffic {
            let nodes_to_send_load_to = LoadDestination::FullnodesOtherwiseValidators
                .get_destination_nodes(ctx.swarm.clone())
                .await;
            let rng = SeedableRng::from_rng(ctx.core().rng())?;
            let (mut emitter, emit_job_request) = create_emitter_and_request(
                ctx.swarm.clone(),
                continuous_traffic.traffic.clone(),
                &nodes_to_send_load_to,
                rng,
            )
            .await
            .context("create emitter")?;

            let job = emitter
                .start_job(
                    ctx.swarm.read().await.chain_info().root_account,
                    emit_job_request,
                    1 + 2 * self.workloads.len(),
                )
                .await
                .context("start emitter job")?;
            Some(job)
        } else {
            None
        };

        let (phase_duration, buffer) = self.workloads.split_duration(ctx.global_duration);

        let mut results = Vec::new();
        for index in 0..self.workloads.len() {
            if index != 0 {
                info!("Sleeping in between loadtests, for {}s", buffer.as_secs());
                std::thread::sleep(buffer);
            }

            if let Some(job) = continous_job.as_mut() {
                job.start_next_phase()
            }

            info!("Starting for {:?}", self.workloads);
            results.push(
                self.evaluate_single(
                    ctx,
                    &self.workloads,
                    index,
                    phase_duration
                        .checked_mul(self.workloads.num_phases(index) as u32)
                        .unwrap(),
                )
                .await?,
            );

            if let Some(job) = continous_job.as_mut() {
                job.start_next_phase()
            }

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

        let continuous_results = match continous_job {
            Some(job) => {
                let stats_by_phase = job.stop_job().await;

                let mut result = vec![];
                for (phase, phase_stats) in stats_by_phase.into_iter().enumerate() {
                    if phase % 2 != 0 {
                        result.push((
                            format!("continuous with traffic {}", phase / 2),
                            phase_stats,
                        ));
                    }
                }

                let table = to_table_continuous("continuous traffic".to_string(), &result);
                for line in table {
                    ctx.report.report_text(line);
                }

                Some(result)
            },
            None => None,
        };

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

        if let Some(results) = continuous_results {
            for (name, stats) in results {
                let rate = stats.rate();
                if let Some(criteria) = &self.continuous_traffic.as_ref().unwrap().criteria {
                    SuccessCriteriaChecker::check_core_for_success(
                        criteria,
                        ctx.report,
                        &rate,
                        None,
                        Some(name),
                    )?;
                }
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

fn to_table_continuous(type_name: String, results: &[(String, TxnStats)]) -> Vec<String> {
    let mut table = Vec::new();
    table.push(format!(
        "{: <40} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12}",
        type_name,
        "submitted/s",
        "committed/s",
        "expired/s",
        "rejected/s",
        "latency",
        "p50 lat",
        "p90 lat",
        "p99 lat",
    ));

    for (name, stats) in results {
        let rate = stats.rate();
        table.push(format!(
            "{: <40} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12}",
            name,
            rate.submitted,
            rate.committed,
            rate.expired,
            rate.failed_submission,
            rate.latency,
            rate.p50_latency,
            rate.p90_latency,
            rate.p99_latency,
        ));
    }

    table
}

#[test]
fn test_phases_duration() {
    use assert_approx_eq::assert_approx_eq;
    use std::ops::{Add, Mul};

    let one_phase = TransactionWorkload {
        transaction_type: TransactionTypeArg::CoinTransfer,
        num_modules: 1,
        unique_senders: false,
        mempool_backlog: 20000,
    };
    let two_phase = TransactionWorkload {
        transaction_type: TransactionTypeArg::ModifyGlobalResource,
        num_modules: 1,
        unique_senders: true,
        mempool_backlog: 20000,
    };

    {
        let workload = Workloads::TRANSACTIONS(vec![one_phase]);
        let (phase, _buffer) = workload.split_duration(Duration::from_secs(1));
        assert_approx_eq!(phase.as_secs_f32(), 1.0);
    }

    {
        let workload = Workloads::TRANSACTIONS(vec![one_phase, one_phase]);
        let (phase, buffer) = workload.split_duration(Duration::from_secs(1));
        assert_approx_eq!(phase.as_secs_f32(), 1.0 / 2.2);
        assert_approx_eq!(buffer.as_secs_f32(), 1.0 / 2.2 * 0.2);
        assert_approx_eq!(phase.add(phase).add(buffer).as_secs_f32(), 1.0);
    }

    {
        let workload = Workloads::TRANSACTIONS(vec![two_phase]);
        let (phase, _buffer) = workload.split_duration(Duration::from_secs(1));
        assert_approx_eq!(phase.as_secs_f32(), 0.5);
    }

    {
        let workload =
            Workloads::TRANSACTIONS(vec![one_phase, one_phase, two_phase, two_phase, two_phase]);
        let (phase, buffer) = workload.split_duration(Duration::from_secs(1));
        assert_approx_eq!(phase.as_secs_f32(), 1.0 / 8.8);
        assert_approx_eq!(buffer.as_secs_f32(), 1.0 / 8.8 * 0.2);
        assert_approx_eq!(phase.mul(8).add(buffer.mul(4)).as_secs_f32(), 1.0);
    }
}
