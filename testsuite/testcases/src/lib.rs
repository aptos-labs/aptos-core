// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod compatibility_test;
pub mod consensus_reliability_tests;
pub mod dag_onchain_enable_test;
pub mod forge_setup_test;
pub mod framework_upgrade;
pub mod fullnode_reboot_stress_test;
pub mod load_vs_perf_benchmark;
pub mod modifiers;
pub mod multi_region_network_test;
pub mod network_bandwidth_test;
pub mod network_loss_test;
pub mod network_partition_test;
pub mod partial_nodes_down_test;
pub mod performance_test;
pub mod public_fullnode_performance;
pub mod quorum_store_onchain_enable_test;
pub mod reconfiguration_test;
pub mod state_sync_performance;
pub mod three_region_simulation_test;
pub mod twin_validator_test;
pub mod two_traffics_test;
pub mod validator_join_leave_test;
pub mod validator_reboot_stress_test;

use anyhow::Context;
use velor_forge::{
    prometheus_metrics::{fetch_latency_breakdown, LatencyBreakdown},
    EmitJob, EmitJobRequest, NetworkContext, NetworkContextSynchronizer, NetworkTest, NodeExt,
    Result, Swarm, SwarmExt, Test, TestReport, TxnEmitter, TxnStats, Version,
};
use velor_rest_client::Client as RestClient;
use velor_sdk::{transaction_builder::TransactionFactory, types::PeerId};
use async_trait::async_trait;
use futures::future::join_all;
use log::info;
use rand::{rngs::StdRng, SeedableRng};
use std::{
    borrow::Cow,
    fmt::Write,
    ops::DerefMut,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::runtime::{Handle, Runtime};

pub const WARMUP_DURATION_FRACTION: f32 = 0.07;
pub const COOLDOWN_DURATION_FRACTION: f32 = 0.04;

async fn batch_update(
    ctx: &mut NetworkContext<'_>,
    validators_to_update: &[PeerId],
    version: &Version,
) -> Result<()> {
    for validator in validators_to_update {
        ctx.swarm
            .write()
            .await
            .upgrade_validator(*validator, version)
            .await?;
    }

    ctx.swarm.read().await.health_check().await?;
    let deadline = Instant::now() + Duration::from_secs(60);
    for validator in validators_to_update {
        ctx.swarm
            .read()
            .await
            .validator(*validator)
            .unwrap()
            .wait_until_healthy(deadline)
            .await?;
    }

    Ok(())
}

async fn batch_update_gradually(
    ctxa: NetworkContextSynchronizer<'_>,
    validators_to_update: &[PeerId],
    version: &Version,
    wait_until_healthy: bool,
    delay: Duration,
    max_wait: Duration,
) -> Result<()> {
    for validator in validators_to_update {
        info!("batch_update_gradually upgrade start: {}", validator);
        {
            ctxa.ctx
                .lock()
                .await
                .swarm
                .write()
                .await
                .upgrade_validator(*validator, version)
                .await?;
        }
        if wait_until_healthy {
            info!("batch_update_gradually upgrade waiting: {}", validator);
            let deadline = Instant::now() + max_wait;
            ctxa.ctx
                .lock()
                .await
                .swarm
                .read()
                .await
                .validator(*validator)
                .unwrap()
                .wait_until_healthy(deadline)
                .await?;
            info!("batch_update_gradually upgrade healthy: {}", validator);
        }
        if !delay.is_zero() {
            info!("batch_update_gradually upgrade delay: {:?}", delay);
            tokio::time::sleep(delay).await;
        }
        info!("batch_update_gradually upgrade done: {}", validator);
    }

    ctxa.ctx
        .lock()
        .await
        .swarm
        .read()
        .await
        .health_check()
        .await?;

    Ok(())
}

pub async fn create_emitter_and_request(
    swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
    mut emit_job_request: EmitJobRequest,
    nodes: &[PeerId],
    rng: StdRng,
) -> Result<(TxnEmitter, EmitJobRequest)> {
    // as we are loading nodes, use higher client timeout
    let client_timeout = Duration::from_secs(30);

    let chain_info = swarm.read().await.chain_info();
    let transaction_factory = TransactionFactory::new(chain_info.chain_id);
    let rest_cli = swarm
        .read()
        .await
        .validators()
        .next()
        .unwrap()
        .rest_client();
    let emitter = TxnEmitter::new(transaction_factory, rng, rest_cli);

    emit_job_request = emit_job_request.rest_clients(
        swarm
            .read()
            .await
            .get_clients_for_peers(nodes, client_timeout),
    );
    Ok((emitter, emit_job_request))
}

pub fn traffic_emitter_runtime() -> Result<Runtime> {
    let runtime = velor_runtimes::spawn_named_runtime("emitter".into(), Some(64));
    Ok(runtime)
}

pub async fn generate_traffic(
    ctx: &mut NetworkContext<'_>,
    nodes: &[PeerId],
    duration: Duration,
) -> Result<TxnStats> {
    let emit_job_request = ctx.emit_job.clone();
    let rng = SeedableRng::from_rng(ctx.core().rng())?;
    let (emitter, emit_job_request) =
        create_emitter_and_request(ctx.swarm.clone(), emit_job_request, nodes, rng).await?;

    let stats = emitter
        .emit_txn_for(
            ctx.swarm.read().await.chain_info().root_account,
            emit_job_request,
            duration,
        )
        .await?;

    Ok(stats)
}

pub enum LoadDestination {
    AllNodes,
    AllValidators,
    AllFullnodes,
    // Send to AllFullnodes, if any exist, otherwise to AllValidators
    FullnodesOtherwiseValidators,
    Peers(Vec<PeerId>),
}

impl LoadDestination {
    async fn get_destination_nodes(
        self,
        swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
    ) -> Vec<PeerId> {
        let swarm = swarm.read().await;
        let all_validators = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();
        let all_fullnodes = swarm.full_nodes().map(|v| v.peer_id()).collect::<Vec<_>>();

        match self {
            LoadDestination::AllNodes => [&all_validators[..], &all_fullnodes[..]].concat(),
            LoadDestination::AllValidators => all_validators,
            LoadDestination::AllFullnodes => all_fullnodes,
            LoadDestination::FullnodesOtherwiseValidators => {
                if all_fullnodes.is_empty() {
                    all_validators
                } else {
                    all_fullnodes
                }
            },
            LoadDestination::Peers(peers) => peers,
        }
    }
}

#[async_trait]
pub trait NetworkLoadTest: Test {
    async fn setup<'a>(&self, _ctx: &mut NetworkContext<'a>) -> Result<LoadDestination> {
        Ok(LoadDestination::FullnodesOtherwiseValidators)
    }

    // Load is started before this function is called, and stops after this function returns.
    // Expected duration is passed into this function, expecting this function to take that much
    // time to finish. How long this function takes will dictate how long the actual test lasts.
    async fn test(
        &self,
        _swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
        _report: &mut TestReport,
        duration: Duration,
    ) -> Result<()> {
        tokio::time::sleep(duration).await;
        Ok(())
    }

    async fn finish<'a>(&self, _ctx: &mut NetworkContext<'a>) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl NetworkTest for dyn NetworkLoadTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        let mut ctx_locker = ctx.ctx.lock().await;
        let ctx = ctx_locker.deref_mut();
        let start_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        let (start_version, _) = ctx
            .swarm
            .read()
            .await
            .get_client_with_newest_ledger_version()
            .await
            .context("no clients replied for start version")?;
        let emit_job_request = ctx.emit_job.clone();
        let duration = ctx.global_duration;
        let stats_by_phase = self
            .network_load_test(
                ctx,
                emit_job_request,
                duration,
                WARMUP_DURATION_FRACTION,
                COOLDOWN_DURATION_FRACTION,
                None,
            )
            .await?;

        let phased = stats_by_phase.len() > 1;
        for (phase, phase_stats) in stats_by_phase.iter().enumerate() {
            let test_name = if phased {
                format!("{}_phase_{}", self.name(), phase)
            } else {
                self.name().to_string()
            };
            ctx.report
                .report_txn_stats(test_name, &phase_stats.emitter_stats);
            ctx.report.report_text(format!(
                "Latency breakdown for phase {}: {:?}",
                phase,
                phase_stats
                    .latency_breakdown
                    .keys()
                    .into_iter()
                    .map(|slice| {
                        let slice_samples = phase_stats
                            .latency_breakdown
                            .get_samples(&slice)
                            .expect("Could not get samples");
                        format!(
                            "{:?}: max: {:.3}, avg: {:.3}",
                            slice,
                            slice_samples.max_sample(),
                            slice_samples.avg_sample()
                        )
                    })
                    .collect::<Vec<_>>()
            ));
        }

        let end_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        let (end_version, _) = ctx
            .swarm
            .read()
            .await
            .get_client_with_newest_ledger_version()
            .await
            .context("no clients replied for end version")?;

        self.finish(ctx).await.context("finish NetworkLoadTest ")?;

        for phase_stats in stats_by_phase.into_iter() {
            ctx.check_for_success(
                &phase_stats.emitter_stats,
                phase_stats.actual_duration,
                &phase_stats.latency_breakdown,
                start_timestamp as i64,
                end_timestamp as i64,
                start_version,
                end_version,
            )
            .await
            .context("check for success")?;
        }

        Ok(())
    }
}

