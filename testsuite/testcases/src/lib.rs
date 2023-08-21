// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod compatibility_test;
pub mod consensus_reliability_tests;
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
use aptos_forge::{
    prometheus_metrics::{fetch_latency_breakdown, LatencyBreakdown},
    EmitJobRequest, NetworkContext, NetworkTest, NodeExt, Result, Swarm, SwarmExt, Test,
    TestReport, TxnEmitter, TxnStats, Version,
};
use aptos_logger::info;
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{transaction_builder::TransactionFactory, types::PeerId};
use futures::future::join_all;
use rand::{rngs::StdRng, SeedableRng};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;

const WARMUP_DURATION_FRACTION: f32 = 0.07;
const COOLDOWN_DURATION_FRACTION: f32 = 0.04;

async fn batch_update(
    ctx: &mut NetworkContext<'_>,
    validators_to_update: &[PeerId],
    version: &Version,
) -> Result<()> {
    for validator in validators_to_update {
        ctx.swarm().upgrade_validator(*validator, version).await?;
    }

    ctx.swarm().health_check().await?;
    let deadline = Instant::now() + Duration::from_secs(60);
    for validator in validators_to_update {
        ctx.swarm()
            .validator_mut(*validator)
            .unwrap()
            .wait_until_healthy(deadline)
            .await?;
    }

    Ok(())
}

pub fn create_emitter_and_request(
    swarm: &mut dyn Swarm,
    mut emit_job_request: EmitJobRequest,
    nodes: &[PeerId],
    rng: StdRng,
) -> Result<(TxnEmitter, EmitJobRequest)> {
    // as we are loading nodes, use higher client timeout
    let client_timeout = Duration::from_secs(30);

    let chain_info = swarm.chain_info();
    let transaction_factory = TransactionFactory::new(chain_info.chain_id);
    let emitter = TxnEmitter::new(transaction_factory, rng);

    emit_job_request =
        emit_job_request.rest_clients(swarm.get_clients_for_peers(nodes, client_timeout));
    Ok((emitter, emit_job_request))
}

pub fn traffic_emitter_runtime() -> Result<Runtime> {
    let runtime = aptos_runtimes::spawn_named_runtime("emitter".into(), Some(64));
    Ok(runtime)
}

