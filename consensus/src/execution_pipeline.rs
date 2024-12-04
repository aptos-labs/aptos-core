// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    block_preparer::BlockPreparer,
    counters::{self, log_executor_error_occurred},
    monitor,
    pipeline::pipeline_phase::CountedRequest,
    state_computer::StateComputeResultFut,
};
use aptos_consensus_types::{
    block::Block, pipeline_execution_result::PipelineExecutionResult,
    pipelined_block::PipelinedBlock,
};
use aptos_crypto::HashValue;
use aptos_executor_types::{
    state_compute_result::StateComputeResult, BlockExecutorTrait, ExecutorError, ExecutorResult,
};
use aptos_experimental_runtimes::thread_manager::optimal_min_len;
use aptos_logger::{debug, info, warn};
use aptos_types::{
    block_executor::{
        config::BlockExecutorConfigFromOnchain,
        partitioner::{ExecutableBlock, ExecutableTransactions},
    },
    block_metadata_ext::BlockMetadataExt,
    transaction::{
        signature_verified_transaction::{
            SignatureVerifiedTransaction,
            SignatureVerifiedTransaction::{Invalid, Valid},
        },
        SignedTransaction,
        Transaction::UserTransaction,
        TransactionStatus,
    },
    txn_provider::{blocking_txn_provider::BlockingTxnProvider, TxnIndex, TxnProvider},
};
use fail::fail_point;
use futures::future::BoxFuture;
use itertools::Itertools;
use once_cell::sync::Lazy;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use std::{
    collections::HashSet,
    iter::zip,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, oneshot};

/// Smallest number of transactions Rayon should put into a single worker task.
/// Same as in execution/executor-benchmark/src/block_preparation.rs
pub const SIG_VERIFY_RAYON_MIN_THRESHOLD: usize = 32;

pub type PreCommitHook =
    Box<dyn 'static + FnOnce(&StateComputeResult) -> BoxFuture<'static, ()> + Send>;

#[allow(clippy::unwrap_used)]
pub static SIG_VERIFY_POOL: Lazy<Arc<rayon::ThreadPool>> = Lazy::new(|| {
    Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(8) // More than 8 threads doesn't seem to help much
            .thread_name(|index| format!("signature-checker-{}", index))
            .build()
            .unwrap(),
    )
});

pub struct ExecutionPipeline {
    prepare_block_tx: mpsc::UnboundedSender<PrepareBlockCommand>,
}

impl ExecutionPipeline {
    pub fn spawn(
        executor: Arc<dyn BlockExecutorTrait>,
        runtime: &tokio::runtime::Handle,
        enable_pre_commit: bool,
    ) -> Self {
        let (prepare_block_tx, prepare_block_rx) = mpsc::unbounded_channel();
        let (execute_block_tx, execute_block_rx) = mpsc::unbounded_channel();
        let (ledger_apply_tx, ledger_apply_rx) = mpsc::unbounded_channel();
        let (pre_commit_tx, pre_commit_rx) = mpsc::unbounded_channel();

        runtime.spawn(Self::prepare_block_stage(
            prepare_block_rx,
            execute_block_tx,
        ));
        runtime.spawn(Self::execute_stage(
            execute_block_rx,
            ledger_apply_tx,
            executor.clone(),
        ));
        runtime.spawn(Self::ledger_apply_stage(
            ledger_apply_rx,
            pre_commit_tx,
            executor.clone(),
            enable_pre_commit,
        ));
        runtime.spawn(Self::pre_commit_stage(pre_commit_rx, executor));

        Self { prepare_block_tx }
    }

    pub async fn queue(
        &self,
        block: PipelinedBlock,
        max_block_txns: u64,
        metadata: BlockMetadataExt,
        parent_block_id: HashValue,
        txn_generator: BlockPreparer,
        block_executor_onchain_config: BlockExecutorConfigFromOnchain,
        pre_commit_hook: PreCommitHook,
        lifetime_guard: CountedRequest<()>,
    ) -> StateComputeResultFut {
        let (result_tx, result_rx) = oneshot::channel();
        let block_round = block.round();
        let block_id = block.id();
        self.prepare_block_tx
            .send(PrepareBlockCommand {
                block,
                max_block_txns,
                metadata,
                block_executor_onchain_config,
                parent_block_id,
                block_preparer: txn_generator,
                result_tx,
                command_creation_time: Instant::now(),
                pre_commit_hook,
                lifetime_guard,
            })
            .expect("Failed to send block to execution pipeline.");

        Box::pin(async move {
            let result = result_rx
                .await
                .map_err(|err| ExecutorError::InternalError {
                    error: format!(
                        "Failed to receive execution result for block {}: {:?}.",
                        block_id, err
                    ),
                })?;
            info!(
                "received result_rx for round {} block {}.",
                block_round, block_id
            );
            result
        })
    }

