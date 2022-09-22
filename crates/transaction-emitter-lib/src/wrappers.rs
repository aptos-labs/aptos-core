// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    args::{ClusterArgs, EmitArgs},
    cluster::Cluster,
    emitter::{stats::TxnStats, EmitJobMode, EmitJobRequest, TxnEmitter},
    instance::Instance,
};
use anyhow::{Context, Result};
use aptos_sdk::transaction_builder::TransactionFactory;
use rand::{rngs::StdRng, Rng};
use rand_core::{OsRng, SeedableRng};
use std::{
    cmp::{max, min},
    time::Duration,
};

pub async fn emit_transactions(
    cluster_args: &ClusterArgs,
    emit_args: &EmitArgs,
) -> Result<TxnStats> {
    let cluster = Cluster::try_from_cluster_args(cluster_args)
        .await
        .context("Failed to build cluster")?;
    emit_transactions_with_cluster(&cluster, emit_args, cluster_args.reuse_accounts).await
}

pub async fn emit_transactions_with_cluster(
    cluster: &Cluster,
    args: &EmitArgs,
    reuse_accounts: bool,
) -> Result<TxnStats> {
    let emitter_mode = EmitJobMode::create(args.mempool_backlog, args.target_tps);

    let duration = Duration::from_secs(args.duration);
    let client = cluster.random_instance().rest_client();
    let mut root_account = cluster.load_aptos_root_account(&client).await?;
    let mut emitter = TxnEmitter::new(
        TransactionFactory::new(cluster.chain_id)
            .with_transaction_expiration_time(args.txn_expiration_time_secs)
            .with_gas_unit_price(aptos_global_constants::GAS_UNIT_PRICE),
        StdRng::from_seed(OsRng.gen()),
    );

    let transaction_mix = if args.transaction_type_weights.is_empty() {
        args.transaction_type.iter().map(|t| (*t, 1)).collect()
    } else {
        assert_eq!(
            args.transaction_type_weights.len(),
            args.transaction_type.len(),
            "Transaction types and weights need to be the same length"
        );
        args.transaction_type
            .iter()
            .cloned()
            .zip(args.transaction_type_weights.iter().cloned())
            .collect()
    };

    let mut emit_job_request =
        EmitJobRequest::new(cluster.all_instances().map(Instance::rest_client).collect())
            .mode(emitter_mode)
            .invalid_transaction_ratio(args.invalid_tx)
            .transaction_mix(transaction_mix)
            .txn_expiration_time_secs(args.txn_expiration_time_secs)
            .gas_price(aptos_global_constants::GAS_UNIT_PRICE);
    if reuse_accounts {
        emit_job_request = emit_job_request.reuse_accounts();
    }
    let stats = emitter
        .emit_txn_for_with_stats(
            &mut root_account,
            emit_job_request,
            duration,
            min(10, max(args.duration / 5, 1)),
        )
        .await?;
    Ok(stats)
}