pub async fn create_buffered_load(
    swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
    nodes_to_send_load_to: &[PeerId],
    emit_job_request: EmitJobRequest,
    duration: Duration,
    warmup_duration_fraction: f32,
    cooldown_duration_fraction: f32,
    mut inner_test_and_report: Option<(&dyn NetworkLoadTest, &mut TestReport)>,
    mut synchronized_with_job: Option<&mut EmitJob>,
) -> Result<Vec<LoadTestPhaseStats>> {
    // Generate some traffic
    let (mut emitter, emit_job_request) = create_emitter_and_request(
        swarm.clone(),
        emit_job_request,
        nodes_to_send_load_to,
        StdRng::from_entropy(),
    )
    .await
    .context("create emitter")?;

    let clients = swarm
        .read()
        .await
        .get_clients_for_peers(nodes_to_send_load_to, Duration::from_secs(10));

    let mut stats_tracking_phases = emit_job_request.get_num_phases();
    assert!(stats_tracking_phases > 0 && stats_tracking_phases != 2);
    if stats_tracking_phases == 1 {
        stats_tracking_phases = 3;
    }

    info!("Starting emitting txns for {}s", duration.as_secs());
    let mut job = emitter
        .start_job(
            swarm.read().await.chain_info().root_account,
            emit_job_request,
            stats_tracking_phases,
        )
        .await
        .context("start emitter job")?;

    let total_start = PhaseTimingStart::now();

    let warmup_duration = duration.mul_f32(warmup_duration_fraction);
    let cooldown_duration = duration.mul_f32(cooldown_duration_fraction);
    let test_duration = duration - warmup_duration - cooldown_duration;
    let phase_duration = test_duration.div_f32((stats_tracking_phases - 2) as f32);

    job = job.periodic_stat_forward(warmup_duration, 60).await;
    info!("{}s warmup finished", warmup_duration.as_secs());

    if let Some(job) = synchronized_with_job.as_mut() {
        job.start_next_phase()
    }

    let mut phase_timing = Vec::new();
    let mut phase_start_network_state = Vec::new();
    let test_start = Instant::now();
    for i in 0..stats_tracking_phases - 2 {
        phase_start_network_state.push(NetworkState::new(&clients).await);
        job.start_next_phase();

        if i > 0 {
            info!(
                "Starting test phase {} out of {}",
                i,
                stats_tracking_phases - 2,
            );
        }
        let phase_start = PhaseTimingStart::now();

        let join_stats = Handle::current().spawn(job.periodic_stat_forward(phase_duration, 60));
        if let Some((inner_test, context)) = inner_test_and_report.as_mut() {
            inner_test
                .test(swarm.clone(), context, phase_duration)
                .await
                .context("test NetworkLoadTest")?;
        }
        job = join_stats.await.context("join stats")?;
        phase_timing.push(phase_start.elapsed());
    }
    let actual_test_duration = test_start.elapsed();
    info!(
        "{}s test finished after {}s",
        test_duration.as_secs(),
        actual_test_duration.as_secs()
    );

    phase_start_network_state.push(NetworkState::new(&clients).await);
    job.start_next_phase();
    if let Some(job) = synchronized_with_job.as_mut() {
        job.start_next_phase()
    }
    let cooldown_start = Instant::now();

    let cooldown_used = cooldown_start.elapsed();
    if cooldown_used < cooldown_duration {
        job = job
            .periodic_stat_forward(cooldown_duration - cooldown_used, 60)
            .await;
    }
    info!("{}s cooldown finished", cooldown_duration.as_secs());

    let total_timing = total_start.elapsed();
    info!(
        "Emitting txns ran for {} secs(from {} to {}), stopping job...",
        duration.as_secs(),
        total_timing.start_unixtime_s,
        total_timing.end_unixtime_s,
    );
    let stats_by_phase = job.stop_job().await;

    info!("Stopped job");
    info!("Warmup stats: {}", stats_by_phase[0].rate());

    let mut stats: Option<TxnStats> = None;
    let mut stats_by_phase_filtered = Vec::new();
    for i in 0..stats_tracking_phases - 2 {
        let next_i = i + 1;
        let cur = &stats_by_phase[next_i];
        info!("Test stats [test phase {}]: {}", i, cur.rate());
        stats = if let Some(previous) = stats {
            Some(&previous + cur)
        } else {
            Some(cur.clone())
        };
        let latency_breakdown = fetch_latency_breakdown(
            swarm.clone(),
            phase_timing[i].start_unixtime_s,
            phase_timing[i].end_unixtime_s,
        )
        .await?;
        info!(
            "Latency breakdown details for phase {}: from {} to {}: {:?}",
            i, phase_timing[i].start_unixtime_s, phase_timing[i].end_unixtime_s, latency_breakdown
        );
        stats_by_phase_filtered.push(LoadTestPhaseStats {
            emitter_stats: cur.clone(),
            actual_duration: phase_timing[i].duration,
            phase_start_unixtime_s: phase_timing[i].start_unixtime_s,
            phase_end_unixtime_s: phase_timing[i].end_unixtime_s,
            ledger_transactions: NetworkState::ledger_transactions(
                &phase_start_network_state[i],
                &phase_start_network_state[next_i],
            ),
            latency_breakdown,
        });
    }
    info!("Cooldown stats: {}", stats_by_phase.last().unwrap().rate());

    Ok(stats_by_phase_filtered)
}

