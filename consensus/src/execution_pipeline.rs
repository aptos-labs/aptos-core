// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{monitor, state_computer::StateComputeResultFut};
use aptos_crypto::HashValue;
use aptos_executor_types::{
    state_checkpoint_output::StateCheckpointOutput, BlockExecutorTrait, ExecutorError,
    ExecutorResult, StateComputeResult,
};
use aptos_logger::{debug, error};
use aptos_types::{
    block_executor::partitioner::ExecutableBlock,
    transaction::{into_signature_verified, SignatureVerifiedTransaction, Transaction},
};
use fail::fail_point;
use once_cell::sync::Lazy;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
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
    execute_block_tx: mpsc::UnboundedSender<ExecuteBlockCommand>,
}

impl ExecutionPipeline {
    pub fn spawn(executor: Arc<dyn BlockExecutorTrait>, runtime: &tokio::runtime::Handle) -> Self {
        let (prepare_block_tx, prepare_block_rx) = mpsc::unbounded_channel();
        let (execute_block_tx, execute_block_rx) = mpsc::unbounded_channel();
        let (ledger_apply_tx, ledger_apply_rx) = mpsc::unbounded_channel();
        runtime.spawn(Self::prepare_block(
            prepare_block_rx,
            execute_block_tx.clone(),
        ));
        runtime.spawn(Self::execute_stage(
            execute_block_rx,
            ledger_apply_tx,
            executor.clone(),
        ));
        runtime.spawn(Self::ledger_apply_stage(ledger_apply_rx, executor));
        Self {
            prepare_block_tx,
            execute_block_tx,
        }
    }

    pub async fn queue(
        &self,
        block_id: HashValue,
        parent_block_id: HashValue,
        txns_to_execute: Vec<Transaction>,
        maybe_block_gas_limit: Option<u64>,
    ) -> StateComputeResultFut {
        let (result_tx, result_rx) = oneshot::channel();
        self.prepare_block_tx
            .send(PrepareBlockCommand {
                block_id,
                txns_to_execute,
                maybe_block_gas_limit,
                parent_block_id,
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
        mut prepare_block_rx: mpsc::UnboundedReceiver<PrepareBlockCommand>,
        execute_block_tx: mpsc::UnboundedSender<ExecuteBlockCommand>,
    ) {
        while let Some(PrepareBlockCommand {
            block_id,
            txns_to_execute,
            maybe_block_gas_limit,
            parent_block_id,
            result_tx,
        }) = prepare_block_rx.recv().await
        {
            debug!("prepare_block received block {}.", block_id);
            let execute_block_tx = execute_block_tx.clone();
            let sig_verified_txns = monitor!(
                "prepare_block",
                tokio::task::spawn_blocking(move || {
                    let sig_verified_txns: Vec<SignatureVerifiedTransaction> = SIG_VERIFY_POOL
                        .install(|| {
                            txns_to_execute
                                .into_par_iter()
                                .map(into_signature_verified)
                                .collect::<Vec<_>>()
                        });
                    sig_verified_txns
                })
                .await
            )
            .expect("Failed to spawn_blocking.");

            execute_block_tx
                .send(ExecuteBlockCommand {
                    block: (block_id, sig_verified_txns).into(),
                    parent_block_id,
                    maybe_block_gas_limit,
                    result_tx,
                })
                .expect("Failed to send block to execution pipeline.");
        }
    }

    async fn execute_stage(
        mut block_rx: mpsc::UnboundedReceiver<ExecuteBlockCommand>,
        ledger_apply_tx: mpsc::UnboundedSender<LedgerApplyCommand>,
        executor: Arc<dyn BlockExecutorTrait>,
    ) {
        while let Some(ExecuteBlockCommand {
            block,
            parent_block_id,
            maybe_block_gas_limit,
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
                    executor.execute_and_state_checkpoint(
                        block,
                        parent_block_id,
                        maybe_block_gas_limit,
                    )
                })
                .await
            )
            .expect("Failed to spawn_blocking.");

            ledger_apply_tx
                .send(LedgerApplyCommand {
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
            block_id,
            parent_block_id,
            state_checkpoint_output,
            result_tx,
        }) = block_rx.recv().await
        {
            debug!("ledger_apply stage received block {}.", block_id);
            let res = async {
                let executor = executor.clone();
                monitor!(
                    "ledger_apply",
                    tokio::task::spawn_blocking(move || {
                        executor.ledger_update(block_id, parent_block_id, state_checkpoint_output?)
                    })
                )
                .await
                .expect("Failed to spawn_blocking().")
            }
            .await;
            result_tx.send(res).unwrap_or_else(|err| {
                error!(
                    block_id = block_id,
                    "Failed to send back execution result for block {}: {:?}", block_id, err,
                );
            });
        }
        debug!("ledger_apply stage quitting.");
    }
}

struct PrepareBlockCommand {
    block_id: HashValue,
    txns_to_execute: Vec<Transaction>,
    maybe_block_gas_limit: Option<u64>,
    // The parent block id.
    parent_block_id: HashValue,
    result_tx: oneshot::Sender<ExecutorResult<StateComputeResult>>,
}

struct ExecuteBlockCommand {
    block: ExecutableBlock,
    parent_block_id: HashValue,
    maybe_block_gas_limit: Option<u64>,
    result_tx: oneshot::Sender<ExecutorResult<StateComputeResult>>,
}

struct LedgerApplyCommand {
    block_id: HashValue,
    parent_block_id: HashValue,
    state_checkpoint_output: ExecutorResult<StateCheckpointOutput>,
    result_tx: oneshot::Sender<ExecutorResult<StateComputeResult>>,
}
