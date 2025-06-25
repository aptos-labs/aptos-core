// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_preparation::BlockPreparationStage,
    ledger_update_stage::{CommitProcessing, LedgerUpdateStage},
    measurements::{EventMeasurements, OverallMeasuring},
    metrics::NUM_TXNS,
    OverallMeasurement, TransactionCommitter, TransactionExecutor,
};
use aptos_block_partitioner::v2::config::PartitionerV2Config;
use aptos_consensus::scheduled_txns_handler::ScheduledTxnsHandler;
use aptos_crypto::HashValue;
use aptos_executor::block_executor::BlockExecutor;
use aptos_executor_types::{state_compute_result::StateComputeResult, BlockExecutorTrait};
use aptos_infallible::Mutex;
use aptos_logger::info;
use aptos_types::{
    block_executor::partitioner::ExecutableBlock,
    block_metadata::BlockMetadata,
    transaction::{
        scheduled_txn::SCHEDULED_TRANSACTIONS_MODULE_INFO, Transaction, TransactionPayload, Version,
    },
};
use aptos_vm::{AptosVM, VMBlockExecutor};
use derivative::Derivative;
use move_core_types::{
    language_storage::StructTag,
    value::{serialize_values, MoveValue},
};
use std::{
    collections::{BTreeMap, HashMap},
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
    pub generate_then_execute: bool,
    pub split_stages: bool,
    pub skip_commit: bool,
    pub allow_aborts: bool,
    pub allow_discards: bool,
    pub allow_retries: bool,
    #[derivative(Default(value = "0"))]
    pub num_executor_shards: usize,
    #[derivative(Default(value = "4"))]
    pub num_generator_workers: usize,
    pub partitioner_config: PartitionerV2Config,
    #[derivative(Default(value = "8"))]
    pub num_sig_verify_threads: usize,
    pub run_scheduled_txns: bool,
    #[derivative(Default(value = "1000"))]
    pub ready_sched_txns_limit: usize,

    pub print_transactions: bool,
}

pub struct Pipeline<V> {
    join_handles: Vec<JoinHandle<u64>>,
    phantom: PhantomData<V>,
    start_pipeline_tx: Option<SyncSender<()>>,
    staged_result: Arc<Mutex<Vec<OverallMeasurement>>>,
    staged_events: Arc<Mutex<BTreeMap<(usize, StructTag), usize>>>,
}

