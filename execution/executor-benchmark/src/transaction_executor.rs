// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{metrics::TIMER, pipeline::LedgerUpdateMessage};
use aptos_crypto::hash::HashValue;
use aptos_executor::block_executor::BlockExecutor;
use aptos_executor_types::BlockExecutorTrait;
use aptos_logger::info;
use aptos_metrics_core::TimerHelper;
use aptos_types::block_executor::{
    config::BlockExecutorConfigFromOnchain, partitioner::ExecutableBlock,
};
use aptos_vm::VMBlockExecutor;
use std::{
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};

pub const BENCHMARKS_BLOCK_EXECUTOR_ONCHAIN_CONFIG: BlockExecutorConfigFromOnchain =
    BlockExecutorConfigFromOnchain::on_but_large_for_test();

pub struct TransactionExecutor<V> {
    num_blocks_processed: usize,
    executor: Arc<BlockExecutor<V>>,
    parent_block_id: HashValue,
    maybe_first_block_start_time: Option<Instant>,
    ledger_update_sender: mpsc::SyncSender<LedgerUpdateMessage>,
}

impl<V> TransactionExecutor<V>
where
    V: VMBlockExecutor,
{
    pub fn new(
        executor: Arc<BlockExecutor<V>>,
        parent_block_id: HashValue,
        ledger_update_sender: mpsc::SyncSender<LedgerUpdateMessage>,
    ) -> Self {
        Self {
            num_blocks_processed: 0,
            executor,
            parent_block_id,
            maybe_first_block_start_time: None,
            ledger_update_sender,
        }
    }

    pub fn execute_block(
        &mut self,
        current_block_start_time: Instant,
        partition_time: Duration,
        executable_block: ExecutableBlock,
        stage: usize,
    ) {
        let execution_start_time = Instant::now();
        if self.maybe_first_block_start_time.is_none() {
            self.maybe_first_block_start_time = Some(current_block_start_time);
        }
        let block_id = executable_block.block_id;
        info!(
            "In iteration {}, received block {}.",
            self.num_blocks_processed, block_id
        );
        let num_input_txns = executable_block.transactions.num_transactions();
        {
            let _timer = TIMER.timer_with(&["execute"]);
            self.executor
                .execute_and_update_state(
                    executable_block,
                    self.parent_block_id,
                    BENCHMARKS_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
                )
                .unwrap();
        }
        let msg = LedgerUpdateMessage {
            current_block_start_time,
            first_block_start_time: *self.maybe_first_block_start_time.as_ref().unwrap(),
            partition_time,
            execution_time: Instant::now().duration_since(execution_start_time),
            block_id,
            parent_block_id: self.parent_block_id,
            num_input_txns,
            stage,
        };
        self.ledger_update_sender.send(msg).unwrap();
        self.parent_block_id = block_id;
        self.num_blocks_processed += 1;
    }
}
