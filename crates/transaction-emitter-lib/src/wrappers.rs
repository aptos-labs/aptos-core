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
    let emitter_mode = if args.mempool_backlog > 0 {
        assert!(
            args.target_tps == 0,
            "Cannot set both mempool-backlog and target-tps"
        );
        EmitJobMode::MaxLoad {
            mempool_backlog: args.mempool_backlog,
        }
    } else {
        assert!(
            args.target_tps > 0,
            "Need to set either mempool-backlog or target-tps"
        );
        EmitJobMode::ConstTps {
            tps: args.target_tps,
        }
    };

    let duration = Duration::from_secs(args.duration);
    let client = cluster.random_instance().rest_client();
    let mut root_account = cluster.load_aptos_root_account(&client).await?;
    let mut emitter = TxnEmitter::new(
        &mut root_account,
        client,
        TransactionFactory::new(cluster.chain_id)
            .with_gas_unit_price(1)
            .with_transaction_expiration_time(args.txn_expiration_time_secs),
        StdRng::from_seed(OsRng.gen()),
    );
    let mut emit_job_request =
        EmitJobRequest::new(cluster.all_instances().map(Instance::rest_client).collect())
            .mode(emitter_mode)
            .invalid_transaction_ratio(args.invalid_tx)
            .transaction_type(args.transaction_type)
            .duration(duration)
            .txn_expiration_time_secs(args.txn_expiration_time_secs)
            .check_stats_at_end(!args.do_not_check_stats_at_end)
            .gas_price(1);
    if reuse_accounts {
        emit_job_request = emit_job_request.reuse_accounts();
    }
    let stats = emitter
        .emit_txn_for_with_stats(emit_job_request, min(10, max(args.duration / 5, 1)))
        .await?;
    Ok(stats)
}
