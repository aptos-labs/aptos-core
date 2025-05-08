// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    block_preparer::BlockPreparer,
    counters::{self, log_executor_error_occurred},
    monitor,
    pipeline::pipeline_phase::CountedRequest,
    state_computer::StateComputeResultFut,
    transaction_shuffler::TransactionShuffler,
};
use aptos_consensus_types::{
    block::Block, pipeline_execution_result::PipelineExecutionResult, quorum_cert::QuorumCert,
};
use aptos_crypto::HashValue;
use aptos_executor_types::{
    state_compute_result::StateComputeResult, BlockExecutorTrait, ExecutorError, ExecutorResult,
};
use aptos_experimental_runtimes::thread_manager::optimal_min_len;
use aptos_logger::{debug, warn};
use aptos_types::{
    block_executor::{config::BlockExecutorConfigFromOnchain, partitioner::ExecutableBlock},
    block_metadata_ext::BlockMetadataExt,
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, SignedTransaction,
    },
};
use fail::fail_point;
use futures::{future::BoxFuture, FutureExt};
use once_cell::sync::Lazy;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use std::{
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
        block: Block,
        metadata: BlockMetadataExt,
        parent_block_id: HashValue,
        block_qc: Option<Arc<QuorumCert>>,
        txn_generator: BlockPreparer,
        block_executor_onchain_config: BlockExecutorConfigFromOnchain,
        pre_commit_hook: PreCommitHook,
        lifetime_guard: CountedRequest<()>,
        shuffler: Arc<dyn TransactionShuffler>,
    ) -> StateComputeResultFut {
        let (result_tx, result_rx) = oneshot::channel();
        let block_id = block.id();
        self.prepare_block_tx
            .send(PrepareBlockCommand {
                block,
                metadata,
                block_executor_onchain_config,
                parent_block_id,
                block_preparer: txn_generator,
                result_tx,
                command_creation_time: Instant::now(),
                pre_commit_hook,
                lifetime_guard,
                block_qc,
                shuffler,
            })
            .expect("Failed to send block to execution pipeline.");

        Box::pin(async move {
            result_rx
                .await
                .map_err(|err| ExecutorError::InternalError {
                    error: format!(
                        "Failed to receive execution result for block {}: {:?}.",
                        block_id, err
                    ),
                })?
        })
    }

    async fn prepare_block(
        execute_block_tx: mpsc::UnboundedSender<ExecuteBlockCommand>,
        command: PrepareBlockCommand,
    ) {
        let PrepareBlockCommand {
            block,
            metadata,
            block_executor_onchain_config,
            parent_block_id,
            block_preparer,
            pre_commit_hook,
            result_tx,
            command_creation_time,
            lifetime_guard,
            block_qc,
            shuffler,
        } = command;
        counters::PREPARE_BLOCK_WAIT_TIME.observe_duration(command_creation_time.elapsed());
        debug!("prepare_block received block {}.", block.id());
        let prepare_block_result = block_preparer
            .prepare_block(&block, async { block_qc }.shared())
            .await;
        if let Err(e) = prepare_block_result {
            result_tx
                .send(Err(e))
                .unwrap_or_else(log_failed_to_send_result("prepare_block", block.id()));
            return;
        }
        let validator_txns = block.validator_txns().cloned().unwrap_or_default();
        let (input_txns, block_gas_limit) =
            prepare_block_result.expect("prepare_block must return Ok");
        let block_executor_onchain_config =
            block_executor_onchain_config.with_block_gas_limit_override(block_gas_limit);
        tokio::task::spawn_blocking(move || {
            let txns_to_execute = Block::combine_to_input_transactions(
                validator_txns,
                input_txns.clone(),
                metadata,
                block_executor_onchain_config.clone(),
            );
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
            execute_block_tx
                .send(ExecuteBlockCommand {
                    input_txns,
                    block: (block.id(), sig_verified_txns).into(),
                    parent_block_id,
                    block_executor_onchain_config,
                    pre_commit_hook,
                    result_tx,
                    command_creation_time: Instant::now(),
                    lifetime_guard,
                    shuffler,
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
        while let Some(ExecuteBlockCommand {
            input_txns,
            block,
            parent_block_id,
            block_executor_onchain_config,
            pre_commit_hook,
            result_tx,
            command_creation_time,
            lifetime_guard,
            shuffler: _,
        }) = block_rx.recv().await
        {
            counters::EXECUTE_BLOCK_WAIT_TIME.observe_duration(command_creation_time.elapsed());
            let block_id = block.block_id;
            debug!("execute_stage received block {}.", block_id);
            let executor = executor.clone();
            let execution_time = monitor!(
                "execute_block",
                tokio::task::spawn_blocking(move || {
                    fail_point!("consensus::compute", |_| {
                        Err(ExecutorError::InternalError {
                            error: "Injected error in compute".into(),
                        })
                    });
                    let start = Instant::now();
                    executor
                        .execute_and_update_state(
                            block,
                            parent_block_id,
                            block_executor_onchain_config,
                        )
                        .map(|_| start.elapsed())
                })
                .await
            )
            .expect("Failed to spawn_blocking.");

            ledger_apply_tx
                .send(LedgerApplyCommand {
                    input_txns,
                    block_id,
                    parent_block_id,
                    execution_time,
                    pre_commit_hook,
                    result_tx,
                    command_creation_time: Instant::now(),
                    lifetime_guard,
                })
                .expect("Failed to send block to ledger_apply stage.");
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
            debug!("ledger_apply stage received block {}.", block_id);
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
    block: Block,
    metadata: BlockMetadataExt,
    block_executor_onchain_config: BlockExecutorConfigFromOnchain,
    // The parent block id.
    parent_block_id: HashValue,
    block_preparer: BlockPreparer,
    pre_commit_hook: PreCommitHook,
    result_tx: oneshot::Sender<ExecutorResult<PipelineExecutionResult>>,
    command_creation_time: Instant,
    lifetime_guard: CountedRequest<()>,
    block_qc: Option<Arc<QuorumCert>>,
    shuffler: Arc<dyn TransactionShuffler>,
}

struct ExecuteBlockCommand {
    input_txns: Vec<SignedTransaction>,
    block: ExecutableBlock,
    parent_block_id: HashValue,
    block_executor_onchain_config: BlockExecutorConfigFromOnchain,
    pre_commit_hook: PreCommitHook,
    result_tx: oneshot::Sender<ExecutorResult<PipelineExecutionResult>>,
    command_creation_time: Instant,
    lifetime_guard: CountedRequest<()>,
    #[allow(dead_code)]
    shuffler: Arc<dyn TransactionShuffler>,
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
                false,
            );
        }
    }
}
