// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

pub mod compatibility_test;
pub mod fixed_tps_test;
pub mod gas_price_test;
pub mod partial_nodes_down_test;
pub mod performance_test;
pub mod reconfiguration_test;
pub mod state_sync_performance;

use diem_sdk::{transaction_builder::TransactionFactory, types::PeerId};
use forge::{EmitJobRequest, NetworkContext, NodeExt, Result, TxnEmitter, TxnStats, Version};
use rand::SeedableRng;
use std::{
    convert::TryInto,
    time::{Duration, Instant},
};
use tokio::runtime::Runtime;

fn batch_update<'t>(
    ctx: &mut NetworkContext<'t>,
    validators_to_update: &[PeerId],
    version: &Version,
) -> Result<()> {
    for validator in validators_to_update {
        ctx.swarm().upgrade_validator(*validator, version)?;
    }

    ctx.swarm().health_check()?;
    let deadline = Instant::now() + Duration::from_secs(60);
    for validator in validators_to_update {
        ctx.swarm()
            .validator_mut(*validator)
            .unwrap()
            .wait_until_healthy(deadline)?;
    }

    Ok(())
}

pub fn generate_traffic<'t>(
    ctx: &mut NetworkContext<'t>,
    validators: &[PeerId],
    duration: Duration,
    gas_price: u64,
    fixed_tps: Option<u64>,
) -> Result<TxnStats> {
    let rt = Runtime::new()?;
    let rng = SeedableRng::from_rng(ctx.core().rng())?;
    let validator_clients = ctx
        .swarm()
        .validators()
        .filter(|v| validators.contains(&v.peer_id()))
        .map(|n| n.async_json_rpc_client())
        .collect::<Vec<_>>();
    let chain_info = ctx.swarm().chain_info();
    let transaction_factory = TransactionFactory::new(chain_info.chain_id);
    let mut emitter = TxnEmitter::new(
        chain_info.treasury_compliance_account,
        chain_info.designated_dealer_account,
        validator_clients[0].clone(),
        transaction_factory,
        rng,
    );
    let mut emit_job_request = EmitJobRequest::new(validator_clients).gas_price(gas_price);
    if let Some(target_tps) = fixed_tps {
        emit_job_request = emit_job_request.fixed_tps(target_tps.try_into().unwrap());
    }
    let stats = rt.block_on(emitter.emit_txn_for(duration, emit_job_request))?;

    Ok(stats)
}
