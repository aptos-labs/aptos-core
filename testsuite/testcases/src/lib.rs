// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub mod compatibility_test;
pub mod consensus_reliability_tests;
pub mod forge_setup_test;
pub mod fullnode_reboot_stress_test;
pub mod gas_price_test;
pub mod load_vs_perf_benchmark;
pub mod network_bandwidth_test;
pub mod network_loss_test;
pub mod network_partition_test;
pub mod partial_nodes_down_test;
pub mod performance_test;
pub mod performance_with_fullnode_test;
pub mod reconfiguration_test;
pub mod state_sync_performance;
pub mod three_region_simulation_test;
pub mod twin_validator_test;
pub mod validator_reboot_stress_test;

use anyhow::{anyhow, ensure};
use aptos_logger::info;
use aptos_sdk::{transaction_builder::TransactionFactory, types::PeerId};
use forge::{
    EmitJobRequest, NetworkContext, NetworkTest, NodeExt, Result, Swarm, SwarmExt, Test,
    TxnEmitter, TxnStats, Version,
};
use futures::future::join_all;
use rand::{rngs::StdRng, SeedableRng};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::runtime::Builder;

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
    gas_price: u64,
    rng: StdRng,
) -> Result<(TxnEmitter, EmitJobRequest)> {
    ensure!(gas_price > 0, "gas_price is required to be non zero");

    // as we are loading nodes, use higher client timeout
    let client_timeout = Duration::from_secs(30);

    let chain_info = swarm.chain_info();
    let transaction_factory = TransactionFactory::new(chain_info.chain_id)
        .with_gas_unit_price(aptos_global_constants::GAS_UNIT_PRICE);
    let emitter = TxnEmitter::new(transaction_factory, rng);

    emit_job_request = emit_job_request
        .rest_clients(swarm.get_clients_for_peers(nodes, client_timeout))
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
    fn setup(&self, _ctx: &mut NetworkContext) -> Result<LoadDestination> {
        Ok(LoadDestination::AllNodes)
    }
    // Load is started before this funciton is called, and stops after this function returns.
    // Expected duration is passed into this function, expecting this function to take that much
    // time to finish. How long this function takes will dictate how long the actual test lasts.
    fn test(&self, _swarm: &mut dyn Swarm, duration: Duration) -> Result<()> {
        std::thread::sleep(duration);
        Ok(())
    }

    fn finish(&self, _swarm: &mut dyn Swarm) -> Result<()> {
        Ok(())
    }
}

impl NetworkTest for dyn NetworkLoadTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        let start_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        let one_client = ctx.swarm().aptos_public_info().client().clone();
        let start_version = ctx
            .runtime
            .block_on(one_client.get_ledger_information())?
            .into_inner()
            .version;
        let emit_job_request = ctx.emit_job.clone();
        let rng = SeedableRng::from_rng(ctx.core().rng())?;
        let duration = ctx.global_duration;
        let (txn_stat, actual_test_duration, _ledger_transactions) = self.network_load_test(
            ctx,
            emit_job_request,
            duration,
            WARMUP_DURATION_FRACTION,
            COOLDOWN_DURATION_FRACTION,
            rng,
        )?;
        ctx.report
            .report_txn_stats(self.name().to_string(), &txn_stat, actual_test_duration);

        let end_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        let end_version = ctx
            .runtime
            .block_on(one_client.get_ledger_information())?
            .into_inner()
            .version;

        self.finish(ctx.swarm())?;

        ctx.check_for_success(
            &txn_stat,
            &actual_test_duration,
            start_timestamp as i64,
            end_timestamp as i64,
            start_version,
            end_version,
        )?;

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
    ) -> Result<(TxnStats, Duration, u64)> {
        let all_validators = ctx
            .swarm()
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();

        let all_fullnodes = ctx
            .swarm()
            .full_nodes()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();

        let nodes_to_send_load_to = match self.setup(ctx)? {
            LoadDestination::AllNodes => [&all_validators[..], &all_fullnodes[..]].concat(),
            LoadDestination::AllValidators => all_validators,
            LoadDestination::AllFullnodes => all_fullnodes,
            LoadDestination::Peers(peers) => peers,
        };

        // Generate some traffic

        let (mut emitter, emit_job_request) = create_emitter_and_request(
            ctx.swarm(),
            emit_job_request,
            &nodes_to_send_load_to,
            aptos_global_constants::GAS_UNIT_PRICE,
            rng,
        )?;

        let mut runtime_builder = Builder::new_multi_thread();
        runtime_builder.disable_lifo_slot().enable_all();
        runtime_builder.worker_threads(64);
        let rt = runtime_builder
            .build()
            .map_err(|err| anyhow!("Failed to start runtime for transaction emitter. {}", err))?;

        let clients = ctx
            .swarm()
            .get_clients_for_peers(&nodes_to_send_load_to, Duration::from_secs(10));

        // Read first
        for client in &clients {
            let start = Instant::now();
            let _v = rt.block_on(client.get_ledger_information())?;
            let duration = start.elapsed();
            info!(
                "Fetch from {:?} took {}ms",
                client.path_prefix_string(),
                duration.as_millis(),
            );
        }

        let job = rt.block_on(emitter.start_job(
            ctx.swarm().chain_info().root_account,
            emit_job_request,
            3,
        ))?;

        let warmup_duration = duration.mul_f32(warmup_duration_fraction);
        let cooldown_duration = duration.mul_f32(cooldown_duration_fraction);
        let test_duration = duration - warmup_duration - cooldown_duration;
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

        job.start_next_phase();

        let test_start = Instant::now();
        self.test(ctx.swarm(), test_duration)?;
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
        let txn_stats = rt.block_on(emitter.stop_job(job));

        info!("Stopped job");
        info!("Warmup stats: {}", txn_stats[0].rate(warmup_duration));
        info!("Test stats: {}", txn_stats[1].rate(actual_test_duration));
        info!("Cooldown stats: {}", txn_stats[2].rate(cooldown_duration));

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
            txn_stats.into_iter().nth(1).unwrap(),
            actual_test_duration,
            ledger_transactions,
        ))
    }
}
