// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_block_partitioner::sharded_block_partitioner::ShardedBlockPartitioner;
use aptos_crypto::hash::HashValue;
use aptos_executor::block_executor::{BlockExecutor, TransactionBlockExecutor};
use aptos_executor_types::BlockExecutorTrait;
use aptos_types::transaction::{Transaction, Version};
use std::{
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};
use aptos_types::block_executor::partitioner::{ExecutableBlock, ExecutableTransactions};

pub struct TransactionExecutor<V> {
    block_partitioner: Option<ShardedBlockPartitioner>,
    executor: Arc<BlockExecutor<V>>,
    parent_block_id: HashValue,
    start_time: Option<Instant>,
    version: Version,
    // If commit_sender is `None`, we will commit all the execution result immediately in this struct.
    commit_sender:
        Option<mpsc::SyncSender<(HashValue, HashValue, Instant, Instant, Duration, usize)>>,
    allow_discards: bool,
    allow_aborts: bool,
}

impl<V> TransactionExecutor<V>
where
    V: TransactionBlockExecutor,
{
    pub fn new(
        maybe_block_partitioner: Option<ShardedBlockPartitioner>,
        executor: Arc<BlockExecutor<V>>,
        parent_block_id: HashValue,
        version: Version,
        commit_sender: Option<
            mpsc::SyncSender<(HashValue, HashValue, Instant, Instant, Duration, usize)>,
        >,
        allow_discards: bool,
        allow_aborts: bool,
    ) -> Self {
        Self {
            block_partitioner: maybe_block_partitioner,
            executor,
            parent_block_id,
            version,
            start_time: None,
            commit_sender,
            allow_discards,
            allow_aborts,
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
        let executable_block = match &self.block_partitioner {
            Some(partitioner) => {
                let analyzed_transactions = transactions.into_iter().map(|t|t.into()).collect();
                let sub_blocks = partitioner.partition(analyzed_transactions, 2);
                ExecutableBlock::new(block_id, ExecutableTransactions::Sharded(sub_blocks))
            },
            None => {
                (block_id, transactions).into()
            },
        };
        let output = self
            .executor
            .execute_block(executable_block, self.parent_block_id, None)
            .unwrap();

        assert_eq!(output.compute_status().len(), num_txns);
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

        self.parent_block_id = block_id;

        if let Some(ref commit_sender) = self.commit_sender {
            commit_sender
                .send((
                    block_id,
                    output.root_hash(),
                    self.start_time.unwrap(),
                    execution_start,
                    Instant::now().duration_since(execution_start),
                    num_txns - discards.len(),
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