pub fn generate_traffic(
    ctx: &mut NetworkContext<'_>,
    nodes: &[PeerId],
    duration: Duration,
) -> Result<TxnStats> {
    let emit_job_request = ctx.emit_job.clone();
    let rng = SeedableRng::from_rng(ctx.core().rng())?;
    let (emitter, emit_job_request) =
        create_emitter_and_request(ctx.swarm(), emit_job_request, nodes, rng)?;

    let rt = traffic_emitter_runtime()?;
    let stats = rt.block_on(emitter.emit_txn_for(
        ctx.swarm().chain_info().root_account,
        emit_job_request,
        duration,
    ))?;

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
    fn get_destination_nodes(self, swarm: &mut dyn Swarm) -> Vec<PeerId> {
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

pub trait NetworkLoadTest: Test {
    fn setup(&self, _ctx: &mut NetworkContext) -> Result<LoadDestination> {
        Ok(LoadDestination::FullnodesOtherwiseValidators)
    }

    // Load is started before this function is called, and stops after this function returns.
    // Expected duration is passed into this function, expecting this function to take that much
    // time to finish. How long this function takes will dictate how long the actual test lasts.
    fn test(
        &self,
        _swarm: &mut dyn Swarm,
        _report: &mut TestReport,
        duration: Duration,
    ) -> Result<()> {
        std::thread::sleep(duration);
        Ok(())
    }

    fn finish(&self, _swarm: &mut dyn Swarm) -> Result<()> {
        Ok(())
    }
}

impl NetworkTest for dyn NetworkLoadTest {
    fn run(&self, ctx: &mut NetworkContext<'_>) -> Result<()> {
        let runtime = Runtime::new().unwrap();
        let start_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        let (start_version, _) = runtime
            .block_on(ctx.swarm().get_client_with_newest_ledger_version())
            .context("no clients replied for start version")?;
        let emit_job_request = ctx.emit_job.clone();
        let rng = SeedableRng::from_rng(ctx.core().rng())?;
        let duration = ctx.global_duration;
        let stats_by_phase = self.network_load_test(
            ctx,
            emit_job_request,
            duration,
            WARMUP_DURATION_FRACTION,
            COOLDOWN_DURATION_FRACTION,
            rng,
        )?;

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
                        let slice_samples = phase_stats.latency_breakdown.get_samples(&slice);
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
        let (end_version, _) = runtime
            .block_on(ctx.swarm().get_client_with_newest_ledger_version())
            .context("no clients replied for end version")?;

        self.finish(ctx.swarm())
            .context("finish NetworkLoadTest ")?;

        for (_phase, phase_stats) in stats_by_phase.into_iter().enumerate() {
            ctx.check_for_success(
                &phase_stats.emitter_stats,
                phase_stats.actual_duration,
                &phase_stats.latency_breakdown,
                start_timestamp as i64,
                end_timestamp as i64,
                start_version,
                end_version,
            )
            .context("check for success")?;
        }

        Ok(())
    }
}

impl dyn NetworkLoadTest {
    pub fn network_load_test(
        &self,
        ctx: &mut NetworkContext,
        emit_job_request: EmitJobRequest,
        duration: Duration,
        warmup_duration_fraction: f32,
        cooldown_duration_fraction: f32,
        rng: StdRng,
    ) -> Result<Vec<LoadTestPhaseStats>> {
        let destination = self.setup(ctx).context("setup NetworkLoadTest")?;
        let nodes_to_send_load_to = destination.get_destination_nodes(ctx.swarm());

        // Generate some traffic

        let (mut emitter, emit_job_request) =
            create_emitter_and_request(ctx.swarm(), emit_job_request, &nodes_to_send_load_to, rng)
                .context("create emitter")?;

        let rt = traffic_emitter_runtime()?;
        let clients = ctx
            .swarm()
            .get_clients_for_peers(&nodes_to_send_load_to, Duration::from_secs(10));

        let mut stats_tracking_phases = emit_job_request.get_num_phases();
        assert!(stats_tracking_phases > 0 && stats_tracking_phases != 2);
        if stats_tracking_phases == 1 {
            stats_tracking_phases = 3;
        }

        info!("Starting emitting txns for {}s", duration.as_secs());
        let mut job = rt
            .block_on(emitter.start_job(
                ctx.swarm().chain_info().root_account,
                emit_job_request,
                stats_tracking_phases,
            ))
            .context("start emitter job")?;

        let total_start = PhaseTimingStart::now();

        let warmup_duration = duration.mul_f32(warmup_duration_fraction);
        let cooldown_duration = duration.mul_f32(cooldown_duration_fraction);
        let test_duration = duration - warmup_duration - cooldown_duration;
        let phase_duration = test_duration.div_f32((stats_tracking_phases - 2) as f32);

        job = rt.block_on(job.periodic_stat_forward(warmup_duration, 60));
        info!("{}s warmup finished", warmup_duration.as_secs());

        let mut phase_timing = Vec::new();
        let mut phase_start_network_state = Vec::new();
        let test_start = Instant::now();
        for i in 0..stats_tracking_phases - 2 {
            phase_start_network_state.push(rt.block_on(NetworkState::new(&clients)));
            job.start_next_phase();

            if i > 0 {
                info!(
                    "Starting test phase {} out of {}",
                    i,
                    stats_tracking_phases - 2,
                );
            }
            let phase_start = PhaseTimingStart::now();

            let join_stats = rt.spawn(job.periodic_stat_forward(phase_duration, 60));
            self.test(ctx.swarm, ctx.report, phase_duration)
                .context("test NetworkLoadTest")?;
            job = rt.block_on(join_stats).context("join stats")?;
            phase_timing.push(phase_start.elapsed());
        }
        let actual_test_duration = test_start.elapsed();
        info!(
            "{}s test finished after {}s",
            test_duration.as_secs(),
            actual_test_duration.as_secs()
        );

        phase_start_network_state.push(rt.block_on(NetworkState::new(&clients)));
        job.start_next_phase();
        let cooldown_start = Instant::now();

        let cooldown_used = cooldown_start.elapsed();
        if cooldown_used < cooldown_duration {
            job = rt.block_on(job.periodic_stat_forward(cooldown_duration - cooldown_used, 60));
        }
        info!("{}s cooldown finished", cooldown_duration.as_secs());

        let total_timing = total_start.elapsed();
        info!(
            "Emitting txns ran for {} secs(from {} to {}), stopping job...",
            duration.as_secs(),
            total_timing.start_unixtime_s,
            total_timing.end_unixtime_s,
        );
        let stats_by_phase = rt.block_on(job.stop_job());

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
            let latency_breakdown = rt.block_on(fetch_latency_breakdown(
                ctx.swarm(),
                phase_timing[i].start_unixtime_s,
                phase_timing[i].end_unixtime_s,
            ))?;
            info!(
                "Latency breakdown details for phase {}: from {} to {}: {:?}",
                i,
                phase_timing[i].start_unixtime_s,
                phase_timing[i].end_unixtime_s,
                latency_breakdown
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

impl NetworkTest for CompositeNetworkTest {
    fn run(&self, ctx: &mut NetworkContext<'_>) -> anyhow::Result<()> {
        for wrapper in &self.wrappers {
            wrapper.setup(ctx)?;
        }
        self.test.run(ctx)?;
        for wrapper in &self.wrappers {
            wrapper.finish(ctx.swarm())?;
        }
        Ok(())
    }
}

impl Test for CompositeNetworkTest {
    fn name(&self) -> &'static str {
        "CompositeNetworkTest"
    }
}
