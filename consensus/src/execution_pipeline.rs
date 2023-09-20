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
use aptos_types::block_executor::partitioner::ExecutableBlock;
use fail::fail_point;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

pub struct ExecutionPipeline {
    block_tx: mpsc::UnboundedSender<ExecuteBlockCommand>,
}

impl ExecutionPipeline {
    pub fn spawn(executor: Arc<dyn BlockExecutorTrait>, runtime: &tokio::runtime::Handle) -> Self {
        let (block_tx, block_rx) = mpsc::unbounded_channel();
        let (ledger_apply_tx, ledger_apply_rx) = mpsc::unbounded_channel();
        runtime.spawn(Self::execute_stage(
            block_rx,
            ledger_apply_tx,
            executor.clone(),
        ));
        runtime.spawn(Self::ledger_apply_stage(ledger_apply_rx, executor));
        Self { block_tx }
    }

    pub async fn queue(
        &self,
        block: ExecutableBlock,
        parent_block_id: HashValue,
        maybe_block_gas_limit: Option<u64>,
    ) -> StateComputeResultFut {
        let (result_tx, result_rx) = oneshot::channel();
        let block_id = block.block_id;
        self.block_tx
            .send(ExecuteBlockCommand {
                block,
                parent_block_id,
                maybe_block_gas_limit,
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

const PIPELINE_DEPTH: usize = 8;

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
