// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    TransactionCommitter, TransactionExecutor,
};
use aptos_block_partitioner::sharded_block_partitioner::ShardedBlockPartitioner;
use aptos_executor::block_executor::{BlockExecutor, TransactionBlockExecutor};
use aptos_executor_types::BlockExecutorTrait;
use aptos_logger::info;
use aptos_types::transaction::{Transaction, Version};
use aptos_vm::counters::TXN_GAS_USAGE;
use std::{
    marker::PhantomData,
    sync::{
        Arc,
        mpsc::{self, SyncSender},
    },
    thread::JoinHandle,
    time::Instant,
};
use aptos_crypto::HashValue;
use aptos_types::block_executor::partitioner::{CrossShardDependencies, ExecutableBlock, ExecutableTransactions, TransactionWithDependencies};
use crate::block_partitioning::BlockPartitioningStage;

#[derive(Clone, Debug)]
pub struct PipelineConfig {
    pub delay_execution_start: bool,
    pub split_stages: bool,
    pub skip_commit: bool,
    pub allow_discards: bool,
    pub allow_aborts: bool,
    pub num_executor_shards: usize,
    pub async_partitioning: bool,
}

pub struct Pipeline<V> {
    join_handles: Vec<JoinHandle<()>>,
    phantom: PhantomData<V>,
    start_execution_tx: Option<SyncSender<()>>,
}

impl<V> Pipeline<V>
where
    V: TransactionBlockExecutor + 'static,
{
    pub fn new(
        executor: BlockExecutor<V>,
        version: Version,
        config: PipelineConfig,
        // Need to specify num blocks, to size queues correctly, when delay_execution_start, split_stages or skip_commit are used
        num_blocks: Option<usize>,
    ) -> (Self, mpsc::SyncSender<Vec<Transaction>>) {
        let parent_block_id = executor.committed_block_id();
        let executor_1 = Arc::new(executor);
        let executor_2 = executor_1.clone();

        let (raw_block_sender, raw_block_receiver) = mpsc::sync_channel::<Vec<Transaction>>(
            if config.delay_execution_start {
                (num_blocks.unwrap() + 1).max(50)
            } else {
                50
            }, /* bound */
        );

        let (executable_block_sender, executable_block_receiver) = mpsc::channel::<ParToExeMsg>();

        // Assume the distributed executor and the distributed partitioner share the same worker set.
        let num_partitioner_shards = config.num_executor_shards;

        let (maybe_exe_fin_sender, maybe_exe_fin_receiver) = if !config.async_partitioning {
            let (tx, rx) = mpsc::channel::<()>();
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };

        let (commit_sender, commit_receiver) = mpsc::sync_channel(
            if config.split_stages || config.skip_commit {
                (num_blocks.unwrap() + 1).max(3)
            } else {
                3
            }, /* bound */
        );

        let (start_execution_tx, start_execution_rx) = if config.delay_execution_start {
            let (start_execution_tx, start_execution_rx) = mpsc::sync_channel::<()>(1);
            (Some(start_execution_tx), Some(start_execution_rx))
        } else {
            (None, None)
        };

        let (start_commit_tx, start_commit_rx) = if config.split_stages || config.skip_commit {
            let (start_commit_tx, start_commit_rx) = mpsc::sync_channel::<()>(1);
            (Some(start_commit_tx), Some(start_commit_rx))
        } else {
            (None, None)
        };

        let partitioning_thread = std::thread::Builder::new().name("block_partitioning".to_string()).spawn(move||{
            let mut partitioning_stage = BlockPartitioningStage::new(executable_block_sender, maybe_exe_fin_receiver, num_partitioner_shards);
            while let Ok(txns) = raw_block_receiver.recv() {
                partitioning_stage.process(txns);
            }
        }).expect("Failed to spawn block partitioner thread.");

        let exe_thread = std::thread::Builder::new()
            .name("txn_executor".to_string())
            .spawn(move || {
                start_execution_rx.map(|rx| rx.recv());

                let mut exe = TransactionExecutor::new(
                    executor_1,
                    parent_block_id,
                    version,
                    Some(commit_sender),
                    config.allow_discards,
                    config.allow_aborts,
                    maybe_exe_fin_sender,
                );
                let start_time = Instant::now();
                let mut executed = 0;
                let start_gas = TXN_GAS_USAGE.get_sample_sum();
                while let Ok(msg) = executable_block_receiver.recv() {
                    let ParToExeMsg { current_block_start_time, block } = msg;
                    let block_size = block.transactions.num_transactions();
                    info!("Received block of size {:?} to execute", block_size);
                    executed += block_size;
                    exe.execute_block(current_block_start_time, block);
                    info!("Finished executing block");
                }

                let delta_gas = TXN_GAS_USAGE.get_sample_sum() - start_gas;

                let elapsed = start_time.elapsed().as_secs_f32();
                info!(
                    "Overall execution TPS: {} txn/s (over {} txns)",
                    executed as f32 / elapsed,
                    executed
                );
                info!(
                    "Overall execution GPS: {} gas/s (over {} txns)",
                    delta_gas as f32 / elapsed,
                    executed
                );

                start_commit_tx.map(|tx| tx.send(()));
            })
            .expect("Failed to spawn transaction executor thread.");

        let skip_commit = config.skip_commit;

        let commit_thread = std::thread::Builder::new()
            .name("txn_committer".to_string())
            .spawn(move || {
                start_commit_rx.map(|rx| rx.recv());
                info!("Starting commit thread");
                if !skip_commit {
                    let mut committer =
                        TransactionCommitter::new(executor_2, version, commit_receiver);
                    committer.run();
                }
            })
            .expect("Failed to spawn transaction committer thread.");
        let join_handles = vec![partitioning_thread, exe_thread, commit_thread];

        (
            Self {
                join_handles,
                phantom: PhantomData,
                start_execution_tx,
            },
            raw_block_sender,
        )
    }

    pub fn start_execution(&self) {
        self.start_execution_tx.as_ref().map(|tx| tx.send(()));
    }

    pub fn join(self) {
        for handle in self.join_handles {
            handle.join().unwrap()
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

}

/// Message from partitioning thread to execution thread.
pub struct ParToExeMsg {
    pub current_block_start_time: Instant,
    pub block: ExecutableBlock<Transaction>,
}