impl dyn NetworkLoadTest + '_ {
    pub async fn network_load_test<'a>(
        &self,
        ctx: &mut NetworkContext<'a>,
        emit_job_request: EmitJobRequest,
        duration: Duration,
        warmup_duration_fraction: f32,
        cooldown_duration_fraction: f32,
        synchronized_with_job: Option<&mut EmitJob>,
    ) -> Result<Vec<LoadTestPhaseStats>> {
        let destination = self.setup(ctx).await.context("setup NetworkLoadTest")?;
        let nodes_to_send_load_to = destination.get_destination_nodes(ctx.swarm.clone()).await;

        create_buffered_load(
            ctx.swarm.clone(),
            &nodes_to_send_load_to,
            emit_job_request,
            duration,
            warmup_duration_fraction,
            cooldown_duration_fraction,
            Some((self, ctx.report)),
            synchronized_with_job,
        )
        .await
    }
}

struct PhaseTimingStart {
    now: Instant,
    unixtime_s: u64,
}

impl PhaseTimingStart {
    fn now() -> PhaseTimingStart {
        let now = Instant::now();
        let unixtime_s = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        PhaseTimingStart { now, unixtime_s }
    }

    fn elapsed(&self) -> PhaseTiming {
        PhaseTiming {
            duration: self.now.elapsed(),
            start_unixtime_s: self.unixtime_s,
            end_unixtime_s: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs(),
        }
    }
}

