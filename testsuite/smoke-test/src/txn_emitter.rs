// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::new_local_swarm_with_aptos;
use anyhow::ensure;
use aptos_sdk::{transaction_builder::TransactionFactory, types::PeerId};
use forge::{
    EmitJobMode, EmitJobRequest, NodeExt, Result, Swarm, TransactionType, TxnEmitter, TxnStats,
};
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
    runtime_builder.disable_lifo_slot().enable_all();
    runtime_builder.worker_threads(64);
    let rng = SeedableRng::from_rng(OsRng)?;
    let validator_clients = swarm
        .validators()
        .filter(|v| nodes.contains(&v.peer_id()))
        .map(|n| n.rest_client())
        .collect::<Vec<_>>();
    let mut emit_job_request = EmitJobRequest::default();
    let chain_info = swarm.chain_info();
    let transaction_factory = TransactionFactory::new(chain_info.chain_id).with_gas_unit_price(1);
    let mut emitter = TxnEmitter::new(transaction_factory, rng);

    emit_job_request = emit_job_request
        .rest_clients(validator_clients)
        .gas_price(gas_price)
        .transaction_mix(vec![
            (TransactionType::P2P, 70),
            (TransactionType::AccountGeneration, 20),
            (TransactionType::NftMint, 10),
        ])
        .mode(EmitJobMode::ConstTps { tps: 20 });
    emitter
        .emit_txn_for_with_stats(chain_info.root_account, emit_job_request, duration, 3)
        .await
}

#[ignore]
#[tokio::test]
async fn test_txn_emmitter() {
    let mut swarm = new_local_swarm_with_aptos(1).await;

    let all_validators = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();

    let txn_stat = generate_traffic(&mut swarm, &all_validators, Duration::from_secs(10), 1)
        .await
        .unwrap();
    println!("{:?}", txn_stat.rate(Duration::from_secs(10)));
    // assert some much smaller number than expected, so it doesn't fail under contention
    assert!(txn_stat.submitted > 30);
    assert!(txn_stat.committed > 30);
}
