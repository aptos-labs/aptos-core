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

use aptos_db::utils::ShardedStateKvSchemaBatch;

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

// TODO (bowu)
pub struct CommitBatches {
    state_kv_metadata_batch: SchemaBatch,
    sharded_state_kv_batches: ShardedStateKvSchemaBatch,
}

pub struct TransactionCommitter<V> {
    executor: Arc<BlockExecutor<V>>,
    start_version: Version,
    block_receiver: mpsc::Receiver<CommitBlockMessage>,
    batch_sender: mpsc::Sender<CommitBatches>,
    batch_receiver: mpsc::Receiver<CommitBatches>,
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
        // spawn a new thread in backgrond to do the actual commit
        let (batch_sender, batch_receiver) = mpsc::channel();

        Self {
            executor,
            start_version,
            block_receiver,
            batch_sender,
            batch_receiver,
        }
    }

    fn commit_batch(&self, batch: SchemaBatch) -> Result<()> {
        Ok(())
    }

    fn prepare_commit(&self, block_id: u64, ledger_info_sigs: LedgerInfoWithSignatures) -> Result<()> {
        self.executor.pre_commit_block(block_id)?;
        self.executor.commit_ledger(ledger_info_sigs)?;
        Ok(())
    }

    pub fn run(&mut self) {
        info!("Start with version: {}", self.start_version);

        // Spawn a new thread in backgrond to do the actual commit
        let commit_thread = thread::spawn(move || {
            while let Ok(batch) = self.batch_receiver.recv() {
                self.commit_batch(batch).unwrap();
            }
        });

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
            NUM_TXNS
                .with_label_values(&["commit"])
                .inc_by(num_input_txns as u64);

            let version = output.expect_last_version();
            let commit_start = Instant::now();
            let ledger_info_with_sigs = gen_li_with_sigs(block_id, root_hash, version);
            self.prepare_commit(block_id, ledger_info_sigs).unwrap();

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