struct PhaseTiming {
    duration: Duration,
    start_unixtime_s: u64,
    end_unixtime_s: u64,
}

pub(crate) struct NetworkState {
    max_version_and_height: Option<(u64, u64)>,
}

impl NetworkState {
    pub async fn new(clients: &[RestClient]) -> NetworkState {
        let max_version_and_height =
            join_all(clients.iter().map(|client| client.get_ledger_information()))
                .await
                .into_iter()
                .filter(|r| r.is_ok())
                .map(|r| r.unwrap().into_inner())
                .map(|s| (s.version, s.block_height))
                .max();
        NetworkState {
            max_version_and_height,
        }
    }

    pub fn ledger_transactions(start: &NetworkState, end: &NetworkState) -> u64 {
        if let (Some((end_version, end_height)), Some((start_version, start_height))) =
            (end.max_version_and_height, start.max_version_and_height)
        {
            (end_version - end_height * 2) - (start_version - start_height * 2)
        } else {
            0
        }
    }
}

pub struct LoadTestPhaseStats {
    pub emitter_stats: TxnStats,
    pub actual_duration: Duration,
    pub phase_start_unixtime_s: u64,
    pub phase_end_unixtime_s: u64,
    pub ledger_transactions: u64,
    pub latency_breakdown: LatencyBreakdown,
}