    async fn prepare_block(
        execute_block_tx: mpsc::UnboundedSender<ExecuteBlockCommand>,
        command: PrepareBlockCommand,
    ) {
        let PrepareBlockCommand {
            block,
            max_block_txns,
            metadata,
            mut block_executor_onchain_config,
            parent_block_id,
            block_preparer,
            pre_commit_hook,
            result_tx,
            command_creation_time,
            lifetime_guard,
        } = command;
        counters::PREPARE_BLOCK_WAIT_TIME.observe_duration(command_creation_time.elapsed());
        debug!("prepare_block received block {}.", block.id());
        let prepare_result = block_preparer
            .prepare_block(block.block(), block.block_window())
            .await;
        if let Err(e) = prepare_result {
            result_tx
                .send(Err(e))
                .unwrap_or_else(log_failed_to_send_result("prepare_block", block.id()));
            return;
        }
        let validator_txns = block.validator_txns().cloned().unwrap_or_default();
        let (input_txns, max_txns_to_execute, block_gas_limit) =
            prepare_result.expect("input_txns must be Some.");
        block_executor_onchain_config.block_gas_limit_override = block_gas_limit;
        tokio::task::spawn_blocking(move || {
            let txns_to_execute =
                Block::combine_to_input_transactions(validator_txns, input_txns.clone(), metadata);
            let sig_verification_start = Instant::now();
            let sig_verified_txns: Vec<SignatureVerifiedTransaction> =
                SIG_VERIFY_POOL.install(|| {
                    let num_txns = txns_to_execute.len();
                    txns_to_execute
                        .into_par_iter()
                        .with_min_len(optimal_min_len(num_txns, SIG_VERIFY_RAYON_MIN_THRESHOLD))
                        .map(|t| t.into())
                        .collect::<Vec<_>>()
                });
            counters::PREPARE_BLOCK_SIG_VERIFICATION_TIME
                .observe_duration(sig_verification_start.elapsed());
            let block_id = block.id();
            execute_block_tx
                .send(ExecuteBlockCommand {
                    input_txns,
                    max_block_txns,
                    max_txns_to_execute,
                    pipelined_block: block,
                    block: (block_id, sig_verified_txns).into(),
                    parent_block_id,
                    block_executor_onchain_config,
                    pre_commit_hook,
                    result_tx,
                    command_creation_time: Instant::now(),
                    lifetime_guard,
                })
                .expect("Failed to send block to execution pipeline.");
        })
        .await
        .expect("Failed to spawn_blocking.");
    }

    async fn prepare_block_stage(
        mut prepare_block_rx: mpsc::UnboundedReceiver<PrepareBlockCommand>,
        execute_block_tx: mpsc::UnboundedSender<ExecuteBlockCommand>,
    ) {
        while let Some(command) = prepare_block_rx.recv().await {
            monitor!(
                "prepare_block",
                Self::prepare_block(execute_block_tx.clone(), command).await
            );
        }
        debug!("prepare_block_stage quitting.");
    }

