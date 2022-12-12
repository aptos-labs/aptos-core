// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::hash::HashValue;
use aptos_db::metrics::API_LATENCY_SECONDS;
use aptos_executor::{
    block_executor::BlockExecutor,
    metrics::{
        APTOS_EXECUTOR_COMMIT_BLOCKS_SECONDS, APTOS_EXECUTOR_EXECUTE_BLOCK_SECONDS,
        APTOS_EXECUTOR_VM_EXECUTE_BLOCK_SECONDS,
    },
};
use aptos_executor_types::BlockExecutorTrait;
use aptos_logger::prelude::*;
use aptos_types::aggregate_signature::AggregateSignature;
use aptos_types::{
    block_info::BlockInfo,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    transaction::Version,
};
use aptos_vm::AptosVM;
use std::{
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};

pub(crate) fn gen_li_with_sigs(
    block_id: HashValue,
    root_hash: HashValue,
    version: Version,
) -> LedgerInfoWithSignatures {
    let block_info = BlockInfo::new(
        1,        /* epoch */
        0,        /* round, doesn't matter */
        block_id, /* id, doesn't matter */
        root_hash, version, 0,    /* timestamp_usecs, doesn't matter */
        None, /* next_epoch_state */
    );
    let ledger_info = LedgerInfo::new(
        block_info,
        HashValue::zero(), /* consensus_data_hash, doesn't matter */
    );
    LedgerInfoWithSignatures::new(
        ledger_info,
        AggregateSignature::empty(), /* signatures */
    )
}

pub struct TransactionCommitter {
    executor: Arc<BlockExecutor<AptosVM>>,
    version: Version,
    block_receiver: mpsc::Receiver<(HashValue, HashValue, Instant, Instant, Duration, usize)>,
}

impl TransactionCommitter {
    pub fn new(
        executor: Arc<BlockExecutor<AptosVM>>,
        version: Version,
        block_receiver: mpsc::Receiver<(HashValue, HashValue, Instant, Instant, Duration, usize)>,
    ) -> Self {
        Self {
            version,
            executor,
            block_receiver,
        }
    }

    pub fn run(&mut self) {
        let start_version = self.version;
        info!("Start with version: {}", start_version);

        let mut agg_stats = AggStats {
            total_blocks: 0,
            total_transactions: 0,
            total_execution_time: 0.0,
            total_commit_time: 0.0,
            total_latency: 0.0,
        };

        while let Ok((
            block_id,
            root_hash,
            global_start_time,
            execution_start_time,
            execution_time,
            num_txns,
        )) = self.block_receiver.recv()
        {
            self.version += num_txns as u64;
            let commit_start = std::time::Instant::now();
            let ledger_info_with_sigs = gen_li_with_sigs(block_id, root_hash, self.version);
            self.executor
                .commit_blocks_ext(vec![block_id], ledger_info_with_sigs, false)
                .unwrap();

            report_block(
                start_version,
                self.version,
                global_start_time,
                execution_start_time,
                execution_time,
                Instant::now().duration_since(commit_start),
                num_txns,
                &mut agg_stats,
            );
        }
    }
}

struct AggStats {
    total_blocks: usize,
    total_transactions: u64,
    total_execution_time: f64,
    total_commit_time: f64,
    total_latency: f64,
}

fn report_block(
    start_version: Version,
    version: Version,
    global_start_time: Instant,
    execution_start_time: Instant,
    execution_time: Duration,
    commit_time: Duration,
    block_size: usize,
    agg_stats: &mut AggStats,
) {
    let total_versions = (version - start_version) as f64;
    let latency = Instant::now().duration_since(execution_start_time);

    agg_stats.total_blocks += 1;
    agg_stats.total_transactions += block_size as u64;
    agg_stats.total_execution_time += execution_time.as_secs_f64();
    agg_stats.total_commit_time += commit_time.as_secs_f64();
    agg_stats.total_latency += block_size as f64 * latency.as_secs_f64();
    info!(
        "Version: {}. latency: {} ms, execute time: {} ms. commit time: {} ms. TPS: {:.0} (execution: {:.0}, commit: {:.0}). Accumulative TPS: {:.0}",
        version,
        latency.as_millis(),
        execution_time.as_millis(),
        commit_time.as_millis(),
        block_size as f64 / (std::cmp::max(execution_time, commit_time)).as_secs_f64(),
        block_size as f64 / execution_time.as_secs_f64(),
        block_size as f64 / commit_time.as_secs_f64(),
        total_versions / global_start_time.elapsed().as_secs_f64(),
    );
    info!(
        "Accumulative TPS: {:.0}, execute time: {} ms. commit time: {} ms. execution TPS: {:.0}, commit TPS: {:.0}",
        total_versions / global_start_time.elapsed().as_secs_f64(),
        agg_stats.total_execution_time * 1000.0 / agg_stats.total_blocks as f64,
        agg_stats.total_commit_time * 1000.0 / agg_stats.total_blocks as f64,
        agg_stats.total_transactions as f64 / agg_stats.total_execution_time,
        agg_stats.total_transactions as f64 / agg_stats.total_commit_time,
    );

    info!(
            "Accumulative total: VM time: {:.0} secs, executor time: {:.0} secs, commit time: {:.0} secs, DB commit time: {:.0} secs",
            APTOS_EXECUTOR_VM_EXECUTE_BLOCK_SECONDS.get_sample_sum(),
            APTOS_EXECUTOR_EXECUTE_BLOCK_SECONDS.get_sample_sum() - APTOS_EXECUTOR_VM_EXECUTE_BLOCK_SECONDS.get_sample_sum(),
            APTOS_EXECUTOR_COMMIT_BLOCKS_SECONDS.get_sample_sum(),
            API_LATENCY_SECONDS.get_metric_with_label_values(&["save_transactions", "Ok"]).expect("must exist.").get_sample_sum(),
        );
    const NANOS_PER_SEC: f64 = 1_000_000_000.0;
    info!(
            "Accumulative per transaction: VM time: {:.0} ns, executor time: {:.0} ns, commit time: {:.0} ns, DB commit time: {:.0} ns",
            APTOS_EXECUTOR_VM_EXECUTE_BLOCK_SECONDS.get_sample_sum() * NANOS_PER_SEC
                / total_versions,
            (APTOS_EXECUTOR_EXECUTE_BLOCK_SECONDS.get_sample_sum() - APTOS_EXECUTOR_VM_EXECUTE_BLOCK_SECONDS.get_sample_sum()) * NANOS_PER_SEC
                / total_versions,
            APTOS_EXECUTOR_COMMIT_BLOCKS_SECONDS.get_sample_sum() * NANOS_PER_SEC
                / total_versions,
            API_LATENCY_SECONDS.get_metric_with_label_values(&["save_transactions", "Ok"]).expect("must exist.").get_sample_sum() * NANOS_PER_SEC
                / total_versions,
        );
}
