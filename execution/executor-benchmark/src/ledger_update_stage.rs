// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::pipeline::{CommitBlockMessage, LedgerUpdateMessage};
use aptos_executor::block_executor::{BlockExecutor, TransactionBlockExecutor};
use aptos_executor_types::BlockExecutorTrait;
use aptos_types::transaction::Version;
use std::sync::{mpsc, Arc};

pub enum CommitProcessing {
    SendToQueue(mpsc::SyncSender<CommitBlockMessage>),
    #[allow(dead_code)]
    ExecuteInline,
    Skip,
}

pub struct LedgerUpdateStage<V> {
    executor: Arc<BlockExecutor<V>>,
    commit_processing: CommitProcessing,
    version: Version,
}

impl<V> LedgerUpdateStage<V>
where
    V: TransactionBlockExecutor,
{
    pub fn new(
        executor: Arc<BlockExecutor<V>>,
        commit_processing: CommitProcessing,
        version: Version,
    ) -> Self {
        Self {
            executor,
            version,
            commit_processing,
        }
    }

    pub fn ledger_update(&mut self, ledger_update_message: LedgerUpdateMessage) {
        // let ledger_update_start_time = Instant::now();
        let LedgerUpdateMessage {
            current_block_start_time,
            execution_time,
            partition_time,
            block_id,
            parent_block_id,
            state_checkpoint_output,
            first_block_start_time,
        } = ledger_update_message;

        let output = self
            .executor
            .ledger_update(block_id, parent_block_id, state_checkpoint_output)
            .unwrap();

        self.version += output.transactions_to_commit_len() as Version;

        match &self.commit_processing {
            CommitProcessing::SendToQueue(commit_sender) => {
                let msg = CommitBlockMessage {
                    block_id,
                    root_hash: output.root_hash(),
                    first_block_start_time,
                    current_block_start_time,
                    partition_time,
                    execution_time,
                    num_txns: output.transactions_to_commit_len(),
                };
                commit_sender.send(msg).unwrap();
            },
            CommitProcessing::ExecuteInline => {
                let ledger_info_with_sigs = super::transaction_committer::gen_li_with_sigs(
                    block_id,
                    output.root_hash(),
                    self.version,
                );
                self.executor
                    .commit_blocks(vec![block_id], ledger_info_with_sigs)
                    .unwrap();
            },
            CommitProcessing::Skip => {},
        }
    }
}