    async fn execute_stage(
        mut block_rx: mpsc::UnboundedReceiver<ExecuteBlockCommand>,
        ledger_apply_tx: mpsc::UnboundedSender<LedgerApplyCommand>,
        executor: Arc<dyn BlockExecutorTrait>,
    ) {
        'outer: while let Some(ExecuteBlockCommand {
            input_txns: _,
            max_block_txns,
            max_txns_to_execute,
            pipelined_block,
            block,
            parent_block_id,
            block_executor_onchain_config,
            pre_commit_hook,
            result_tx,
            command_creation_time,
            lifetime_guard,
        }) = block_rx.recv().await
        {
            let now = Instant::now();

            counters::EXECUTE_BLOCK_WAIT_TIME.observe_duration(command_creation_time.elapsed());
            let block_id = block.block_id;
            let round = pipelined_block.round();
            info!("execute_stage received block {}.", block_id);

            let mut committed_transactions = HashSet::new();

            // TODO: lots of repeated code here
            monitor!("execute_wait_for_committed_transactions", {
                let num_blocks_in_window = pipelined_block.block_window().pipelined_blocks().len();
                for b in pipelined_block
                    .block_window()
                    .pipelined_blocks()
                    .iter()
                    .skip(num_blocks_in_window.saturating_sub(1))
                {
                    info!(
                        "Execution: Waiting for committed transactions at block {} for block {}",
                        b.round(),
                        pipelined_block.round()
                    );
                    let txn_hashes = b.wait_for_committed_transactions();
                    match txn_hashes {
                        Ok(txn_hashes) => {
                            for txn_hash in txn_hashes.iter() {
                                committed_transactions.insert(*txn_hash);
                            }
                        },
                        Err(e) => {
                            info!(
                                "Execution: Waiting for committed transactions at block {} for block {}: Failed {}",
                                b.round(),
                                pipelined_block.round(),
                                e
                            );
                            // TODO: can't clone, so make the whole thing return an error, then send it after this block of code.
                            result_tx
                                .send(Err(ExecutorError::CouldNotGetCommittedTransactions))
                                .unwrap_or_else(log_failed_to_send_result(
                                    "execute_stage",
                                    block_id,
                                ));
                            continue 'outer;
                        },
                    }
                    info!(
                        "Execution: Waiting for committed transactions at block {} for block {}: Done",
                        b.round(),
                        pipelined_block.round()
                    );
                }
            });

            let mut txns = monitor!("execute_filter_block_committed_transactions", {
                // TODO: Find a better way to do this.
                match block.transactions {
                    ExecutableTransactions::Unsharded(txns) => {
                        let transactions: Vec<_> = txns
                            .into_iter()
                            .filter(|txn| {
                                if let Valid(UserTransaction(user_txn)) = txn {
                                    !committed_transactions.contains(&user_txn.committed_hash())
                                } else {
                                    true
                                }
                            })
                            .collect();
                        transactions
                    },
                    ExecutableTransactions::UnshardedBlocking(_) => {
                        unimplemented!("Not expecting this yet.")
                    },
                    ExecutableTransactions::Sharded(_) => {
                        unimplemented!("Sharded transactions are not supported yet.")
                    },
                }
            });
            let num_validator_txns = if let Some((first_user_txn_idx, _)) =
                txns.iter().find_position(|txn| {
                    let txn = match txn {
                        Valid(txn) => txn,
                        Invalid(txn) => txn,
                    };
                    matches!(txn, UserTransaction(_))
                }) {
                first_user_txn_idx
            } else {
                txns.len()
            };
            let mut num_txns_to_execute = txns.len().min(max_block_txns as usize);
            if let Some(max_user_txns_to_execute) = max_txns_to_execute {
                num_txns_to_execute = num_txns_to_execute
                    .min(num_validator_txns.saturating_add(max_user_txns_to_execute as usize));
            }
            let blocking_txn_provider = BlockingTxnProvider::new(num_txns_to_execute);
            let blocking_txn_writer = blocking_txn_provider.clone();
            let join_shuffle = tokio::task::spawn_blocking(move || {
                // TODO: keep this previously split so we don't have to re-split it here
                if num_txns_to_execute > num_validator_txns {
                    let timer = Instant::now();
                    let validator_txns: Vec<_> = txns.drain(0..num_validator_txns).collect();
                    info!(
                        "Execution: Split validator txns from user txns in {} micros",
                        timer.elapsed().as_micros()
                    );
                    // TODO: we could probably constrain this too with max_txns_to_execute
                    let shuffle_iterator = crate::transaction_shuffler::use_case_aware::iterator::ShuffledTransactionIterator::new(crate::transaction_shuffler::use_case_aware::Config {
                            sender_spread_factor: 32,
                            platform_use_case_spread_factor: 0,
                            user_use_case_spread_factor: 16,
                        }).extended_with(txns);
                    for (idx, txn) in validator_txns
                        .into_iter()
                        .chain(shuffle_iterator)
                        .take(num_txns_to_execute)
                        .enumerate()
                    {
                        blocking_txn_writer.set_txn(idx as TxnIndex, txn);
                    }
                } else {
                    for (idx, txn) in txns.into_iter().take(num_txns_to_execute).enumerate() {
                        blocking_txn_writer.set_txn(idx as TxnIndex, txn);
                    }
                }
            });
            let transactions = ExecutableTransactions::UnshardedBlocking(blocking_txn_provider);
            let transactions_cloned = transactions.clone();
            let block = ExecutableBlock::new(block.block_id, transactions);

            let executor = executor.clone();
            let join_execute = tokio::task::spawn_blocking(move || {
                fail_point!("consensus::compute", |_| {
                    Err(ExecutorError::InternalError {
                        error: "Injected error in compute".into(),
                    })
                });
                let start = Instant::now();
                info!(
                    "execute_and_state_checkpoint start. {}, {}",
                    round, block_id
                );
                executor
                    .execute_and_state_checkpoint(
                        block,
                        parent_block_id,
                        block_executor_onchain_config,
                    )
                    .map(|output| {
                        info!(
                            "execute_and_state_checkpoint end. elapsed = {}ms, {}, {}",
                            start.elapsed().as_millis(),
                            round,
                            block_id
                        );
                        (output, start.elapsed())
                    })
            });

            join_shuffle.await.expect("Failed to join_shuffle.");

            let input_txns = monitor!("execute_filter_input_committed_transactions", {
                let txns_provider_reader = match &transactions_cloned {
                    ExecutableTransactions::UnshardedBlocking(txns) => txns.clone(),
                    ExecutableTransactions::Unsharded(_) => {
                        unreachable!("Should have been converted to UnshardedBlocking")
                    },
                    ExecutableTransactions::Sharded(_) => {
                        unreachable!("Should have been converted to UnshardedBlocking")
                    },
                };
                let mut input_txns = vec![];
                for idx in 0..txns_provider_reader.num_txns() {
                    match txns_provider_reader.get_txn(idx as TxnIndex) {
                        Valid(UserTransaction(user_txn)) => {
                            input_txns.push(user_txn.clone());
                        },
                        Invalid(UserTransaction(user_txn)) => {
                            input_txns.push(user_txn.clone());
                        },
                        _ => {},
                    }
                }
                input_txns
            });

            let state_checkpoint_output =
                monitor!("execute_block", join_execute.await).expect("Failed to join_execute.");

            monitor!("execute_update_committed_transactions", {
                if let Ok((output, _)) = &state_checkpoint_output {
                    // Block metadata + validator transactions
                    let num_system_txns = 1 + pipelined_block
                        .validator_txns()
                        .map_or(0, |txns| txns.len());
                    let committed_transactions: Vec<_> =
                        zip(input_txns.iter(), output.iter().skip(num_system_txns))
                            .filter_map(|(input_txn, txn_status)| {
                                if let TransactionStatus::Keep(_) = txn_status {
                                    Some(input_txn.committed_hash())
                                } else {
                                    None
                                }
                            })
                            .collect();
                    pipelined_block.set_committed_transactions(committed_transactions);
                } else {
                    warn!("Not doing cancel of committed transactions: execute_block failed for block ({},{}) {}.", pipelined_block.epoch(), pipelined_block.round(), block_id);
                    // pipelined_block.cancel_committed_transactions();
                }
            });

            ledger_apply_tx
                .send(LedgerApplyCommand {
                    input_txns,
                    block_id,
                    parent_block_id,
                    execution_time: state_checkpoint_output.map(|(_, time)| time),
                    pre_commit_hook,
                    result_tx,
                    command_creation_time: Instant::now(),
                    lifetime_guard,
                })
                .expect("Failed to send block to ledger_apply stage.");

            info!(
                "execute_stage for block ({},{}) took {} ms",
                pipelined_block.epoch(),
                pipelined_block.round(),
                now.elapsed().as_millis()
            );
        }
        debug!("execute_stage quitting.");
    }

    async fn ledger_apply_stage(
        mut block_rx: mpsc::UnboundedReceiver<LedgerApplyCommand>,
        pre_commit_tx: mpsc::UnboundedSender<PreCommitCommand>,
        executor: Arc<dyn BlockExecutorTrait>,
        enable_pre_commit: bool,
    ) {
        while let Some(LedgerApplyCommand {
            input_txns,
            block_id,
            parent_block_id,
            execution_time,
            pre_commit_hook,
            result_tx,
            command_creation_time,
            lifetime_guard,
        }) = block_rx.recv().await
        {
            counters::APPLY_LEDGER_WAIT_TIME.observe_duration(command_creation_time.elapsed());
            info!("ledger_apply stage received block {}.", block_id);
            let res = async {
                let execution_duration = execution_time?;
                let executor = executor.clone();
                monitor!(
                    "ledger_apply",
                    tokio::task::spawn_blocking(move || {
                        executor.ledger_update(block_id, parent_block_id)
                    })
                    .await
                )
                .expect("Failed to spawn_blocking().")
                .map(|output| (output, execution_duration))
            }
            .await;
            let pipeline_res = res.map(|(output, execution_duration)| {
                let pre_commit_hook_fut = pre_commit_hook(&output);
                let pre_commit_fut: BoxFuture<'static, ExecutorResult<()>> =
                    if output.epoch_state().is_some() || !enable_pre_commit {
                        // hack: it causes issue if pre-commit is finished at an epoch ending, and
                        // we switch to state sync, so we do the pre-commit only after we actually
                        // decide to commit (in the commit phase)
                        let executor = executor.clone();
                        Box::pin(async move {
                            tokio::task::spawn_blocking(move || {
                                executor.pre_commit_block(block_id)
                            })
                            .await
                            .expect("failed to spawn_blocking")?;
                            pre_commit_hook_fut.await;
                            Ok(())
                        })
                    } else {
                        // kick off pre-commit right away
                        let (pre_commit_result_tx, pre_commit_result_rx) = oneshot::channel();
                        // schedule pre-commit
                        pre_commit_tx
                            .send(PreCommitCommand {
                                block_id,
                                pre_commit_hook_fut,
                                result_tx: pre_commit_result_tx,
                                lifetime_guard,
                            })
                            .expect("Failed to send block to pre_commit stage.");
                        Box::pin(async {
                            pre_commit_result_rx
                                .await
                                .map_err(ExecutorError::internal_err)?
                        })
                    };

                PipelineExecutionResult::new(input_txns, output, execution_duration, pre_commit_fut)
            });
            result_tx
                .send(pipeline_res)
                .unwrap_or_else(log_failed_to_send_result("ledger_apply", block_id));
        }
        debug!("ledger_apply stage quitting.");
    }

    async fn pre_commit_stage(
        mut block_rx: mpsc::UnboundedReceiver<PreCommitCommand>,
        executor: Arc<dyn BlockExecutorTrait>,
    ) {
        while let Some(PreCommitCommand {
            block_id,
            pre_commit_hook_fut,
            result_tx,
            lifetime_guard,
        }) = block_rx.recv().await
        {
            debug!("pre_commit stage received block {}.", block_id);
            let res = async {
                let executor = executor.clone();
                monitor!(
                    "pre_commit",
                    tokio::task::spawn_blocking(move || { executor.pre_commit_block(block_id) })
                )
                .await
                .expect("Failed to spawn_blocking().")?;
                pre_commit_hook_fut.await;
                Ok(())
            }
            .await;
            result_tx
                .send(res)
                .unwrap_or_else(log_failed_to_send_result("pre_commit", block_id));
            drop(lifetime_guard);
        }
        debug!("pre_commit stage quitting.");
    }
}

