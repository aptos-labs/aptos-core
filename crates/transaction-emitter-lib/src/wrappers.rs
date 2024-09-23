// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    args::{ClusterArgs, EmitArgs},
    cluster::Cluster,
    emitter::{
        account_minter::bulk_create_accounts, get_needed_balance_per_account_from_req,
        local_account_generator::PrivateKeyAccountGenerator, stats::TxnStats,
        transaction_executor::RestApiReliableTransactionSubmitter, EmitJobMode, EmitJobRequest,
        NumAccountsMode, TxnEmitter,
    },
    instance::Instance,
    CreateAccountsArgs,
};
use anyhow::{bail, Context, Result};
use aptos_logger::{error, info};
use aptos_sdk::transaction_builder::TransactionFactory;
use aptos_transaction_generator_lib::{args::TransactionTypeArg, WorkflowProgress};
use rand::{rngs::StdRng, SeedableRng};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

pub async fn emit_transactions(
    cluster_args: &ClusterArgs,
    emit_args: &EmitArgs,
) -> Result<TxnStats> {
    if emit_args.coordination_delay_between_instances.is_none() {
        let cluster = Cluster::try_from_cluster_args(cluster_args)
            .await
            .context("Failed to build cluster")?;
        emit_transactions_with_cluster(&cluster, emit_args).await
    } else {
        let initial_delay_after_minting = emit_args.coordination_delay_between_instances.unwrap();
        let start_time = Instant::now();
        let mut i = 0;
        loop {
            let cur_emit_args = if i > 0 {
                let mut cur_emit_args = emit_args.clone();
                cur_emit_args.coordination_delay_between_instances =
                    initial_delay_after_minting.checked_sub(start_time.elapsed().as_secs());
                if cur_emit_args.coordination_delay_between_instances.is_none() {
                    bail!("txn_emitter couldn't succeed after {} runs", i);
                }
                info!(
                    "Reduced coordination_delay_between_instances to {} for run {}",
                    cur_emit_args.coordination_delay_between_instances.unwrap(),
                    i
                );
                cur_emit_args
            } else {
                emit_args.clone()
            };

            let cluster = Cluster::try_from_cluster_args(cluster_args)
                .await
                .context("Failed to build cluster")?;

            let result = emit_transactions_with_cluster(&cluster, &cur_emit_args).await;
            match result {
                Ok(value) => return Ok(value),
                Err(e) => {
                    error!("Couldn't run txn emitter: {:?}, retrying", e)
                },
            }
            i += 1;
        }
    }
}

