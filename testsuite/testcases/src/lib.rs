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
use rand::SeedableRng;
use std::time::{Duration, Instant};
use tokio::runtime::Builder;

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
    ctx: &mut NetworkContext<'_>,
    nodes: &[PeerId],
    gas_price: u64,
) -> Result<(TxnEmitter, EmitJobRequest)> {
    ensure!(gas_price > 0, "gas_price is required to be non zero");
    let rng = SeedableRng::from_rng(ctx.core().rng())?;

    // as we are loading nodes, use higher client timeout
    let client_timeout = Duration::from_secs(30);
    let validator_clients = ctx
        .swarm()
        .validators()
        .filter(|v| nodes.contains(&v.peer_id()))
        .map(|n| n.rest_client_with_timeout(client_timeout))
        .collect::<Vec<_>>();
    let fullnode_clients = ctx
        .swarm()
        .full_nodes()
        .filter(|v| nodes.contains(&v.peer_id()))
        .map(|n| n.rest_client_with_timeout(client_timeout))
        .collect::<Vec<_>>();
    let all_node_clients = [&fullnode_clients[..], &validator_clients[..]].concat();

    let mut emit_job_request = ctx.emit_job.clone();
    let chain_info = ctx.swarm().chain_info();
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
    let (mut emitter, emit_job_request) = create_emitter_and_request(ctx, nodes, gas_price)?;

    let mut runtime_builder = Builder::new_multi_thread();
    runtime_builder.enable_all();
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
    fn finish(&self, _swarm: &mut dyn Swarm) -> Result<()> {
        Ok(())
    }
}

impl NetworkTest for dyn NetworkLoadTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        let duration = ctx.global_duration;

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

        let nodes_to_send_load_to = match self.setup(ctx.swarm())? {
            LoadDestination::AllNodes => [&all_validators[..], &all_fullnodes[..]].concat(),
            LoadDestination::AllValidators => all_validators,
            LoadDestination::AllFullnodes => all_fullnodes,
            LoadDestination::Peers(peers) => peers,
        };

        // Generate some traffic

        let (mut emitter, emit_job_request) =
            create_emitter_and_request(ctx, &nodes_to_send_load_to, 1)?;

        let mut runtime_builder = Builder::new_multi_thread();
        runtime_builder.enable_all();
        runtime_builder.worker_threads(64);
        let rt = runtime_builder
            .build()
            .map_err(|err| anyhow!("Failed to start runtime for transaction emitter. {}", err))?;

        let job = rt
            .block_on(emitter.start_job(ctx.swarm().chain_info().root_account, emit_job_request))?;
        info!("Starting emitting txns for {} secs", duration.as_secs());

        self.test(ctx.swarm(), duration)?;

        info!("Ran for {} secs, stopping job...", duration.as_secs());
        let txn_stat = rt.block_on(emitter.stop_job(job));
        info!("Stopped job");

        ctx.report
            .report_txn_stats(self.name().to_string(), &txn_stat, duration);

        ctx.check_for_success(&txn_stat, &duration)?;

        self.finish(ctx.swarm())?;

        Ok(())
    }
}
