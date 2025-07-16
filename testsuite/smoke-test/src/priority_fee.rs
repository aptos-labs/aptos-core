// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{smoke_test_environment::SwarmBuilder, txn_emitter::generate_traffic};
use aptos_api_types::ViewFunction;
use aptos_forge::{args::TransactionTypeArg, NodeExt};
use aptos_types::on_chain_config::{
    BlockGasLimitType, ExecutionConfigV4, OnChainExecutionConfig, TransactionDeduperType,
    TransactionShufflerType,
};
use move_core_types::{identifier::Identifier, language_storage::ModuleId};
use std::{str::FromStr, sync::Arc, time::Duration};

#[tokio::test]
async fn test_high_gas_limit_sequential() {
    test_impl(100000000, 1).await;
}

#[tokio::test]
async fn test_high_gas_limit_parallel() {
    test_impl(100000000, 4).await;
}

#[tokio::test]
async fn test_low_gas_limit_sequential() {
    test_impl(30, 1).await;
}

#[tokio::test]
async fn test_low_gas_limit_parallel() {
    test_impl(30, 4).await;
}

async fn test_impl(block_gas_limit: u64, concurrency_level: u16) {
    let mut swarm = SwarmBuilder::new_local(2)
        .with_aptos()
        .with_init_config(Arc::new(move |_, config, _| {
            config.execution.concurrency_level = concurrency_level;
        }))
        // Start with V1
        .with_init_genesis_config(Arc::new(move |genesis_config| {
            let mut block_gas_limit_type = BlockGasLimitType::default_for_genesis();
            match &mut block_gas_limit_type {
                BlockGasLimitType::ComplexLimitV1 {
                    effective_block_gas_limit,
                    ..
                } => *effective_block_gas_limit = block_gas_limit,
                _ => unreachable!(),
            };
            genesis_config.execution_config = OnChainExecutionConfig::V4(ExecutionConfigV4 {
                transaction_shuffler_type: TransactionShufflerType::NoShuffling,
                block_gas_limit_type,
                transaction_deduper_type: TransactionDeduperType::TxnHashAndAuthenticatorV1,
            });
        }))
        .build()
        .await;

    let rest_client = swarm.validators().next().unwrap().rest_client();
    let view_function = ViewFunction {
        module: ModuleId::from_str("0x1::stake").unwrap(),
        function: Identifier::from_str("get_pending_transaction_fee").unwrap(),
        ty_args: vec![],
        args: vec![],
    };
    let result_before: Vec<Vec<u64>> = rest_client
        .view_bcs(&view_function, None)
        .await
        .unwrap()
        .into_inner();

    let all_validators = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();
    let _ = generate_traffic(
        &mut swarm,
        &all_validators,
        Duration::from_secs(20),
        100,
        vec![vec![
            (TransactionTypeArg::CoinTransfer.materialize_default(), 70),
            (
                TransactionTypeArg::AccountGeneration.materialize_default(),
                20,
            ),
        ]],
    )
    .await
    .unwrap();

    let result_after: Vec<Vec<u64>> = rest_client
        .view_bcs(&view_function, None)
        .await
        .unwrap()
        .into_inner();
    assert_ne!(result_before, result_after);
}

// TODO(grao): Add more tests to check the math.
