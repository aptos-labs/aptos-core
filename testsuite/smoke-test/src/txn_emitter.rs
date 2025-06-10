// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    keyless::{remove_training_wheels, spawn_network_and_execute_gov_proposals},
    smoke_test_environment::{new_local_swarm_with_aptos, SwarmBuilder},
    utils::create_and_fund_account,
};
use anyhow::ensure;
use aptos_forge::{
    args::TransactionTypeArg, emitter::NumAccountsMode, AccountType, EmitJobMode, EmitJobRequest,
    EntryPoints, NodeExt, Result, Swarm, TransactionType, TxnEmitter, TxnStats, WorkflowProgress,
};
use aptos_sdk::{transaction_builder::TransactionFactory, types::PeerId};
use aptos_types::keyless::test_utils::{get_sample_esk, get_sample_exp_date, get_sample_jwt_token};
use once_cell::sync::Lazy;
use rand::{rngs::OsRng, SeedableRng};
use std::{sync::Arc, time::Duration};

pub async fn generate_traffic(
    swarm: &mut dyn Swarm,
    nodes: &[PeerId],
    duration: Duration,
    gas_price: u64,
    transaction_mix_per_phase: Vec<Vec<(TransactionType, usize)>>,
) -> Result<TxnStats> {
    ensure!(gas_price > 0, "gas_price is required to be non zero");
    let rng = SeedableRng::from_rng(OsRng)?;

    let emit_job_request = create_emit_job_request(
        swarm,
        nodes,
        gas_price,
        transaction_mix_per_phase,
        AccountType::Local,
    )
    .await?;
    let transaction_factory =
        TransactionFactory::new(swarm.chain_info().chain_id).with_gas_unit_price(gas_price);
    let rest_cli = swarm.validators().next().unwrap().rest_client();
    let emitter = TxnEmitter::new(transaction_factory, rng, rest_cli);
    emitter
        .emit_txn_for_with_stats(
            swarm.chain_info().root_account,
            emit_job_request,
            duration,
            3,
        )
        .await
}

pub async fn generate_keyless_traffic(
    swarm: &mut dyn Swarm,
    nodes: &[PeerId],
    duration: Duration,
    gas_price: u64,
    transaction_mix_per_phase: Vec<Vec<(TransactionType, usize)>>,
) -> Result<TxnStats> {
    ensure!(gas_price > 0, "gas_price is required to be non zero");
    let rng = SeedableRng::from_rng(OsRng)?;

    let emit_job_request = create_emit_job_request(
        swarm,
        nodes,
        gas_price,
        transaction_mix_per_phase,
        AccountType::Keyless,
    )
    .await?;
    let transaction_factory =
        TransactionFactory::new(swarm.chain_info().chain_id).with_gas_unit_price(gas_price);
    let rest_cli = swarm.validators().next().unwrap().rest_client();
    let emitter = TxnEmitter::new(transaction_factory, rng, rest_cli);
    emitter
        .emit_txn_for_with_stats(
            swarm.chain_info().root_account,
            emit_job_request,
            duration,
            3,
        )
        .await
}

pub async fn create_emit_job_request(
    swarm: &mut dyn Swarm,
    nodes: &[PeerId],
    gas_price: u64,
    transaction_mix_per_phase: Vec<Vec<(TransactionType, usize)>>,
    account_type: AccountType,
) -> Result<EmitJobRequest> {
    ensure!(gas_price > 0, "gas_price is required to be non zero");

    let validator_clients = swarm
        .validators()
        .filter(|v| nodes.contains(&v.peer_id()))
        .map(|n| n.rest_client())
        .collect::<Vec<_>>();
    let mut emit_job_request = EmitJobRequest::default();
    emit_job_request = emit_job_request
        .rest_clients(validator_clients)
        .gas_price(gas_price)
        .expected_gas_per_txn(1000000)
        .max_gas_per_txn(2000000)
        .coordination_delay_between_instances(Duration::from_secs(1))
        .transaction_mix_per_phase(transaction_mix_per_phase)
        .mode(EmitJobMode::ConstTps { tps: 20 })
        .account_type(account_type);

    if let AccountType::Keyless = account_type {
        emit_job_request = emit_job_request
            .keyless_ephem_secret_key(get_sample_esk())
            .epk_expiry_date_secs(get_sample_exp_date())
            .keyless_jwt(&get_sample_jwt_token());
    }
    Ok(emit_job_request)
}

