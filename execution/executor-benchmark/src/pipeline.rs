// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{TransactionCommitter, TransactionExecutor};
use aptos_executor::block_executor::{BlockExecutor, TransactionBlockExecutor};
use aptos_executor_types::BlockExecutorTrait;
use aptos_logger::info;
use aptos_types::transaction::{Transaction, Version};
use aptos_vm::counters::TXN_GAS_USAGE;
use std::{
    marker::PhantomData,
    sync::{
        mpsc::{self, SyncSender},
        Arc,
    },
    thread::JoinHandle,
    time::Instant,
};

#[derive(Clone, Debug)]
pub struct PipelineConfig {
    pub delay_execution_start: bool,
    pub split_stages: bool,
    pub skip_commit: bool,
    pub allow_discards: bool,
    pub allow_aborts: bool,
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

        let (block_sender, block_receiver) = mpsc::sync_channel::<Vec<Transaction>>(
            if config.delay_execution_start {
                (num_blocks.unwrap() + 1).max(50)
            } else {
                50
            }, /* bound */
        );
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
                );
                let start_time = Instant::now();
                let mut executed = 0;
                let start_gas = TXN_GAS_USAGE.get_sample_sum();
                while let Ok(transactions) = block_receiver.recv() {
                    info!("Received block of size {:?} to execute", transactions.len());
                    executed += transactions.len();
                    exe.execute_block(transactions);
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
        let join_handles = vec![exe_thread, commit_thread];

        (
            Self {
                join_handles,
                phantom: PhantomData,
                start_execution_tx,
            },
            block_sender,
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
}
