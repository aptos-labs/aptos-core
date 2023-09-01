// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::pipeline::LedgerUpdateMessage;
use aptos_crypto::hash::HashValue;
use aptos_executor::block_executor::{BlockExecutor, TransactionBlockExecutor};
use aptos_executor_types::BlockExecutorTrait;
use aptos_logger::info;
use aptos_types::block_executor::partitioner::ExecutableBlock;
use std::{
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};

pub struct TransactionExecutor<V> {
    num_blocks_processed: usize,
    executor: Arc<BlockExecutor<V>>,
    parent_block_id: HashValue,
    maybe_first_block_start_time: Option<Instant>,
    ledger_update_sender: mpsc::SyncSender<LedgerUpdateMessage>,
}

impl<V> TransactionExecutor<V>
where
    V: TransactionBlockExecutor,
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
        let num_txns = executable_block.transactions.num_transactions();
        let output = self
            .executor
            .execute_and_state_checkpoint(executable_block, self.parent_block_id, None)
            .unwrap();

        assert_eq!(output.txn_statuses().len(), num_txns);

        let msg = LedgerUpdateMessage {
            current_block_start_time,
            first_block_start_time: *self.maybe_first_block_start_time.as_ref().unwrap(),
            partition_time,
            execution_time: Instant::now().duration_since(execution_start_time),
            block_id,
            parent_block_id: self.parent_block_id,
            state_checkpoint_output: output,
        };
        self.ledger_update_sender.send(msg).unwrap();
        self.parent_block_id = block_id;
        self.num_blocks_processed += 1;
    }
}
