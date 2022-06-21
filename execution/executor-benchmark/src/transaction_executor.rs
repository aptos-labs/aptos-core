// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::hash::HashValue;
use aptos_types::transaction::{Transaction, Version};
use aptos_vm::AptosVM;
use executor::block_executor::BlockExecutor;
use executor_types::BlockExecutorTrait;
use std::{
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};

pub struct TransactionExecutor {
    executor: Arc<BlockExecutor<AptosVM>>,
    parent_block_id: HashValue,
    start_time: Option<Instant>,
    version: Version,
    // If commit_sender is `None`, we will commit all the execution result immediately in this struct.
    commit_sender:
        Option<mpsc::SyncSender<(HashValue, HashValue, Instant, Instant, Duration, usize)>>,
}

impl TransactionExecutor {
    pub fn new(
        executor: Arc<BlockExecutor<AptosVM>>,
        parent_block_id: HashValue,
        version: Version,
        commit_sender: Option<
            mpsc::SyncSender<(HashValue, HashValue, Instant, Instant, Duration, usize)>,
        >,
    ) -> Self {
        Self {
            executor,
            parent_block_id,
            version,
            start_time: None,
            commit_sender,
        }
    }

    pub fn execute_block(&mut self, transactions: Vec<Transaction>) {
        if self.start_time.is_none() {
            self.start_time = Some(Instant::now())
        }

        let num_txns = transactions.len();
        self.version += num_txns as Version;

        let execution_start = Instant::now();

        let block_id = HashValue::random();
        let output = self
            .executor
            .execute_block((block_id, transactions), self.parent_block_id)
            .unwrap();

        self.parent_block_id = block_id;

        if let Some(ref commit_sender) = self.commit_sender {
            commit_sender
                .send((
                    block_id,
                    output.root_hash(),
                    self.start_time.unwrap(),
                    execution_start,
                    Instant::now().duration_since(execution_start),
                    num_txns,
                ))
                .unwrap();
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
