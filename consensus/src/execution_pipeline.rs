// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    block_preparer::BlockPreparer,
    counters::{self, log_executor_error_occurred},
    monitor,
    state_computer::{PipelineExecutionResult, StateComputeResultFut},
};
use aptos_consensus_types::block::Block;
use aptos_crypto::HashValue;
use aptos_executor_types::{
    state_checkpoint_output::StateCheckpointOutput, BlockExecutorTrait, ExecutorError,
    ExecutorResult,
};
use aptos_experimental_runtimes::thread_manager::optimal_min_len;
use aptos_logger::{debug, error};
use aptos_types::{
    block_executor::{config::BlockExecutorConfigFromOnchain, partitioner::ExecutableBlock},
    block_metadata_ext::BlockMetadataExt,
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, SignedTransaction,
    },
};
use fail::fail_point;
use once_cell::sync::Lazy;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, oneshot};

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
    pub fn spawn(executor: Arc<dyn BlockExecutorTrait>, runtime: &tokio::runtime::Handle) -> Self {
        let (prepare_block_tx, prepare_block_rx) = mpsc::unbounded_channel();
        let (execute_block_tx, execute_block_rx) = mpsc::unbounded_channel();
        let (ledger_apply_tx, ledger_apply_rx) = mpsc::unbounded_channel();

        runtime.spawn(Self::prepare_block_stage(
            prepare_block_rx,
            execute_block_tx,
        ));
        runtime.spawn(Self::execute_stage(
            execute_block_rx,
            ledger_apply_tx,
            executor.clone(),
        ));
        runtime.spawn(Self::ledger_apply_stage(ledger_apply_rx, executor));
        Self { prepare_block_tx }
    }

    pub async fn queue(
        &self,
        block: Block,
        metadata: BlockMetadataExt,
        parent_block_id: HashValue,
        txn_generator: BlockPreparer,
        block_executor_onchain_config: BlockExecutorConfigFromOnchain,
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
            result_tx,
        } = command;

        debug!("prepare_block received block {}.", block.id());
        let input_txns = block_preparer.prepare_block(&block).await;
        if let Err(e) = input_txns {
            result_tx.send(Err(e)).unwrap_or_else(|value| {
                process_failed_to_send_result(value, block.id(), "prepare")
            });
            return;
        }
        let validator_txns = block.validator_txns().cloned().unwrap_or_default();
        let input_txns = input_txns.expect("input_txns must be Some.");
        tokio::task::spawn_blocking(move || {
            let txns_to_execute =
                Block::combine_to_input_transactions(validator_txns, input_txns.clone(), metadata);
            let sig_verified_txns: Vec<SignatureVerifiedTransaction> =
                SIG_VERIFY_POOL.install(|| {
                    let num_txns = txns_to_execute.len();
                    txns_to_execute
                        .into_par_iter()
                        .with_min_len(optimal_min_len(num_txns, 32))
                        .map(|t| t.into())
                        .collect::<Vec<_>>()
                });
            execute_block_tx
                .send(ExecuteBlockCommand {
                    input_txns,
                    block: (block.id(), sig_verified_txns).into(),
                    parent_block_id,
                    block_executor_onchain_config,
                    result_tx,
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
            result_tx,
        }) = block_rx.recv().await
        {
            let block_id = block.block_id;
            debug!("execute_stage received block {}.", block_id);
            let executor = executor.clone();
            let state_checkpoint_output = monitor!(
                "execute_block",
                tokio::task::spawn_blocking(move || {
                    fail_point!("consensus::compute", |_| {
                        Err(ExecutorError::InternalError {
                            error: "Injected error in compute".into(),
                        })
                    });
                    let start = Instant::now();
                    executor
                        .execute_and_state_checkpoint(
                            block,
                            parent_block_id,
                            block_executor_onchain_config,
                        )
                        .map(|output| (output, start.elapsed()))
                })
                .await
            )
            .expect("Failed to spawn_blocking.");

            ledger_apply_tx
                .send(LedgerApplyCommand {
                    input_txns,
                    block_id,
                    parent_block_id,
                    state_checkpoint_output,
                    result_tx,
                })
                .expect("Failed to send block to ledger_apply stage.");
        }
        debug!("execute_stage quitting.");
    }

    async fn ledger_apply_stage(
        mut block_rx: mpsc::UnboundedReceiver<LedgerApplyCommand>,
        executor: Arc<dyn BlockExecutorTrait>,
    ) {
        while let Some(LedgerApplyCommand {
            input_txns,
            block_id,
            parent_block_id,
            state_checkpoint_output: execution_result,
            result_tx,
        }) = block_rx.recv().await
        {
            debug!("ledger_apply stage received block {}.", block_id);
            let res = async {
                let (state_checkpoint_output, execution_duration) = execution_result?;
                let executor = executor.clone();
                monitor!(
                    "ledger_apply",
                    tokio::task::spawn_blocking(move || {
                        executor.ledger_update(block_id, parent_block_id, state_checkpoint_output)
                    })
                )
                .await
                .expect("Failed to spawn_blocking().")
                .map(|output| (output, execution_duration))
            }
            .await;
            let pipe_line_res = res.map(|(output, execution_duration)| {
                PipelineExecutionResult::new(input_txns, output, execution_duration)
            });
            result_tx.send(pipe_line_res).unwrap_or_else(|value| {
                process_failed_to_send_result(value, block_id, "ledger_apply")
            });
        }
        debug!("ledger_apply stage quitting.");
    }
}

struct PrepareBlockCommand {
    block: Block,
    metadata: BlockMetadataExt,
    block_executor_onchain_config: BlockExecutorConfigFromOnchain,
    // The parent block id.
    parent_block_id: HashValue,
    block_preparer: BlockPreparer,
    result_tx: oneshot::Sender<ExecutorResult<PipelineExecutionResult>>,
}

struct ExecuteBlockCommand {
    input_txns: Vec<SignedTransaction>,
    block: ExecutableBlock,
    parent_block_id: HashValue,
    block_executor_onchain_config: BlockExecutorConfigFromOnchain,
    result_tx: oneshot::Sender<ExecutorResult<PipelineExecutionResult>>,
}

struct LedgerApplyCommand {
    input_txns: Vec<SignedTransaction>,
    block_id: HashValue,
    parent_block_id: HashValue,
    state_checkpoint_output: ExecutorResult<(StateCheckpointOutput, Duration)>,
    result_tx: oneshot::Sender<ExecutorResult<PipelineExecutionResult>>,
}

fn process_failed_to_send_result(
    value: Result<PipelineExecutionResult, ExecutorError>,
    block_id: HashValue,
    from_stage: &str,
) {
    error!(
        block_id = block_id,
        is_err = value.is_err(),
        "Failed to send back execution result from {from_stage} stage",
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
