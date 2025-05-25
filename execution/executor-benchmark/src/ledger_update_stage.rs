// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::pipeline::{CommitBlockMessage, LedgerUpdateMessage};
use aptos_executor::block_executor::BlockExecutor;
use aptos_executor_types::BlockExecutorTrait;
use aptos_infallible::Mutex;
use aptos_vm::VMBlockExecutor;
use move_core_types::language_storage::StructTag;
use std::{
    collections::BTreeMap,
    sync::{mpsc, Arc},
};

pub enum CommitProcessing {
    SendToQueue(mpsc::SyncSender<CommitBlockMessage>),
    #[allow(dead_code)]
    ExecuteInline,
    Skip,
}

pub struct LedgerUpdateStage<V> {
    executor: Arc<BlockExecutor<V>>,
    commit_processing: CommitProcessing,
    allow_aborts: bool,
    allow_discards: bool,
    allow_retries: bool,
    event_summary: Arc<Mutex<BTreeMap<(usize, StructTag), usize>>>,
}

impl<V> LedgerUpdateStage<V>
where
    V: VMBlockExecutor,
{
    pub fn new(
        executor: Arc<BlockExecutor<V>>,
        commit_processing: CommitProcessing,
        allow_aborts: bool,
        allow_discards: bool,
        allow_retries: bool,
        event_summary: Arc<Mutex<BTreeMap<(usize, StructTag), usize>>>,
    ) -> Self {
        Self {
            executor,
            commit_processing,
            allow_aborts,
            allow_discards,
            allow_retries,
            event_summary,
        }
    }

    pub fn ledger_update(&mut self, ledger_update_message: LedgerUpdateMessage) {
        // let ledger_update_start_time = Instant::now();
        let LedgerUpdateMessage {
            first_block_start_time,
            current_block_start_time,
            partition_time,
            execution_time,
            block_id,
            parent_block_id,
            num_input_txns,
            stage,
        } = ledger_update_message;

        let output = self
            .executor
            .ledger_update(block_id, parent_block_id)
            .unwrap();
        output.execution_output.check_aborts_discards_retries(
            self.allow_aborts,
            self.allow_discards,
            self.allow_retries,
        );
        if !self.allow_retries {
            assert_eq!(output.num_transactions_to_commit(), num_input_txns + 1);
        }

        let mut event_summary = self.event_summary.lock();
        for output in &output.execution_output.to_commit.transaction_outputs {
            for event in output.events() {
                let count = event_summary
                    .entry((stage, event.type_tag().struct_tag().unwrap().clone()))
                    .or_insert(0);
                *count += 1;
            }
        }

        match &self.commit_processing {
            CommitProcessing::SendToQueue(commit_sender) => {
                let msg = CommitBlockMessage {
                    block_id,
                    first_block_start_time,
                    current_block_start_time,
                    partition_time,
                    execution_time,
                    output,
                };
                commit_sender.send(msg).unwrap();
            },
            CommitProcessing::ExecuteInline => {
                let ledger_info_with_sigs = super::transaction_committer::gen_li_with_sigs(
                    block_id,
                    output.root_hash(),
                    output.expect_last_version(),
                );
                self.executor.pre_commit_block(block_id).unwrap();
                self.executor.commit_ledger(ledger_info_with_sigs).unwrap();
            },
            CommitProcessing::Skip => {},
        }
    }
}
