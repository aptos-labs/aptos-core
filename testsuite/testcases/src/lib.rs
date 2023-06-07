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
    EmitJobRequest, NetworkContext, NetworkTest, NodeExt, Result, Swarm, SwarmExt, Test,
    TestReport, TxnEmitter, TxnStats, Version,
};
use aptos_logger::info;
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
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
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
        let (txn_stat, actual_test_duration, _ledger_transactions, _stats_by_phase) = self
            .network_load_test(
                ctx,
                emit_job_request,
                duration,
                WARMUP_DURATION_FRACTION,
                COOLDOWN_DURATION_FRACTION,
                rng,
            )?;
        ctx.report
            .report_txn_stats(self.name().to_string(), &txn_stat);

        let end_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        let (end_version, _) = runtime
            .block_on(ctx.swarm().get_client_with_newest_ledger_version())
            .context("no clients replied for end version")?;

        self.finish(ctx.swarm())
            .context("finish NetworkLoadTest ")?;

        ctx.check_for_success(
            &txn_stat,
            actual_test_duration,
            start_timestamp as i64,
            end_timestamp as i64,
            start_version,
            end_version,
        )
        .context("check for success")?;

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
    ) -> Result<(TxnStats, Duration, u64, Vec<(TxnStats, Duration)>)> {
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

        let mut job = rt
            .block_on(emitter.start_job(
                ctx.swarm().chain_info().root_account,
                emit_job_request,
                stats_tracking_phases,
            ))
            .context("start emitter job")?;

        let warmup_duration = duration.mul_f32(warmup_duration_fraction);
        let cooldown_duration = duration.mul_f32(cooldown_duration_fraction);
        let test_duration = duration - warmup_duration - cooldown_duration;
        let phase_duration = test_duration.div_f32((stats_tracking_phases - 2) as f32);
        info!("Starting emitting txns for {}s", duration.as_secs());

        std::thread::sleep(warmup_duration);
        info!("{}s warmup finished", warmup_duration.as_secs());

        let max_start_ledger_transactions = rt
            .block_on(join_all(
                clients.iter().map(|client| client.get_ledger_information()),
            ))
            .into_iter()
            .filter(|r| r.is_ok())
            .map(|r| r.unwrap().into_inner())
            .map(|s| s.version - 2 * s.block_height)
            .max();

        let mut actual_phase_durations = Vec::new();
        let test_start = Instant::now();
        for i in 0..stats_tracking_phases - 2 {
            job.start_next_phase();

            if i > 0 {
                info!(
                    "Starting test phase {} out of {}",
                    i,
                    stats_tracking_phases - 2,
                );
            }
            let phase_start = Instant::now();

            self.test(ctx.swarm, ctx.report, phase_duration)
                .context("test NetworkLoadTest")?;
            actual_phase_durations.push(phase_start.elapsed());
        }
        let actual_test_duration = test_start.elapsed();
        info!(
            "{}s test finished after {}s",
            test_duration.as_secs(),
            actual_test_duration.as_secs()
        );

        job.start_next_phase();
        let cooldown_start = Instant::now();
        let max_end_ledger_transactions = rt
            .block_on(join_all(
                clients.iter().map(|client| client.get_ledger_information()),
            ))
            .into_iter()
            .filter(|r| r.is_ok())
            .map(|r| r.unwrap().into_inner())
            .map(|s| s.version - 2 * s.block_height)
            .max();

        let cooldown_used = cooldown_start.elapsed();
        if cooldown_used < cooldown_duration {
            std::thread::sleep(cooldown_duration - cooldown_used);
        }
        info!("{}s cooldown finished", cooldown_duration.as_secs());

        info!(
            "Emitting txns ran for {} secs, stopping job...",
            duration.as_secs()
        );
        let stats_by_phase = rt.block_on(emitter.stop_job(job));

        info!("Stopped job");
        info!("Warmup stats: {}", stats_by_phase[0].rate());

        let mut stats: Option<TxnStats> = None;
        let mut stats_and_duration_by_phase_filtered = Vec::new();
        for i in 0..stats_tracking_phases - 2 {
            let cur = &stats_by_phase[1 + i];
            info!("Test stats [test phase {}]: {}", i, cur.rate());
            stats = if let Some(previous) = stats {
                Some(&previous + cur)
            } else {
                Some(cur.clone())
            };
            stats_and_duration_by_phase_filtered.push((cur.clone(), actual_phase_durations[i]));
        }
        info!("Cooldown stats: {}", stats_by_phase.last().unwrap().rate());

        let ledger_transactions = if let Some(end_t) = max_end_ledger_transactions {
            if let Some(start_t) = max_start_ledger_transactions {
                end_t - start_t
            } else {
                0
            }
        } else {
            0
        };

        Ok((
            stats.unwrap(),
            actual_test_duration,
            ledger_transactions,
            stats_and_duration_by_phase_filtered,
        ))
    }
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
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> anyhow::Result<()> {
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