pub async fn emit_transactions_with_cluster(
    cluster: &Cluster,
    args: &EmitArgs,
) -> Result<TxnStats> {
    let emitter_mode = EmitJobMode::create(args.mempool_backlog, args.target_tps);

    let duration = Duration::from_secs(args.duration);
    let client = cluster.random_instance().rest_client();
    let coin_source_account = cluster.load_coin_source_account(&client).await?;
    let emitter = TxnEmitter::new(
        TransactionFactory::new(cluster.chain_id)
            .with_transaction_expiration_time(args.txn_expiration_time_secs)
            .with_gas_unit_price(aptos_global_constants::GAS_UNIT_PRICE),
        StdRng::from_entropy(),
    );

    let transaction_mix_per_phase = TransactionTypeArg::args_to_transaction_mix_per_phase(
        &args.transaction_type,
        &args.transaction_weights,
        &args.transaction_phases,
        args.module_working_set_size.unwrap_or(1),
        args.sender_use_account_pool.unwrap_or(false),
        WorkflowProgress::when_done_default(),
    );
    let mut emit_job_request =
        EmitJobRequest::new(cluster.all_instances().map(Instance::rest_client).collect())
            .mode(emitter_mode)
            .transaction_mix_per_phase(transaction_mix_per_phase)
            .txn_expiration_time_secs(args.txn_expiration_time_secs)
            .coordination_delay_between_instances(Duration::from_secs(
                args.coordination_delay_between_instances.unwrap_or(0),
            ));

    let num_accounts =
        NumAccountsMode::create(args.num_accounts, args.max_transactions_per_account);

    emit_job_request = emit_job_request.num_accounts_mode(num_accounts);

    if let Some(gas_price) = args.gas_price {
        emit_job_request = emit_job_request.gas_price(gas_price);
    }

    if let Some(max_gas_per_txn) = args.max_gas_per_txn {
        emit_job_request = emit_job_request.max_gas_per_txn(max_gas_per_txn);
    }

    if let Some(init_max_gas_per_txn) = args.init_max_gas_per_txn {
        emit_job_request = emit_job_request.init_max_gas_per_txn(init_max_gas_per_txn);
    }

    if let Some(init_gas_price_multiplier) = args.init_gas_price_multiplier {
        emit_job_request = emit_job_request.init_gas_price_multiplier(init_gas_price_multiplier);
    }

    if let Some(expected_max_txns) = args.expected_max_txns {
        emit_job_request = emit_job_request.expected_max_txns(expected_max_txns);
    }
    if let Some(expected_gas_per_txn) = args.expected_gas_per_txn {
        emit_job_request = emit_job_request.expected_gas_per_txn(expected_gas_per_txn);
    }
    if let Some(expected_gas_per_transfer) = args.expected_gas_per_transfer {
        emit_job_request = emit_job_request.expected_gas_per_transfer(expected_gas_per_transfer);
    }
    if let Some(expected_gas_per_account_create) = args.expected_gas_per_account_create {
        emit_job_request =
            emit_job_request.expected_gas_per_account_create(expected_gas_per_account_create);
    }

    if cluster.coin_source_is_root {
        emit_job_request = emit_job_request.set_mint_to_root();
    } else {
        emit_job_request = emit_job_request.prompt_before_spending();
    }

    if let Some(seed) = &args.account_minter_seed {
        emit_job_request = emit_job_request.account_minter_seed(seed);
    }

    if let Some(coins) = args.coins_per_account_override {
        emit_job_request = emit_job_request.coins_per_account_override(coins);
    }

    if let Some(latency_polling_interval_s) = args.latency_polling_interval_s {
        emit_job_request = emit_job_request
            .latency_polling_interval(Duration::from_secs_f32(latency_polling_interval_s));
    }

    if args.skip_funding_accounts {
        emit_job_request = emit_job_request.skip_funding_accounts();
    }

    let coin_source_account = std::sync::Arc::new(coin_source_account);
    let stats = emitter
        .emit_txn_for_with_stats(
            coin_source_account,
            emit_job_request,
            duration,
            (args.duration / 10).clamp(1, 10),
        )
        .await?;
    Ok(stats)
}

pub async fn create_accounts_command(
    cluster_args: &ClusterArgs,
    create_accounts_args: &CreateAccountsArgs,
) -> Result<()> {
    let cluster = Cluster::try_from_cluster_args(cluster_args)
        .await
        .context("Failed to build cluster")?;
    let client = cluster.random_instance().rest_client();
    let coin_source_account = cluster.load_coin_source_account(&client).await?;
    let coin_source_account = Arc::new(coin_source_account);
    let txn_factory = TransactionFactory::new(cluster.chain_id)
        .with_transaction_expiration_time(60)
        .with_max_gas_amount(create_accounts_args.max_gas_per_txn);
    let rest_clients = cluster
        .all_instances()
        .map(Instance::rest_client)
        .collect::<Vec<_>>();
    let mut emit_job_request = EmitJobRequest::new(rest_clients.clone())
        .init_gas_price_multiplier(1)
        .expected_gas_per_txn(create_accounts_args.max_gas_per_txn)
        .max_gas_per_txn(create_accounts_args.max_gas_per_txn)
        .coins_per_account_override(0)
        .expected_max_txns(0)
        .prompt_before_spending();

    if let Some(seed) = &create_accounts_args.account_minter_seed {
        emit_job_request = emit_job_request.account_minter_seed(seed);
    }

    bulk_create_accounts(
        coin_source_account,
        &RestApiReliableTransactionSubmitter::new(rest_clients, 6, Duration::from_secs(10)),
        &txn_factory,
        Box::new(PrivateKeyAccountGenerator),
        (&emit_job_request).into(),
        create_accounts_args.count,
        get_needed_balance_per_account_from_req(&emit_job_request, create_accounts_args.count),
    )
    .await?;

    Ok(())
}
