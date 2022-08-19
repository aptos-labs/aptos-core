// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::new_local_swarm_with_aptos;
use anyhow::ensure;
use aptos_sdk::{transaction_builder::TransactionFactory, types::PeerId};
use forge::{EmitJobRequest, NodeExt, Result, Swarm, TxnEmitter, TxnStats};
use rand::{rngs::OsRng, SeedableRng};
use std::time::Duration;
use tokio::runtime::Builder;

pub async fn generate_traffic(
    swarm: &mut dyn Swarm,
    nodes: &[PeerId],
    duration: Duration,
    gas_price: u64,
) -> Result<TxnStats> {
    ensure!(gas_price > 0, "gas_price is required to be non zero");
    let mut runtime_builder = Builder::new_multi_thread();
    runtime_builder.enable_all();
    runtime_builder.worker_threads(64);
    let rng = SeedableRng::from_rng(OsRng)?;
    let validator_clients = swarm
        .validators()
        .filter(|v| nodes.contains(&v.peer_id()))
        .map(|n| n.rest_client())
        .collect::<Vec<_>>();
    let fullnode_clients = swarm
        .full_nodes()
        .filter(|v| nodes.contains(&v.peer_id()))
        .map(|n| n.rest_client())
        .collect::<Vec<_>>();
    let all_node_clients = [&fullnode_clients[..], &validator_clients[..]].concat();

    let mut emit_job_request = EmitJobRequest::default();
    let chain_info = swarm.chain_info();
    let transaction_factory = TransactionFactory::new(chain_info.chain_id).with_gas_unit_price(1);
    let mut emitter = TxnEmitter::new(
        chain_info.root_account,
        // TODO: swap this with a random client
        all_node_clients[0].clone(),
        transaction_factory,
        rng,
    );

    emit_job_request = emit_job_request
        .rest_clients(all_node_clients)
        .gas_price(gas_price)
        .duration(duration);
    let stats = emitter.emit_txn_for(emit_job_request).await?;

    Ok(stats)
}

#[tokio::test]
async fn test_txn_emmitter() {
    let mut swarm = new_local_swarm_with_aptos(1).await;

    // emit to all validator
    let all_validators = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();

    let txn_stat = generate_traffic(&mut swarm, &all_validators, Duration::from_secs(10), 1)
        .await
        .unwrap();
    println!("{:?}", txn_stat);
    // assert some much smaller number than expected, so it doesn't fail under contention
    assert!(txn_stat.submitted > 100);
    assert!(txn_stat.committed > 100);
}