pub struct CompositeNetworkTest {
    // Wrapper tests - their setup and finish methods are called, before the test ones.
    // TODO don't know how to make this array, and have forge/main.rs work
    pub wrappers: Vec<Box<dyn NetworkLoadTest>>,
    // This is the main test, return values from this test are used in setup, and
    // only it's test function is called.
    pub test: Box<dyn NetworkTest>,
}

impl CompositeNetworkTest {
    pub fn new<W: NetworkLoadTest + 'static, T: NetworkTest + 'static>(
        wrapper: W,
        test: T,
    ) -> CompositeNetworkTest {
        CompositeNetworkTest {
            wrappers: vec![Box::new(wrapper)],
            test: Box::new(test),
        }
    }

    pub fn new_with_two_wrappers<
        T1: NetworkLoadTest + 'static,
        T2: NetworkLoadTest + 'static,
        W: NetworkTest + 'static,
    >(
        wrapper1: T1,
        wrapper2: T2,
        test: W,
    ) -> CompositeNetworkTest {
        CompositeNetworkTest {
            wrappers: vec![Box::new(wrapper1), Box::new(wrapper2)],
            test: Box::new(test),
        }
    }
}

#[async_trait]
impl NetworkTest for CompositeNetworkTest {
    async fn run<'a>(&self, ctxa: NetworkContextSynchronizer<'a>) -> Result<()> {
        {
            let mut ctx_locker = ctxa.ctx.lock().await;
            let ctx = ctx_locker.deref_mut();
            for wrapper in &self.wrappers {
                wrapper.setup(ctx).await?;
            }
        }
        self.test.run(ctxa.clone()).await?;
        {
            let mut ctx_locker = ctxa.ctx.lock().await;
            let ctx = ctx_locker.deref_mut();
            for wrapper in &self.wrappers {
                wrapper.finish(ctx).await?;
            }
        }
        Ok(())
    }
}

impl Test for CompositeNetworkTest {
    fn name(&self) -> &'static str {
        "CompositeNetworkTest"
    }

    fn reporting_name(&self) -> Cow<'static, str> {
        let mut name_builder = self.test.name().to_owned();
        for wrapper in self.wrappers.iter() {
            name_builder = format!("{}({})", wrapper.name(), name_builder);
        }
        name_builder = format!("CompositeNetworkTest({}) with ", name_builder);
        Cow::Owned(name_builder)
    }
}

pub(crate) fn generate_onchain_config_blob(data: &[u8]) -> String {
    let mut buf = String::new();

    write!(buf, "vector[").unwrap();
    for (i, b) in data.iter().enumerate() {
        if i % 20 == 0 {
            if i > 0 {
                writeln!(buf).unwrap();
            }
        } else {
            write!(buf, " ").unwrap();
        }
        write!(buf, "{}u8,", b).unwrap();
    }
    write!(buf, "]").unwrap();
    buf
}
