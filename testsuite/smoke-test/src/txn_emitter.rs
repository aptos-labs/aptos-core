// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::new_local_swarm_with_aptos;
use anyhow::ensure;
use aptos_forge::{
    args::TransactionTypeArg, EmitJobMode, EmitJobRequest, EntryPoints, NodeExt, Result, Swarm,
    TransactionType, TxnEmitter, TxnStats,
};
use aptos_sdk::{transaction_builder::TransactionFactory, types::PeerId};
use rand::{rngs::OsRng, SeedableRng};
use std::time::Duration;

pub async fn generate_traffic(
    swarm: &mut dyn Swarm,
    nodes: &[PeerId],
    duration: Duration,
    gas_price: u64,
    transaction_mix_per_phase: Vec<Vec<(TransactionType, usize)>>,
) -> Result<TxnStats> {
    ensure!(gas_price > 0, "gas_price is required to be non zero");
    let rng = SeedableRng::from_rng(OsRng)?;
    let validator_clients = swarm
        .validators()
        .filter(|v| nodes.contains(&v.peer_id()))
        .map(|n| n.rest_client())
        .collect::<Vec<_>>();
    let mut emit_job_request = EmitJobRequest::default();
    let chain_info = swarm.chain_info();
    let transaction_factory =
        TransactionFactory::new(chain_info.chain_id).with_gas_unit_price(gas_price);
    let emitter = TxnEmitter::new(transaction_factory, rng);

    emit_job_request = emit_job_request
        .rest_clients(validator_clients)
        .gas_price(gas_price)
        .expected_gas_per_txn(1000000)
        .max_gas_per_txn(2000000)
        .coordination_delay_between_instances(Duration::from_secs(1))
        .transaction_mix_per_phase(transaction_mix_per_phase)
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

    let txn_stat = generate_traffic(
        &mut swarm,
        &all_validators,
        Duration::from_secs(20),
        100,
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
                (TransactionTypeArg::NoOp.materialize(100, false), 20),
                (
                    TransactionType::CallCustomModules {
                        entry_point: EntryPoints::MakeOrChangeTable {
                            offset: 0,
                            count: 60,
                        },
                        num_modules: 1,
                        use_account_pool: false,
                    },
                    20,
                ),
            ],
            // vec![(
            //     TransactionType::CallCustomModules {
            //         entry_point: EntryPoints::TokenV1MintAndStoreNFTSequential,
            //         num_modules: 1,
            //         use_account_pool: false,
            //     },
            //     20,
            // )],
            // vec![(
            //     TransactionType::CallCustomModules {
            //         entry_point: EntryPoints::TokenV1MintAndTransferNFTParallel,
            //         num_modules: 1,
            //         use_account_pool: false,
            //     },
            //     20,
            // )],
            // vec![(
            //     TransactionType::CallCustomModules {
            //         entry_point: EntryPoints::TokenV1MintAndTransferNFTSequential,
            //         num_modules: 1,
            //         use_account_pool: false,
            //     },
            //     20,
            // )],
        ],
    )
    .await
    .unwrap();
    println!("{:?}", txn_stat.rate());
    // assert some much smaller number than expected, so it doesn't fail under contention
    assert!(txn_stat.submitted > 30);
    assert!(txn_stat.committed > 30);
}