struct PrepareBlockCommand {
    block: PipelinedBlock,
    max_block_txns: u64,
    metadata: BlockMetadataExt,
    block_executor_onchain_config: BlockExecutorConfigFromOnchain,
    // The parent block id.
    parent_block_id: HashValue,
    block_preparer: BlockPreparer,
    pre_commit_hook: PreCommitHook,
    result_tx: oneshot::Sender<ExecutorResult<PipelineExecutionResult>>,
    command_creation_time: Instant,
    lifetime_guard: CountedRequest<()>,
}

struct ExecuteBlockCommand {
    input_txns: Vec<SignedTransaction>,
    max_block_txns: u64,
    max_txns_to_execute: Option<u64>,
    pipelined_block: PipelinedBlock,
    block: ExecutableBlock,
    parent_block_id: HashValue,
    block_executor_onchain_config: BlockExecutorConfigFromOnchain,
    pre_commit_hook: PreCommitHook,
    result_tx: oneshot::Sender<ExecutorResult<PipelineExecutionResult>>,
    command_creation_time: Instant,
    lifetime_guard: CountedRequest<()>,
}

struct LedgerApplyCommand {
    input_txns: Vec<SignedTransaction>,
    block_id: HashValue,
    parent_block_id: HashValue,
    execution_time: ExecutorResult<Duration>,
    pre_commit_hook: PreCommitHook,
    result_tx: oneshot::Sender<ExecutorResult<PipelineExecutionResult>>,
    command_creation_time: Instant,
    lifetime_guard: CountedRequest<()>,
}

struct PreCommitCommand {
    block_id: HashValue,
    pre_commit_hook_fut: BoxFuture<'static, ()>,
    result_tx: oneshot::Sender<ExecutorResult<()>>,
    lifetime_guard: CountedRequest<()>,
}

fn log_failed_to_send_result<T>(
    from_stage: &'static str,
    block_id: HashValue,
) -> impl FnOnce(ExecutorResult<T>) {
    move |value| {
        warn!(
            from_stage = from_stage,
            block_id = block_id,
            is_err = value.is_err(),
            "Failed to send back execution/pre_commit result. (rx dropped)",
        );
        if let Err(e) = value {
            // receive channel discarding error, log for debugging.
            log_executor_error_occurred(
                e,
                &counters::PIPELINE_DISCARDED_EXECUTOR_ERROR_COUNT,
                block_id,
            );
        }
    }
}
