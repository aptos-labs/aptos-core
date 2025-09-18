// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{metrics::NUM_TXNS, pipeline::CommitBlockMessage};
use aptos_crypto::hash::HashValue;
use aptos_db::metrics::API_LATENCY_SECONDS;
use aptos_executor::{
    block_executor::BlockExecutor,
    metrics::{
        BLOCK_EXECUTION_WORKFLOW_WHOLE, COMMIT_BLOCKS, GET_BLOCK_EXECUTION_OUTPUT_BY_EXECUTING,
    },
};
use aptos_executor_types::BlockExecutorTrait;
use aptos_logger::prelude::*;
use aptos_metrics_core::IntCounterVecHelper;
use aptos_types::{
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    transaction::Version,
};
use aptos_vm::VMBlockExecutor;
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

pub struct TransactionCommitter<V> {
    executor: Arc<BlockExecutor<V>>,
    start_version: Version,
    block_receiver: mpsc::Receiver<CommitBlockMessage>,
}

impl<V> TransactionCommitter<V>
where
    V: VMBlockExecutor,
{
    pub fn new(
        executor: Arc<BlockExecutor<V>>,
        start_version: Version,
        block_receiver: mpsc::Receiver<CommitBlockMessage>,
    ) -> Self {
        Self {
            executor,
            start_version,
            block_receiver,
        }
    }

    pub fn run(&mut self) {
        info!("Start with version: {}", self.start_version);

        while let Ok(msg) = self.block_receiver.recv() {
            let CommitBlockMessage {
                block_id,
                first_block_start_time,
                current_block_start_time,
                partition_time,
                execution_time,
                output,
            } = msg;
            let root_hash = output
                .ledger_update_output
                .transaction_accumulator
                .root_hash();
            let num_input_txns = output.num_input_transactions();
            NUM_TXNS.inc_with_by(&["commit"], num_input_txns as u64);

            let version = output.expect_last_version();
            let commit_start = Instant::now();
            let ledger_info_with_sigs = gen_li_with_sigs(block_id, root_hash, version);
            self.executor.pre_commit_block(block_id).unwrap();
            self.executor.commit_ledger(ledger_info_with_sigs).unwrap();

            report_block(
                self.start_version,
                version,
                first_block_start_time,
                current_block_start_time,
                partition_time,
                execution_time,
                Instant::now().duration_since(commit_start),
                num_input_txns,
            );
        }
    }
}

fn report_block(
    start_version: Version,
    version: Version,
    first_block_start_time: Instant,
    current_block_start_time: Instant,
    partition_time: Duration,
    execution_time: Duration,
    commit_time: Duration,
    block_size: usize,
) {
    let total_versions = (version - start_version) as f64;
    info!(
        "Version: {}. latency: {} ms, partition time: {} ms, execute time: {} ms. commit time: {} ms. TPS: {:.0} (partition: {:.0}, execution: {:.0}, commit: {:.0}). Accumulative TPS: {:.0}",
        version,
        Instant::now().duration_since(current_block_start_time).as_millis(),
        partition_time.as_millis(),
        execution_time.as_millis(),
        commit_time.as_millis(),
        block_size as f64 / (std::cmp::max(std::cmp::max(partition_time, execution_time), commit_time)).as_secs_f64(),
        block_size as f64 / partition_time.as_secs_f64(),
        block_size as f64 / execution_time.as_secs_f64(),
        block_size as f64 / commit_time.as_secs_f64(),
        total_versions / first_block_start_time.elapsed().as_secs_f64(),
    );
    info!(
            "Accumulative total: BlockSTM+VM time: {:.0} secs, executor time: {:.0} secs, commit time: {:.0} secs, DB commit time: {:.0} secs",
            GET_BLOCK_EXECUTION_OUTPUT_BY_EXECUTING.get_sample_sum(),
            BLOCK_EXECUTION_WORKFLOW_WHOLE.get_sample_sum() - GET_BLOCK_EXECUTION_OUTPUT_BY_EXECUTING.get_sample_sum(),
            COMMIT_BLOCKS.get_sample_sum(),
            API_LATENCY_SECONDS.get_metric_with_label_values(&["save_transactions", "Ok"]).expect("must exist.").get_sample_sum(),
        );
    const NANOS_PER_SEC: f64 = 1_000_000_000.0;
    info!(
            "Accumulative per transaction: BlockSTM+VM time: {:.0} ns, executor time: {:.0} ns, commit time: {:.0} ns, DB commit time: {:.0} ns",
            GET_BLOCK_EXECUTION_OUTPUT_BY_EXECUTING.get_sample_sum() * NANOS_PER_SEC
                / total_versions,
            (BLOCK_EXECUTION_WORKFLOW_WHOLE.get_sample_sum() - GET_BLOCK_EXECUTION_OUTPUT_BY_EXECUTING.get_sample_sum()) * NANOS_PER_SEC
                / total_versions,
            COMMIT_BLOCKS.get_sample_sum() * NANOS_PER_SEC
                / total_versions,
            API_LATENCY_SECONDS.get_metric_with_label_values(&["save_transactions", "Ok"]).expect("must exist.").get_sample_sum() * NANOS_PER_SEC
                / total_versions,
        );
}