impl<V> Pipeline<V>
where
    V: VMBlockExecutor + 'static,
{
    pub fn new(
        executor: BlockExecutor<V>,
        start_version: Version,
        config: &PipelineConfig,
        // Need to specify num blocks, to size queues correctly, when delay_execution_start, split_stages or skip_commit are used
        num_blocks: Option<usize>,
    ) -> (Self, SyncSender<Vec<Transaction>>) {
        let parent_block_id = executor.committed_block_id();
        let run_scheduled_txns = config.run_scheduled_txns;
        let ready_sched_txns_limit = config.ready_sched_txns_limit;
        let executor_1 = Arc::new(executor);
        let executor_2 = executor_1.clone();
        let executor_3 = executor_1.clone();
        let executor_4 = executor_1.clone();

        let (raw_block_sender, raw_block_receiver) = mpsc::sync_channel::<Vec<Transaction>>(
            if config.generate_then_execute {
                (num_blocks.unwrap() + 1).max(50)
            } else {
                10
            }, /* bound */
        );

        let (executable_block_sender, executable_block_receiver) =
            mpsc::sync_channel::<ExecuteBlockMessage>(
                if config.split_stages {
                    (num_blocks.unwrap() + 1).max(50)
                } else {
                    10
                }, /* bound */
            );

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

        let (start_pipeline_tx, start_pipeline_rx) =
            create_start_tx_rx(config.generate_then_execute);
        let (start_execution_tx, start_execution_rx) = create_start_tx_rx(config.split_stages);
        let (start_ledger_update_tx, start_ledger_update_rx) =
            create_start_tx_rx(config.split_stages);
        let (start_commit_tx, start_commit_rx) = create_start_tx_rx(config.split_stages);

        let mut join_handles = vec![];

        // signature verification and partitioning
        let mut preparation_stage = BlockPreparationStage::new(
            std::cmp::min(config.num_sig_verify_threads, num_cpus::get()),
            // Assume the distributed executor and the distributed partitioner share the same worker set.
            config.num_executor_shards,
            &config.partitioner_config,
        );

        let mut prepare_ready_txns = BlockPreparationStage::new(
            std::cmp::min(config.num_sig_verify_threads, num_cpus::get()),
            // Assume the distributed executor and the distributed partitioner share the same worker set.
            config.num_executor_shards,
            &config.partitioner_config,
        );

        let mut exe = TransactionExecutor::new(executor_1, parent_block_id, ledger_update_sender);

        let commit_processing = if config.skip_commit {
            CommitProcessing::Skip
        } else {
            CommitProcessing::SendToQueue(commit_sender)
        };

        let staged_events = Arc::new(Mutex::new(BTreeMap::new()));
        let staged_events_clone = staged_events.clone();

        let mut ledger_update_stage = LedgerUpdateStage::new(
            executor_2,
            commit_processing,
            config.allow_aborts,
            config.allow_discards,
            config.allow_retries,
            staged_events_clone,
        );

        let print_transactions = config.print_transactions;
        let staged_result = Arc::new(Mutex::new(Vec::new()));
        let staged_result_clone = staged_result.clone();

        let preparation_thread = std::thread::Builder::new()
            .name("block_preparation".to_string())
            .spawn(move || {
                start_pipeline_rx.map(|rx| rx.recv());
                let mut processed = 0;
                while let Ok(txns) = raw_block_receiver.recv() {
                    processed += txns.len() as u64;
                    if print_transactions {
                        println!("Transactions:");
                        for txn in &txns {
                            println!("{:?}", txn);
                        }
                    }
                    let exe_block_msg = preparation_stage.process(txns);
                    executable_block_sender.send(exe_block_msg).unwrap();
                }
                info!("Done preparation");
                start_execution_tx.map(|tx| tx.send(()));
                processed
            })
            .expect("Failed to spawn block partitioner thread.");
        join_handles.push(preparation_thread);

        fn create_mock_meta_data_txn(block_id: HashValue) -> BlockMetadata {
            BlockMetadata::new(
                block_id,
                0,
                0,
                aptos_types::account_address::AccountAddress::ZERO,
                vec![],
                vec![],
                0,
            )
        }

        fn run_scheduled_transactions<V: VMBlockExecutor>(
            total_scheduled_txns: u64,
            parent_block_id: HashValue,
            prepare_ready_txns: &mut BlockPreparationStage,
            exe: &mut TransactionExecutor<V>,
            executor: &BlockExecutor<V>,
            ready_sched_txns_limit: usize,
        ) -> u64 {
            info!(
                "Running scheduled transactions: {} total scheduled transactions",
                total_scheduled_txns
            );

            let mut total_executed = 0;
            let mut stage_index = 0;
            let mut current_parent_id = parent_block_id;
            // Note: This should be the time set in args.rs while scheduling txns, otherwise txns
            //       can expire
            let mock_block_time_ms = 4_000_000_000_000;

            let overall_measuring = OverallMeasuring::start();

            while total_executed < total_scheduled_txns {
                let state_view = executor.state_view(current_parent_id).unwrap();
                let args = vec![
                    MoveValue::U64(mock_block_time_ms + 1000),
                    MoveValue::U64(ready_sched_txns_limit as u64),
                ];
                let res = AptosVM::execute_function(
                    &state_view,
                    &SCHEDULED_TRANSACTIONS_MODULE_INFO.module_id(),
                    &SCHEDULED_TRANSACTIONS_MODULE_INFO.get_ready_transactions_with_limit_name,
                    vec![],
                    serialize_values(&args),
                );
                let mut current_block_txns = ScheduledTxnsHandler::handle_ready_txns_result(res)
                    .into_iter()
                    .map(Transaction::ScheduledTransaction)
                    .collect::<Vec<Transaction>>();

                if current_block_txns.is_empty() {
                    break; // No more ready transactions available
                }

                // append a mock metadata transaction to the block
                if !current_block_txns.is_empty() {
                    let block_metadata_txn =
                        Transaction::BlockMetadata(create_mock_meta_data_txn(current_parent_id));
                    current_block_txns.push(block_metadata_txn);
                }

                if !current_block_txns.is_empty() {
                    let ExecuteBlockMessage {
                        current_block_start_time,
                        partition_time,
                        block,
                    } = prepare_ready_txns.process(current_block_txns);

                    current_parent_id = block.block_id;
                    let block_size = block.transactions.num_transactions() as u64;
                    total_executed += block_size;
                    exe.execute_block(
                        current_block_start_time,
                        partition_time,
                        block,
                        stage_index,
                        "run_scheduled",
                    );
                    stage_index += 1;

                    info!(
                        "Executed block {} with {} transactions. Total executed: {}/{}",
                        stage_index, block_size, total_executed, total_scheduled_txns
                    );
                } else {
                    info!("No more ready transactions available");
                    break;
                }
            }

            if total_executed > 0 {
                overall_measuring
                    .elapsed(
                        "Overall scheduled txns execution".to_string(),
                        format!("Executed {} transactions", total_executed),
                        total_executed,
                    )
                    .print_end();
            } else {
                info!("No scheduled transactions executed");
            }
            total_executed
        }

        let exe_thread = std::thread::Builder::new()
            .name("txn_executor".to_string())
            .spawn(move || {
                start_execution_rx.map(|rx| rx.recv());
                let overall_measuring = OverallMeasuring::start();
                let mut executed = 0;
                let mut last_block_id = HashValue::zero();

                let mut stage_index = 0;
                let mut stage_overall_measuring = overall_measuring.clone();
                let mut stage_executed = 0;
                let mut stage_txn_occurences: HashMap<String, usize> = HashMap::new();

                while let Ok(msg) = executable_block_receiver.recv() {
                    let ExecuteBlockMessage {
                        current_block_start_time,
                        partition_time,
                        block,
                    } = msg;
                    let block_size = block.transactions.num_transactions() as u64;
                    last_block_id = block.block_id;
                    for txn in block.transactions.txns() {
                        if let Some(txn) = txn.borrow_into_inner().try_as_signed_user_txn() {
                            if let TransactionPayload::EntryFunction(entry) = txn.payload() {
                                *stage_txn_occurences
                                    .entry(format!(
                                        "{}::{}",
                                        entry.module().name(),
                                        entry.function()
                                    ))
                                    .or_insert(0) += 1;
                            }
                        }
                    }

                    NUM_TXNS
                        .with_label_values(&["execution"])
                        .inc_by(block_size);
                    info!("Received block of size {:?} to execute", block_size);
                    executed += block_size;
                    stage_executed += block_size;
                    exe.execute_block(
                        current_block_start_time,
                        partition_time,
                        block,
                        stage_index,
                        "execute",
                    );
                    info!("Finished executing block");

                    // Empty blocks indicate the end of a stage.
                    // Print the accumulated stage stats at that point.
                    if block_size == 0 {
                        if stage_executed > 0 {
                            info!("Execution finished stage {}", stage_index);
                            let stage_measurement = stage_overall_measuring.elapsed(
                                format!("Staged execution: stage {}:", stage_index),
                                format!("{:?}", stage_txn_occurences),
                                stage_executed,
                            );

                            stage_measurement.print_end();
                            staged_result_clone.lock().push(stage_measurement);
                        }
                        stage_index += 1;
                        stage_overall_measuring = OverallMeasuring::start();
                        stage_executed = 0;
                        stage_txn_occurences = HashMap::new();
                    }
                }

                if stage_index > 0 && stage_executed > 0 {
                    info!("Execution finished stage {}", stage_index);
                    let stage_measurement = stage_overall_measuring.elapsed(
                        format!("Staged execution: stage {}:", stage_index),
                        format!("{:?}", stage_txn_occurences),
                        stage_executed,
                    );
                    stage_measurement.print_end();
                    staged_result_clone.lock().push(stage_measurement);
                }

                if num_blocks.is_some() {
                    overall_measuring
                        .elapsed(
                            "Overall execution".to_string(),
                            if stage_index == 0 {
                                format!("{:?}", stage_txn_occurences)
                            } else {
                                "across all stages".to_string()
                            },
                            executed,
                        )
                        .print_end();
                }

                if run_scheduled_txns {
                    run_scheduled_transactions(
                        executed,
                        last_block_id,
                        &mut prepare_ready_txns,
                        &mut exe,
                        executor_4.as_ref(),
                        ready_sched_txns_limit,
                    );
                }
                start_ledger_update_tx.map(|tx| tx.send(()));
                executed
            })
            .expect("Failed to spawn transaction executor thread.");
        join_handles.push(exe_thread);

        let ledger_update_thread = std::thread::Builder::new()
            .name("ledger_update".to_string())
            .spawn(move || {
                start_ledger_update_rx.map(|rx| rx.recv());

                while let Ok(ledger_update_msg) = ledger_update_receiver.recv() {
                    NUM_TXNS
                        .with_label_values(&["ledger_update"])
                        .inc_by(ledger_update_msg.num_input_txns as u64);
                    ledger_update_stage.ledger_update(ledger_update_msg);
                }
                start_commit_tx.map(|tx| tx.send(()));

                0
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
                        TransactionCommitter::new(executor_3, start_version, commit_receiver);
                    committer.run();

                    0
                })
                .expect("Failed to spawn transaction committer thread.");
            join_handles.push(commit_thread);
        }

        (
            Self {
                join_handles,
                phantom: PhantomData,
                start_pipeline_tx,
                staged_result,
                staged_events,
            },
            raw_block_sender,
        )
    }

    pub fn start_pipeline_processing(&self) {
        self.start_pipeline_tx.as_ref().map(|tx| tx.send(()));
    }

    pub fn join(self) -> (Option<u64>, Vec<OverallMeasurement>, EventMeasurements) {
        let mut counts = vec![];
        for handle in self.join_handles {
            let count = handle.join().unwrap();
            if count > 0 {
                counts.push(count);
            }
        }
        (
            counts.into_iter().min(),
            Arc::try_unwrap(self.staged_result).unwrap().into_inner(),
            EventMeasurements {
                staged_events: Arc::try_unwrap(self.staged_events).unwrap().into_inner(),
            },
        )
    }
}

fn create_start_tx_rx(should_wait: bool) -> (Option<SyncSender<()>>, Option<mpsc::Receiver<()>>) {
    let (start_tx, start_rx) = if should_wait {
        let (start_tx, start_rx) = mpsc::sync_channel::<()>(1);
        (Some(start_tx), Some(start_rx))
    } else {
        (None, None)
    };
    (start_tx, start_rx)
}

/// Message from partitioning stage to execution stage.
pub struct ExecuteBlockMessage {
    pub current_block_start_time: Instant,
    pub partition_time: Duration,
    pub block: ExecutableBlock,
}

pub struct LedgerUpdateMessage {
    pub first_block_start_time: Instant,
    pub current_block_start_time: Instant,
    pub execution_time: Duration,
    pub partition_time: Duration,
    pub block_id: HashValue,
    pub parent_block_id: HashValue,
    pub num_input_txns: usize,
    pub stage: usize,
}

/// Message from execution stage to commit stage.
pub struct CommitBlockMessage {
    pub(crate) block_id: HashValue,
    pub(crate) first_block_start_time: Instant,
    pub(crate) current_block_start_time: Instant,
    pub(crate) execution_time: Duration,
    pub(crate) partition_time: Duration,
    pub(crate) output: StateComputeResult,
}
