// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub mod compatibility_test;
pub mod continuous_progress_test;
pub mod forge_setup_test;
pub mod gas_price_test;
pub mod network_bandwidth_test;
pub mod network_latency_test;
pub mod network_loss_test;
pub mod network_partition_test;
pub mod partial_nodes_down_test;
pub mod performance_test;
pub mod performance_with_fullnode_test;
pub mod reconfiguration_test;
pub mod state_sync_performance;

use anyhow::{anyhow, ensure};
use aptos_logger::info;
use aptos_sdk::{transaction_builder::TransactionFactory, types::PeerId};
use forge::{
    EmitJobRequest, NetworkContext, NetworkTest, NodeExt, Result, Swarm, Test, TxnEmitter,
    TxnStats, Version,
};
use rand::{rngs::StdRng, SeedableRng};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::runtime::Builder;

const WARMUP_DURATION_FRACTION: f32 = 0.05;
const COOLDOWN_DURATION_FRACTION: f32 = 0.05;

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
    gas_price: u64,
    rng: StdRng,
) -> Result<(TxnEmitter, EmitJobRequest)> {
    ensure!(gas_price > 0, "gas_price is required to be non zero");

    // as we are loading nodes, use higher client timeout
    let client_timeout = Duration::from_secs(30);
    let validator_clients = swarm
        .validators()
        .filter(|v| nodes.contains(&v.peer_id()))
        .map(|n| n.rest_client_with_timeout(client_timeout))
        .collect::<Vec<_>>();
    let fullnode_clients = swarm
        .full_nodes()
        .filter(|v| nodes.contains(&v.peer_id()))
        .map(|n| n.rest_client_with_timeout(client_timeout))
        .collect::<Vec<_>>();
    let all_node_clients = [&fullnode_clients[..], &validator_clients[..]].concat();

    let chain_info = swarm.chain_info();
    let transaction_factory = TransactionFactory::new(chain_info.chain_id).with_gas_unit_price(1);
    let emitter = TxnEmitter::new(transaction_factory, rng);

    emit_job_request = emit_job_request
        .rest_clients(all_node_clients)
        .gas_price(gas_price);
    Ok((emitter, emit_job_request))
}

pub fn generate_traffic(
    ctx: &mut NetworkContext<'_>,
    nodes: &[PeerId],
    duration: Duration,
    gas_price: u64,
) -> Result<TxnStats> {
    let emit_job_request = ctx.emit_job.clone();
    let rng = SeedableRng::from_rng(ctx.core().rng())?;
    let (mut emitter, emit_job_request) =
        create_emitter_and_request(ctx.swarm(), emit_job_request, nodes, gas_price, rng)?;

    let mut runtime_builder = Builder::new_multi_thread();
    runtime_builder.disable_lifo_slot().enable_all();
    runtime_builder.worker_threads(64);
    let rt = runtime_builder
        .build()
        .map_err(|err| anyhow!("Failed to start runtime for transaction emitter. {}", err))?;
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
    Peers(Vec<PeerId>),
}

pub trait NetworkLoadTest: Test {
    fn setup(&self, _swarm: &mut dyn Swarm) -> Result<LoadDestination> {
        Ok(LoadDestination::AllNodes)
    }
    // Load is started before this funciton is called, and stops after this function returns.
    // Expected duration is passed into this function, expecting this function to take that much
    // time to finish. How long this function takes will dictate how long the actual test lasts.
    fn test(&self, _swarm: &mut dyn Swarm, duration: Duration) -> Result<()> {
        std::thread::sleep(duration);
        Ok(())
    }
    fn finish(&self, _swarm: &mut dyn Swarm, _start_time: u64, _end_time: u64) -> Result<()> {
        Ok(())
    }
}

impl NetworkTest for dyn NetworkLoadTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        let start_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        let emit_job_request = ctx.emit_job.clone();
        let rng = SeedableRng::from_rng(ctx.core().rng())?;
        let duration = ctx.global_duration.clone();
        let (txn_stat, actual_test_duration) =
            self.network_load_test(ctx.swarm(), emit_job_request, duration, rng)?;
        ctx.report
            .report_txn_stats(self.name().to_string(), &txn_stat, actual_test_duration);

        ctx.check_for_success(&txn_stat, &actual_test_duration)?;

        let end_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        ctx.check_for_success(&txn_stat, &actual_test_duration)?;

        self.finish(ctx.swarm(), start_timestamp, end_timestamp)?;

        Ok(())
    }
}

impl dyn NetworkLoadTest {
    pub fn network_load_test(
        &self,
        swarm: &mut dyn Swarm,
        emit_job_request: EmitJobRequest,
        duration: Duration,
        rng: StdRng,
    ) -> Result<(TxnStats, Duration)> {
        let all_validators = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();

        let all_fullnodes = swarm.full_nodes().map(|v| v.peer_id()).collect::<Vec<_>>();

        let nodes_to_send_load_to = match self.setup(swarm)? {
            LoadDestination::AllNodes => [&all_validators[..], &all_fullnodes[..]].concat(),
            LoadDestination::AllValidators => all_validators,
            LoadDestination::AllFullnodes => all_fullnodes,
            LoadDestination::Peers(peers) => peers,
        };

        // Generate some traffic

        let (mut emitter, emit_job_request) =
            create_emitter_and_request(swarm, emit_job_request, &nodes_to_send_load_to, 1, rng)?;

        let mut runtime_builder = Builder::new_multi_thread();
        runtime_builder.disable_lifo_slot().enable_all();
        runtime_builder.worker_threads(64);
        let rt = runtime_builder
            .build()
            .map_err(|err| anyhow!("Failed to start runtime for transaction emitter. {}", err))?;

        let job =
            rt.block_on(emitter.start_job(swarm.chain_info().root_account, emit_job_request, 3))?;

        let warmup_duration = duration.mul_f32(WARMUP_DURATION_FRACTION);
        let cooldown_duration = duration.mul_f32(COOLDOWN_DURATION_FRACTION);
        let test_duration = duration - warmup_duration - cooldown_duration;
        info!("Starting emitting txns for {}s", duration.as_secs());

        std::thread::sleep(warmup_duration);
        info!("{}s warmup finished", warmup_duration.as_secs());

        job.start_next_phase();

        let test_start = Instant::now();
        self.test(swarm, test_duration)?;
        let actual_test_duration = test_start.elapsed();
        info!(
            "{}s test finished after {}s",
            test_duration.as_secs(),
            actual_test_duration.as_secs()
        );

        job.start_next_phase();

        std::thread::sleep(cooldown_duration);
        info!("{}s cooldown finished", cooldown_duration.as_secs());

        info!(
            "Emitting txns ran for {} secs, stopping job...",
            duration.as_secs()
        );
        let txn_stats = rt.block_on(emitter.stop_job(job));

        info!("Stopped job");
        info!("Warmup stats: {}", txn_stats[0].rate(warmup_duration));
        info!("Test stats: {}", txn_stats[1].rate(actual_test_duration));
        info!("Cooldown stats: {}", txn_stats[2].rate(cooldown_duration));

        Ok((
            txn_stats.into_iter().skip(1).next().unwrap(),
            actual_test_duration,
        ))
    }
}
