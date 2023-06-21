// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_block_partitioner::sharded_block_partitioner::ShardedBlockPartitioner;
use aptos_crypto::hash::HashValue;
use aptos_executor::block_executor::{BlockExecutor, TransactionBlockExecutor};
use aptos_executor_types::BlockExecutorTrait;
use aptos_types::{
    block_executor::partitioner::{
        CrossShardDependencies, ExecutableBlock, ExecutableTransactions,
        TransactionWithDependencies,
    },
    transaction::{Transaction, Version},
};
use std::{
    sync::{mpsc, mpsc::Sender, Arc},
    thread,
    time::{Duration, Instant},
};

pub enum PartitionExecutionMode {
    Unsharded,
    ShardedPartitionThenExecute(ShardedBlockPartitioner),
    ShardedPipelined(ShardedBlockPartitioner),
}

pub struct TransactionExecutor<V> {
    partition_mode: PartitionExecutionMode,
    executor: Arc<BlockExecutor<V>>,
    parent_block_id: HashValue,
    maybe_first_block_start_time: Option<Instant>,
    version: Version,
    // If commit_sender is `None`, we will commit all the execution result immediately in this struct.
    commit_sender:
        Option<mpsc::SyncSender<(HashValue, HashValue, Instant, Instant, Duration, usize)>>,
    allow_discards: bool,
    allow_aborts: bool,
    maybe_pipelined_execution_tx: Option<Sender<BlockProcessingParams>>,
    started: bool,
}

impl<V> TransactionExecutor<V>
where
    V: TransactionBlockExecutor + 'static,
{
    pub fn new(
        partition_mode: PartitionExecutionMode,
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
            partition_mode,
            executor,
            parent_block_id,
            version,
            maybe_first_block_start_time: None,
            commit_sender,
            allow_discards,
            allow_aborts,
            maybe_pipelined_execution_tx: None,
            started: false,
        }
    }

    pub fn start(&mut self) {
        assert!(!self.started);
        self.started = true;

        if matches!(
            self.partition_mode,
            PartitionExecutionMode::ShardedPipelined(_)
        ) {
            let (tx, rx) = mpsc::channel();
            self.maybe_pipelined_execution_tx = Some(tx);
            let mut parent_block_id = self.parent_block_id;
            let executor = self.executor.clone();
            let maybe_commit_sender = self.commit_sender.clone();
            let allow_aborts = self.allow_aborts;
            let allow_discards = self.allow_discards;

            let _exe_thread = thread::Builder::new().spawn(move || {
                while let Ok(msg) = rx.recv() {
                    let BlockProcessingParams {
                        first_block_start_time,
                        latest_version,
                        current_block_start_time: start_time,
                        block,
                    } = msg;
                    parent_block_id = Self::process_executable_block(
                        allow_aborts,
                        allow_discards,
                        first_block_start_time,
                        executor.clone(),
                        &maybe_commit_sender,
                        start_time,
                        parent_block_id,
                        block,
                        latest_version,
                    );
                }
            });
        }
    }

    pub fn partition_block(
        block_id: HashValue,
        partitioner: &ShardedBlockPartitioner,
        mut transactions: Vec<Transaction>,
    ) -> ExecutableBlock<Transaction> {
        let last_txn = transactions.pop().unwrap();
        assert!(matches!(last_txn, Transaction::StateCheckpoint(_)));
        let analyzed_transactions = transactions.into_iter().map(|t| t.into()).collect();
        let mut sub_blocks = partitioner.partition(analyzed_transactions, 2);
        sub_blocks
            .last_mut()
            .unwrap()
            .sub_blocks
            .last_mut()
            .unwrap()
            .transactions
            .push(TransactionWithDependencies::new(
                last_txn,
                CrossShardDependencies::default(),
            ));
        ExecutableBlock::new(block_id, ExecutableTransactions::Sharded(sub_blocks))
    }

    pub fn execute_block(&mut self, transactions: Vec<Transaction>) {
        assert!(self.started);

        let execution_start = Instant::now();

        if self.maybe_first_block_start_time.is_none() {
            self.maybe_first_block_start_time = Some(execution_start)
        }

        let num_txns = transactions.len();
        self.version += num_txns as Version;

        let block_id = HashValue::random();
        let first_block_start_time = *self.maybe_first_block_start_time.as_ref().unwrap();
        match &self.partition_mode {
            PartitionExecutionMode::Unsharded => {
                let executable_block = (block_id, transactions).into();
                self.parent_block_id = Self::process_executable_block(
                    self.allow_aborts,
                    self.allow_discards,
                    first_block_start_time,
                    self.executor.clone(),
                    &self.commit_sender,
                    execution_start,
                    self.parent_block_id,
                    executable_block,
                    self.version,
                );
            },
            PartitionExecutionMode::ShardedPartitionThenExecute(partitioner) => {
                let executable_block = Self::partition_block(block_id, partitioner, transactions);
                self.parent_block_id = Self::process_executable_block(
                    self.allow_aborts,
                    self.allow_discards,
                    first_block_start_time,
                    self.executor.clone(),
                    &self.commit_sender,
                    execution_start,
                    self.parent_block_id,
                    executable_block,
                    self.version,
                );
            },
            PartitionExecutionMode::ShardedPipelined(partitioner) => {
                let executable_block = Self::partition_block(block_id, partitioner, transactions);
                let msg = BlockProcessingParams {
                    first_block_start_time,
                    latest_version: self.version,
                    current_block_start_time: execution_start,
                    block: executable_block,
                };
                self.maybe_pipelined_execution_tx
                    .as_ref()
                    .unwrap()
                    .send(msg)
                    .unwrap();
            },
        }
    }

    fn process_executable_block(
        allow_aborts: bool,
        allow_discards: bool,
        first_block_start_time: Instant,
        executor: Arc<BlockExecutor<V>>,
        maybe_commit_sender: &Option<
            mpsc::SyncSender<(HashValue, HashValue, Instant, Instant, Duration, usize)>,
        >,
        current_block_start_time: Instant,
        parent_block_id: HashValue,
        executable_block: ExecutableBlock<Transaction>,
        version: Version,
    ) -> HashValue {
        let block_id = executable_block.block_id;
        let num_txns = executable_block.transactions.num_transactions();
        let output = executor
            .execute_block(executable_block, parent_block_id, None)
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
            allow_discards || discards.is_empty(),
            "No discards allowed, {}, examples: {:?}",
            discards.len(),
            &discards[..(discards.len().min(3))]
        );
        assert!(
            allow_aborts || aborts.is_empty(),
            "No aborts allowed, {}, examples: {:?}",
            aborts.len(),
            &aborts[..(aborts.len().min(3))]
        );

        if let Some(commit_sender) = maybe_commit_sender {
            commit_sender
                .send((
                    block_id,
                    output.root_hash(),
                    first_block_start_time,
                    current_block_start_time,
                    Instant::now().duration_since(current_block_start_time),
                    num_txns - discards.len(),
                ))
                .unwrap();
        } else {
            let ledger_info_with_sigs = super::transaction_committer::gen_li_with_sigs(
                block_id,
                output.root_hash(),
                version,
            );
            executor
                .commit_blocks(vec![block_id], ledger_info_with_sigs)
                .unwrap();
        }

        block_id
    }
}

struct BlockProcessingParams {
    first_block_start_time: Instant,
    latest_version: Version,
    current_block_start_time: Instant,
    block: ExecutableBlock<Transaction>,
}
