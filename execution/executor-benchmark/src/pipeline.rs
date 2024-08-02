// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_preparation::BlockPreparationStage,
    ledger_update_stage::{CommitProcessing, LedgerUpdateStage},
    metrics::NUM_TXNS,
    OverallMeasuring, TransactionCommitter, TransactionExecutor,
};
use aptos_block_partitioner::v2::config::PartitionerV2Config;
use aptos_crypto::HashValue;
use aptos_executor::block_executor::{BlockExecutor, TransactionBlockExecutor};
use aptos_executor_types::{state_checkpoint_output::StateCheckpointOutput, BlockExecutorTrait};
use aptos_logger::info;
use aptos_types::{
    block_executor::partitioner::ExecutableBlock,
    transaction::{Transaction, Version},
};
use derivative::Derivative;
use std::{
    marker::PhantomData,
    sync::{
        mpsc::{self, SyncSender},
        Arc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

#[derive(Debug, Derivative)]
#[derivative(Default)]
pub struct PipelineConfig {
    pub delay_execution_start: bool,
    pub split_stages: bool,
    pub skip_commit: bool,
    pub allow_aborts: bool,
    pub allow_discards: bool,
    pub allow_retries: bool,
    #[derivative(Default(value = "0"))]
    pub num_executor_shards: usize,
    pub use_global_executor: bool,
    #[derivative(Default(value = "4"))]
    pub num_generator_workers: usize,
    pub partitioner_config: PartitionerV2Config,
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
        config: &PipelineConfig,
        // Need to specify num blocks, to size queues correctly, when delay_execution_start, split_stages or skip_commit are used
        num_blocks: Option<usize>,
    ) -> (Self, mpsc::SyncSender<Vec<Transaction>>) {
        let parent_block_id = executor.committed_block_id();
        let executor_1 = Arc::new(executor);
        let executor_2 = executor_1.clone();
        let executor_3 = executor_1.clone();

        let (raw_block_sender, raw_block_receiver) = mpsc::sync_channel::<Vec<Transaction>>(
            if config.delay_execution_start {
                (num_blocks.unwrap() + 1).max(50)
            } else {
                10
            }, /* bound */
        );

        // Assume the distributed executor and the distributed partitioner share the same worker set.
        let num_partitioner_shards = config.num_executor_shards;

        let (ledger_update_sender, ledger_update_receiver) =
            mpsc::sync_channel::<LedgerUpdateMessage>(
                if config.split_stages || config.skip_commit {
                    (num_blocks.unwrap() + 1).max(3)
                } else {
                    3
                }, /* bound */
            );

        let (commit_sender, commit_receiver) = mpsc::sync_channel::<CommitBlockMessage>(
            if config.split_stages {
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

        let (start_commit_tx, start_commit_rx) = if config.split_stages {
            let (start_commit_tx, start_commit_rx) = mpsc::sync_channel::<()>(1);
            (Some(start_commit_tx), Some(start_commit_rx))
        } else {
            (None, None)
        };

        let mut join_handles = vec![];

        let mut partitioning_stage =
            BlockPreparationStage::new(num_partitioner_shards, &config.partitioner_config);

        let mut exe = TransactionExecutor::new(
            executor_1,
            parent_block_id,
            ledger_update_sender,
            config.allow_aborts,
            config.allow_discards,
            config.allow_retries,
        );

        let commit_processing = if config.skip_commit {
            CommitProcessing::Skip
        } else {
            CommitProcessing::SendToQueue(commit_sender)
        };
        let mut ledger_update_stage =
            LedgerUpdateStage::new(executor_2, commit_processing, version);

        let (executable_block_sender, executable_block_receiver) =
            mpsc::sync_channel::<ExecuteBlockMessage>(3);

        let partitioning_thread = std::thread::Builder::new()
            .name("block_partitioning".to_string())
            .spawn(move || {
                while let Ok(txns) = raw_block_receiver.recv() {
                    NUM_TXNS
                        .with_label_values(&["partition"])
                        .inc_by(txns.len() as u64);
                    let exe_block_msg = partitioning_stage.process(txns);
                    executable_block_sender.send(exe_block_msg).unwrap();
                }
            })
            .expect("Failed to spawn block partitioner thread.");
        join_handles.push(partitioning_thread);

        let exe_thread = std::thread::Builder::new()
            .name("txn_executor".to_string())
            .spawn(move || {
                start_execution_rx.map(|rx| rx.recv());
                let overall_measuring = OverallMeasuring::start();
                let mut executed = 0;

                let mut stage_index = 0;
                let mut stage_overall_measuring = overall_measuring.clone();
                let mut stage_executed = 0;

                while let Ok(msg) = executable_block_receiver.recv() {
                    let ExecuteBlockMessage {
                        current_block_start_time,
                        partition_time,
                        block,
                    } = msg;
                    let block_size = block.transactions.num_transactions() as u64;
                    NUM_TXNS
                        .with_label_values(&["execution"])
                        .inc_by(block_size);
                    info!("Received block of size {:?} to execute", block_size);
                    executed += block_size;
                    stage_executed += block_size;
                    exe.execute_block(current_block_start_time, partition_time, block);
                    info!("Finished executing block");

                    // Empty blocks indicate the end of a stage.
                    // Print the accumulated stage stats at that point.
                    if block_size == 0 {
                        if stage_executed > 0 {
                            info!("Execution finished stage {}", stage_index);
                            stage_overall_measuring.print_end(
                                &format!("Staged execution: stage {}:", stage_index),
                                stage_executed,
                            );
                        }
                        stage_index += 1;
                        stage_overall_measuring = OverallMeasuring::start();
                        stage_executed = 0;
                    }
                }

                if stage_index > 0 && stage_executed > 0 {
                    info!("Execution finished stage {}", stage_index);
                    stage_overall_measuring.print_end(
                        &format!("Staged execution: stage {}:", stage_index),
                        stage_executed,
                    );
                }

                if num_blocks.is_some() {
                    overall_measuring.print_end("Overall execution", executed);
                }
                start_commit_tx.map(|tx| tx.send(()));
            })
            .expect("Failed to spawn transaction executor thread.");
        join_handles.push(exe_thread);

        let ledger_update_thread = std::thread::Builder::new()
            .name("ledger_update".to_string())
            .spawn(move || {
                while let Ok(ledger_update_msg) = ledger_update_receiver.recv() {
                    let input_block_size =
                        ledger_update_msg.state_checkpoint_output.input_txns_len();
                    NUM_TXNS
                        .with_label_values(&["ledger_update"])
                        .inc_by(input_block_size as u64);
                    ledger_update_stage.ledger_update(ledger_update_msg);
                }
            })
            .expect("Failed to spawn ledger update thread.");
        join_handles.push(ledger_update_thread);

        if !config.skip_commit {
            let commit_thread = std::thread::Builder::new()
                .name("txn_committer".to_string())
                .spawn(move || {
                    start_commit_rx.map(|rx| rx.recv());
                    info!("Starting commit thread");
                    let mut committer =
                        TransactionCommitter::new(executor_3, version, commit_receiver);
                    committer.run();
                })
                .expect("Failed to spawn transaction committer thread.");
            join_handles.push(commit_thread);
        }

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
}

/// Message from partitioning stage to execution stage.
pub struct ExecuteBlockMessage {
    pub current_block_start_time: Instant,
    pub partition_time: Duration,
    pub block: ExecutableBlock,
}

pub struct LedgerUpdateMessage {
    pub current_block_start_time: Instant,
    pub execution_time: Duration,
    pub partition_time: Duration,
    pub block_id: HashValue,
    pub parent_block_id: HashValue,
    pub state_checkpoint_output: StateCheckpointOutput,
    pub first_block_start_time: Instant,
}

/// Message from execution stage to commit stage.
pub struct CommitBlockMessage {
    pub(crate) block_id: HashValue,
    pub(crate) root_hash: HashValue,
    pub(crate) first_block_start_time: Instant,
    pub(crate) current_block_start_time: Instant,
    pub(crate) partition_time: Duration,
    pub(crate) execution_time: Duration,
    pub(crate) num_txns: usize,
}
