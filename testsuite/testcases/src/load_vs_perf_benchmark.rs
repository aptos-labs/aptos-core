// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{create_emitter_and_request, LoadDestination, NetworkLoadTest};
use anyhow::Context;
use aptos_forge::{
    args::TransactionTypeArg,
    emitter::NumAccountsMode,
    prometheus_metrics::{LatencyBreakdown, LatencyBreakdownSlice, MetricSamples},
    success_criteria::{SuccessCriteria, SuccessCriteriaChecker},
    EmitJob, EmitJobMode, EmitJobRequest, NetworkContext, NetworkContextSynchronizer, NetworkTest,
    ReplayProtectionType, Result, Test, TxnStats, WorkflowProgress,
};
use async_trait::async_trait;
use log::{error, info};
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

    fn desc(&self, index: usize) -> String {
        match self {
            Self::TPS(tpss) => {
                format!("TPS({})", tpss[index])
            },
            Self::TRANSACTIONS(workloads) => format!("TRANSACTIONS({:?})", workloads[index]),
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

#[derive(Debug, Clone)]
pub struct TransactionWorkload {
    pub transaction_type: TransactionTypeArg,
    pub replay_protection_type: ReplayProtectionType,
    pub num_modules: usize,
    pub unique_senders: bool,
    pub load: EmitJobMode,
    pub transactions_per_account_override: Option<usize>,
}

impl TransactionWorkload {
    pub fn new(
        transaction_type: TransactionTypeArg,
        replay_protection_type: ReplayProtectionType,
        mempool_backlog: usize,
    ) -> Self {
        Self {
            transaction_type,
            replay_protection_type,
            num_modules: 1,
            unique_senders: false,
            load: EmitJobMode::MaxLoad { mempool_backlog },
            transactions_per_account_override: None,
        }
    }

    pub fn new_const_tps(
        transaction_type: TransactionTypeArg,
        replay_protection_type: ReplayProtectionType,
        tps: usize,
    ) -> Self {
        Self {
            transaction_type,
            replay_protection_type,
            num_modules: 1,
            unique_senders: false,
            load: EmitJobMode::ConstTps { tps },
            transactions_per_account_override: None,
        }
    }

    pub fn new_wave_tps(
        transaction_type: TransactionTypeArg,
        replay_protection_type: ReplayProtectionType,
        average_tps: usize,
        wave_ratio: f32,
        num_waves: usize,
    ) -> Self {
        Self {
            transaction_type,
            replay_protection_type,
            num_modules: 1,
            unique_senders: false,
            load: EmitJobMode::WaveTps {
                average_tps,
                wave_ratio,
                num_waves,
            },
            transactions_per_account_override: None,
        }
    }

    pub fn with_num_modules(mut self, num_modules: usize) -> Self {
        self.num_modules = num_modules;
        self
    }

    pub fn with_unique_senders(mut self) -> Self {
        self.unique_senders = true;
        self
    }

    pub fn with_transactions_per_account(mut self, transactions_per_account: usize) -> Self {
        self.transactions_per_account_override = Some(transactions_per_account);
        self
    }

    fn is_phased(&self) -> bool {
        self.unique_senders
    }

    fn configure(&self, request: EmitJobRequest) -> EmitJobRequest {
        let account_creation_type =
            TransactionTypeArg::AccountGenerationLargePool.materialize_default();

        let mut request = request.mode(self.load.clone());

        if let Some(transactions_per_account) = &self.transactions_per_account_override {
            request = request.num_accounts_mode(NumAccountsMode::TransactionsPerAccount(
                *transactions_per_account,
            ))
        }

        if self.is_phased() {
            let write_type = self.transaction_type.materialize(
                self.num_modules,
                true,
                WorkflowProgress::when_done_default(),
            );
            request.transaction_mix_per_phase(vec![
                // warmup
                vec![(
                    account_creation_type.clone(),
                    self.replay_protection_type,
                    1,
                )],
                vec![(account_creation_type, self.replay_protection_type, 1)],
                vec![(write_type.clone(), self.replay_protection_type, 1)],
                // cooldown
                vec![(write_type, self.replay_protection_type, 1)],
            ])
        } else {
            request.transaction_type(
                self.transaction_type.materialize(
                    self.num_modules,
                    false,
                    WorkflowProgress::when_done_default(),
                ),
                self.replay_protection_type,
            )
        }
    }

    fn phase_name(&self, phase: usize) -> String {
        format!(
            "{}{}[{}]",
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
            match self.load {
                EmitJobMode::MaxLoad { mempool_backlog } =>
                    format!("B:{:.1}k", mempool_backlog as f32 / 1000.0),
                EmitJobMode::ConstTps { tps } => format!("T:{:.1}k", tps as f32 / 1000.0),
                EmitJobMode::WaveTps { average_tps, .. } =>
                    format!("T:~{:.1}k", average_tps as f32 / 1000.0),
            },
            // ,
        )
    }
}

pub struct BackgroundTraffic {
    pub traffic: EmitJobRequest,
    pub criteria: Vec<SuccessCriteria>,
}

pub struct LoadVsPerfBenchmark {
    pub test: Box<dyn NetworkLoadTest>,
    pub workloads: Workloads,
    pub criteria: Vec<SuccessCriteria>,

    pub background_traffic: Option<BackgroundTraffic>,
}

impl Test for LoadVsPerfBenchmark {
    fn name(&self) -> &'static str {
        match self.workloads {
            Workloads::TPS(_) => "load vs perf test",
            Workloads::TRANSACTIONS(_) => "workload vs perf test",
        }
    }
}

impl LoadVsPerfBenchmark {
    async fn evaluate_single(
        &self,
        ctx: &mut NetworkContext<'_>,
        workloads: &Workloads,
        index: usize,
        duration: Duration,
        synchronized_with_job: Option<&mut EmitJob>,
    ) -> Result<Vec<SingleRunStats>> {
        let emit_job_request = workloads.configure(index, ctx.emit_job.clone());
        let stats_by_phase = self
            .test
            .network_load_test(
                ctx,
                emit_job_request,
                duration,
                PER_TEST_WARMUP_DURATION_FRACTION,
                PER_TEST_COOLDOWN_DURATION_FRACTION,
                synchronized_with_job,
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

        let mut background_job = if let Some(background_traffic) = &self.background_traffic {
            let nodes_to_send_load_to = LoadDestination::FullnodesOtherwiseValidators
                .get_destination_nodes(ctx.swarm.clone())
                .await;
            let rng = SeedableRng::from_rng(ctx.core().rng())?;
            let (mut emitter, emit_job_request) = create_emitter_and_request(
                ctx.swarm.clone(),
                background_traffic.traffic.clone(),
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

            info!("Starting for [{}]: {:?}", index, self.workloads.desc(index));
            results.push(
                self.evaluate_single(
                    ctx,
                    &self.workloads,
                    index,
                    phase_duration
                        .checked_mul(self.workloads.num_phases(index) as u32)
                        .unwrap(),
                    background_job.as_mut(),
                )
                .await
                .inspect_err(|e| {
                    error!(
                        "Failed evaluating single run [{}]: {:?} with {:?}",
                        index,
                        self.workloads.desc(index),
                        e
                    )
                })?,
            );

            let table = to_table(self.workloads.type_name(), &results);
            for line in table {
                info!("{}", line);
            }

            if let Some(job) = &background_job {
                let stats_by_phase = job.peek_and_accumulate();
                for line in to_table_background(
                    "background traffic".to_string(),
                    &extract_background_stats(stats_by_phase),
                ) {
                    info!("{}", line);
                }
            }

            // Note: uncomment below to perform reconfig during a test
            // let mut aptos_info = ctx.swarm().aptos_public_info();
            // runtime.block_on(aptos_info.reconfig());
        }

        let table = to_table(self.workloads.type_name(), &results);
        for line in table {
            ctx.report.report_text(line);
        }

        let background_results = match background_job {
            Some(job) => {
                let stats_by_phase = job.stop_job().await;

                let result = extract_background_stats(stats_by_phase);

                for line in to_table_background("background traffic".to_string(), &result) {
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

        if let Some(results) = background_results {
            for (index, (name, stats)) in results.into_iter().enumerate() {
                let rate = stats.rate();
                if let Some(criteria) = self
                    .background_traffic
                    .as_ref()
                    .unwrap()
                    .criteria
                    .get(index)
                {
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

fn extract_background_stats(stats_by_phase: Vec<TxnStats>) -> Vec<(String, TxnStats)> {
    let mut result = vec![];
    for (phase, phase_stats) in stats_by_phase.into_iter().enumerate() {
        if phase % 2 != 0 {
            result.push((
                format!("background with traffic {}", phase / 2),
                phase_stats,
            ));
        }
    }
    result
}

fn to_table(type_name: String, results: &[Vec<SingleRunStats>]) -> Vec<String> {
    let name_width = (results
        .iter()
        .flatten()
        .map(|result| result.name.len())
        .max()
        .unwrap_or(28)
        + 2)
    .max(30);

    let mut table = Vec::new();
    table.push(format!(
        "{: <name_width$} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <14} | {: <12} | {: <13} | {: <12} | {: <12} | {: <12} | {: <12}",
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
        "mempool->block",
        "prop->order",
        "order->commit",
        "actual dur",
        // optional indexer metrics
        "idx_fn",
        "idx_cache",
        "idx_data",
    ));

    for run_results in results {
        for result in run_results {
            let rate = result.stats.rate();
            table.push(format!(
                "{: <name_width$} | {: <12.2} | {: <12.2} | {: <12.2} | {: <12.2} | {: <12.2} | {: <12.3} | {: <12.3} | {: <12.3} | {: <12.3} | {: <14.3} | {: <12.3} | {: <13.3} | {: <12.3} | {: <12} | {: <12.3} | {: <12.3}",
                result.name,
                rate.submitted,
                rate.committed,
                rate.expired,
                rate.failed_submission,
                result.ledger_transactions / result.actual_duration.as_secs(),
                rate.latency / 1000.0,
                rate.p50_latency as f64 / 1000.0,
                rate.p90_latency as f64 / 1000.0,
                rate.p99_latency as f64 / 1000.0,
                result.latency_breakdown.get_samples(&LatencyBreakdownSlice::MempoolToBlockCreation).unwrap_or(&MetricSamples::default()).max_sample(),
                result.latency_breakdown.get_samples(&LatencyBreakdownSlice::ConsensusProposalToOrdered).unwrap_or(&MetricSamples::default()).max_sample(),
                result.latency_breakdown.get_samples(&LatencyBreakdownSlice::ConsensusOrderedToCommit).unwrap_or(&MetricSamples::default()).max_sample(),
                result.actual_duration.as_secs(),
                // optional indexer metrics
                result.latency_breakdown.get_samples(&LatencyBreakdownSlice::IndexerFullnodeProcessedBatch).unwrap_or(&MetricSamples::default()).max_sample(),
                result.latency_breakdown.get_samples(&LatencyBreakdownSlice::IndexerCacheWorkerProcessedBatch).unwrap_or(&MetricSamples::default()).max_sample(),
                result.latency_breakdown.get_samples(&LatencyBreakdownSlice::IndexerDataServiceAllChunksSent).unwrap_or(&MetricSamples::default()).max_sample(),
            ));
        }
    }

    table
}

fn to_table_background(type_name: String, results: &[(String, TxnStats)]) -> Vec<String> {
    let name_width = (results
        .iter()
        .map(|(name, _)| name.len())
        .max()
        .unwrap_or(28)
        + 2)
    .max(30);

    let mut table = Vec::new();
    table.push(format!(
        "{: <name_width$} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12} | {: <12}",
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
            "{: <name_width$} | {: <12.2} | {: <12.2} | {: <12.2} | {: <12.2} | {: <12.3} | {: <12.3} | {: <12.3} | {: <12.3}",
            name,
            rate.submitted,
            rate.committed,
            rate.expired,
            rate.failed_submission,
            rate.latency / 1000.0,
            rate.p50_latency as f64 / 1000.0,
            rate.p90_latency as f64 / 1000.0,
            rate.p99_latency as f64 / 1000.0,
        ));
    }

    table
}

#[test]
fn test_phases_duration() {
    use assert_approx_eq::assert_approx_eq;
    use std::ops::{Add, Mul};

    let one_phase = TransactionWorkload::new(
        TransactionTypeArg::CoinTransfer,
        ReplayProtectionType::SequenceNumber,
        20000,
    );
    let two_phase = TransactionWorkload::new(
        TransactionTypeArg::ModifyGlobalResource,
        ReplayProtectionType::Nonce,
        20000,
    )
    .with_unique_senders();

    {
        let workload = Workloads::TRANSACTIONS(vec![one_phase.clone()]);
        let (phase, _buffer) = workload.split_duration(Duration::from_secs(1));
        assert_approx_eq!(phase.as_secs_f32(), 1.0);
    }

    {
        let workload = Workloads::TRANSACTIONS(vec![one_phase.clone(), one_phase.clone()]);
        let (phase, buffer) = workload.split_duration(Duration::from_secs(1));
        assert_approx_eq!(phase.as_secs_f32(), 1.0 / 2.2);
        assert_approx_eq!(buffer.as_secs_f32(), 1.0 / 2.2 * 0.2);
        assert_approx_eq!(phase.add(phase).add(buffer).as_secs_f32(), 1.0);
    }

    {
        let workload = Workloads::TRANSACTIONS(vec![two_phase.clone()]);
        let (phase, _buffer) = workload.split_duration(Duration::from_secs(1));
        assert_approx_eq!(phase.as_secs_f32(), 0.5);
    }

    {
        let workload = Workloads::TRANSACTIONS(vec![
            one_phase.clone(),
            one_phase,
            two_phase.clone(),
            two_phase.clone(),
            two_phase,
        ]);
        let (phase, buffer) = workload.split_duration(Duration::from_secs(1));
        assert_approx_eq!(phase.as_secs_f32(), 1.0 / 8.8);
        assert_approx_eq!(buffer.as_secs_f32(), 1.0 / 8.8 * 0.2);
        assert_approx_eq!(phase.mul(8).add(buffer.mul(4)).as_secs_f32(), 1.0);
    }
}