static TRANSACTION_MIX_PER_PHASE: Lazy<Vec<Vec<(TransactionType, usize)>>> = Lazy::new(|| {
    vec![
        // vec![(
        //     TransactionType::AccountGeneration {
        //         add_created_accounts_to_pool: true,
        //         max_account_working_set: 1_000_000,
        //         creation_balance: 1_000_000,
        //     },
        //     20,
        // )],
        // vec![
        //     (TransactionTypeArg::CoinTransfer.materialize_default(), 20),
        //     // // commenting this out given it consistently fails smoke test
        //     // // and it seems to be called only from `test_txn_emmitter`
        //     (
        //         TransactionType::PublishPackage {
        //             use_account_pool: false,
        //         },
        //         20,
        //     ),
        // ],
        vec![
            (
                TransactionTypeArg::NoOp.materialize(
                    100,
                    false,
                    WorkflowProgress::when_done_default(),
                ),
                20,
            ),
            (
                TransactionType::CallCustomModules {
                    entry_point: Box::new(EntryPoints::MakeOrChangeTable {
                        offset: 0,
                        count: 60,
                    }),
                    num_modules: 1,
                    use_account_pool: false,
                },
                20,
            ),
        ],
    ]
});

// TODO: Run this smoke test with the new workflow

#[ignore]
#[tokio::test]
async fn test_txn_emmitter() {
    let mut swarm = new_local_swarm_with_aptos(1).await;

    let all_validators = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();

    let txn_stat = generate_traffic(
        &mut swarm,
        &all_validators,
        Duration::from_secs(20),
        100,
        TRANSACTION_MIX_PER_PHASE.to_vec(),
    )
    .await
    .unwrap();
    println!("{:?}", txn_stat.rate());
    // assert some much smaller number than expected, so it doesn't fail under contention
    assert!(txn_stat.submitted > 30);
    assert!(txn_stat.committed > 30);
}

#[ignore]
#[tokio::test]
async fn test_keyless_txn_emmitter() {
    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(1)
        .with_aptos()
        .build_with_cli(0)
        .await;

    let (_tw_sk, _config, _jwk, root_idx) =
        spawn_network_and_execute_gov_proposals(&mut swarm, &mut cli).await;

    remove_training_wheels(&mut cli, &mut swarm.aptos_public_info(), root_idx).await;

    let all_validators = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();

    let txn_stat = generate_keyless_traffic(
        &mut swarm,
        &all_validators,
        Duration::from_secs(20),
        100,
        TRANSACTION_MIX_PER_PHASE.to_vec(),
    )
    .await
    .unwrap();
    println!("{:?}", txn_stat.rate());
    // assert some much smaller number than expected, so it doesn't fail under contention
    assert!(txn_stat.submitted > 30);
    assert!(txn_stat.committed > 30);
}

#[tokio::test]
async fn test_txn_emmitter_with_high_pending_latency() {
    let mut swarm = SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_config(Arc::new(|_, conf, _| {
            conf.api.failpoints_enabled = true;
            conf.consensus.pipeline_backpressure.truncate(1);
            conf.consensus.pipeline_backpressure[0]
                .max_sending_block_txns_after_filtering_override = 2;
            conf.consensus.pipeline_backpressure[0].back_pressure_pipeline_latency_limit_ms = 0;
        }))
        .build()
        .await;

    let all_validators = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();

    let txn_stat = generate_traffic(
        &mut swarm,
        &all_validators,
        Duration::from_secs(20),
        100,
        vec![vec![(
            TransactionType::CallCustomModules {
                entry_point: Box::new(EntryPoints::SmartTablePicture {
                    length: 128 * 1024,
                    num_points_per_txn: 256,
                }),
                num_modules: 1,
                use_account_pool: false,
            },
            1,
        )]],
    )
    .await
    .unwrap();
    assert!(txn_stat.submitted > 30);
}

#[tokio::test]
async fn test_txn_emmitter_low_funds() {
    let mut swarm = new_local_swarm_with_aptos(1).await;
    let account_1 = create_and_fund_account(&mut swarm, 9705100).await;

    let transaction_type = TransactionType::CallCustomModules {
        entry_point: Box::new(EntryPoints::Nop),
        num_modules: 1,
        use_account_pool: false,
    };

    let rng = SeedableRng::from_rng(OsRng).unwrap();
    let validator_clients = swarm
        .validators()
        .map(|n| n.rest_client())
        .collect::<Vec<_>>();
    let chain_info = swarm.chain_info();
    let transaction_factory = TransactionFactory::new(chain_info.chain_id).with_gas_unit_price(100);
    let emitter = TxnEmitter::new(transaction_factory, rng, validator_clients[0].clone());

    let emit_job_request = EmitJobRequest::default()
        .rest_clients(validator_clients)
        .gas_price(100)
        .expected_max_txns(2000)
        .expected_gas_per_txn(5)
        .init_gas_price_multiplier(1)
        .init_max_gas_per_txn(20000)
        .max_gas_per_txn(1000)
        .num_accounts_mode(NumAccountsMode::TransactionsPerAccount(5))
        .transaction_type(transaction_type)
        .mode(EmitJobMode::MaxLoad {
            mempool_backlog: 10,
        });

    let account_1 = Arc::new(account_1);
    let txn_stat = emitter
        .emit_txn_for_with_stats(account_1, emit_job_request, Duration::from_secs(10), 3)
        .await
        .unwrap();

    assert!(txn_stat.submitted > 30);
}
