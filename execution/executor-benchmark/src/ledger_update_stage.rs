// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::pipeline::{CommitBlockMessage, LedgerUpdateMessage};
use aptos_executor::block_executor::{BlockExecutor, TransactionBlockExecutor};
use aptos_executor_types::BlockExecutorTrait;
use aptos_types::transaction::Version;
use std::sync::{mpsc, Arc};

pub struct LedgerUpdateStage<V> {
    executor: Arc<BlockExecutor<V>>,
    // If commit_sender is `None`, we will commit all the execution result immediately in this struct.
    commit_sender: Option<mpsc::SyncSender<CommitBlockMessage>>,
    version: Version,
    allow_discards: bool,
    allow_aborts: bool,
}

impl<V> LedgerUpdateStage<V>
where
    V: TransactionBlockExecutor,
{
    pub fn new(
        executor: Arc<BlockExecutor<V>>,
        commit_sender: Option<mpsc::SyncSender<CommitBlockMessage>>,
        version: Version,
        allow_discards: bool,
        allow_aborts: bool,
    ) -> Self {
        Self {
            executor,
            version,
            commit_sender,
            allow_discards,
            allow_aborts,
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

        let num_txns = output.compute_status().len();
        self.version += num_txns as Version;
        let discards = output
            .compute_status()
            .iter()
            .flat_map(|status| match status.status() {
                Ok(_) => None,
                Err(error_code) => Some(format!("{:?}", error_code)),
            })
            .collect::<Vec<_>>();

        let aborts = output
            .compute_status()
            .iter()
            .flat_map(|status| match status.status() {
                Ok(execution_status) => {
                    if execution_status.is_success() {
                        None
                    } else {
                        Some(format!("{:?}", execution_status))
                    }
                },
                Err(_) => None,
            })
            .collect::<Vec<_>>();
        if !discards.is_empty() || !aborts.is_empty() {
            println!(
                "Some transactions were not successful: {} discards and {} aborts out of {}, examples: discards: {:?}, aborts: {:?}",
                discards.len(),
                aborts.len(),
                output.compute_status().len(),
                &discards[..(discards.len().min(3))],
                &aborts[..(aborts.len().min(3))]
            )
        }

        assert!(
            self.allow_discards || discards.is_empty(),
            "No discards allowed, {}, examples: {:?}",
            discards.len(),
            &discards[..(discards.len().min(3))]
        );
        assert!(
            self.allow_aborts || aborts.is_empty(),
            "No aborts allowed, {}, examples: {:?}",
            aborts.len(),
            &aborts[..(aborts.len().min(3))]
        );

        if let Some(commit_sender) = &self.commit_sender {
            let msg = CommitBlockMessage {
                block_id,
                root_hash: output.root_hash(),
                first_block_start_time,
                current_block_start_time,
                partition_time,
                execution_time,
                num_txns: num_txns - discards.len(),
            };
            commit_sender.send(msg).unwrap();
        } else {
            let ledger_info_with_sigs = super::transaction_committer::gen_li_with_sigs(
                block_id,
                output.root_hash(),
                self.version,
            );
            self.executor
                .commit_blocks(vec![block_id], ledger_info_with_sigs)
                .unwrap();
        }
    }
}
