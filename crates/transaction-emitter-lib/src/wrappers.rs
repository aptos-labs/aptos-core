// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::args::{ClusterArgs, EmitArgs};
use crate::cluster::Cluster;
use crate::emit::EmitJobRequest;
use crate::emit::EmitThreadParams;
use crate::emit::TxnEmitter;
use crate::emit::TxnStats;
use crate::instance::Instance;
use anyhow::{Context, Result};
use aptos_sdk::transaction_builder::TransactionFactory;
use rand::rngs::StdRng;
use rand::Rng;
use rand_core::OsRng;
use rand_core::SeedableRng;
use std::cmp::min;
use std::convert::TryFrom;
use std::time::Duration;

pub async fn emit_transactions(
    cluster_args: &ClusterArgs,
    emit_args: &EmitArgs,
) -> Result<TxnStats> {
    let cluster = Cluster::try_from(cluster_args).context("Failed to build cluster")?;
    emit_transactions_with_cluster(&cluster, &emit_args, cluster_args.vasp).await
}

pub async fn emit_transactions_with_cluster(
    cluster: &Cluster,
    args: &EmitArgs,
    vasp: bool,
) -> Result<TxnStats> {
    let thread_params = EmitThreadParams {
        wait_millis: args.wait_millis,
        wait_committed: !args.burst,
        txn_expiration_time_secs: args.txn_expiration_time_secs,
        check_stats_at_end: args.check_stats_at_end,
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
            .accounts_per_client(args.accounts_per_client)
            .thread_params(thread_params)
            .invalid_transaction_ratio(args.invalid_tx)
            .max_tps(args.max_tps)
            .gas_price(1);
    if let Some(workers_per_endpoint) = args.workers_per_ac {
        emit_job_request = emit_job_request.workers_per_endpoint(workers_per_endpoint);
    }
    if vasp {
        emit_job_request = emit_job_request.vasp();
    }
    let stats = emitter
        .emit_txn_for_with_stats(duration, emit_job_request, min(10, args.duration / 5))
        .await?;
    Ok(stats)
}
