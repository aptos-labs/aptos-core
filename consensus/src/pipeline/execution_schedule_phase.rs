// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::{NUM_NON_PREEXECUTED_BLOCKS, NUM_PREEXECUTED_BLOCKS, NUM_PREEXECUTED_BLOCKS_SENT_TO_PRECOMMIT, NUM_RE_EXECUTED_BLOCKS}, pipeline::{
        execution_wait_phase::ExecutionWaitRequest,
        pipeline_phase::{CountedRequest, StatelessPipeline}, pre_execution_phase::ExecutionType,
    }, state_computer::{StateComputeResultFut, SyncStateComputeResultFut}, state_replication::StateComputer
};
use aptos_consensus_types::{pipeline_execution_result::PipelineExecutionResult, pipelined_block::PipelinedBlock};
use aptos_crypto::HashValue;
use aptos_executor_types::{ExecutorError, ExecutorResult};
use aptos_logger::{debug, info};
use async_trait::async_trait;
use dashmap::DashMap;
use futures::{FutureExt, TryFutureExt};
use std::{
    collections::HashMap, fmt::{Debug, Display, Formatter}, pin::Pin, sync::Arc
};

/// [ This class is used when consensus.decoupled = true ]
/// ExecutionSchedulePhase is a singleton that receives ordered blocks from
/// the buffer manager and send them to the ExecutionPipeline.

pub struct ExecutionRequest {
    pub ordered_blocks: Vec<PipelinedBlock>,
    // Pass down a CountedRequest to the ExecutionPipeline stages in order to guarantee the executor
    // doesn't get reset with pending tasks stuck in the pipeline.
    pub lifetime_guard: CountedRequest<()>,
}

impl Debug for ExecutionRequest {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for ExecutionRequest {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "ExecutionScheduleRequest({:?})", self.ordered_blocks)
    }
}

pub struct ExecutionSchedulePhase {
    execution_proxy: Arc<dyn StateComputer>,
    execution_futures: Arc<DashMap<HashValue, SyncStateComputeResultFut>>,
}

impl ExecutionSchedulePhase {
    pub fn new(execution_proxy: Arc<dyn StateComputer>, execution_futures: Arc<DashMap<HashValue, SyncStateComputeResultFut>>) -> Self {
        Self {
            execution_proxy,
            execution_futures,
        }
    }
}

#[async_trait]
impl StatelessPipeline for ExecutionSchedulePhase {
    type Request = ExecutionRequest;
    type Response = ExecutionWaitRequest;

    const NAME: &'static str = "execution_schedule";

    async fn process(&self, req: ExecutionRequest) -> ExecutionWaitRequest {
        let ExecutionRequest {
            ordered_blocks,
            lifetime_guard,
        } = req;

        let block_id = match ordered_blocks.last() {
            Some(block) => block.id(),
            None => {
                return ExecutionWaitRequest {
                    block_id: HashValue::zero(),
                    fut: Box::pin(async { Err(aptos_executor_types::ExecutorError::EmptyBlocks) }),
                }
            },
        };

        // Call schedule_compute() for each block here (not in the fut being returned) to
        // make sure they are scheduled in order.
        for block in &ordered_blocks {
            match self.execution_futures.entry(block.id()) {
                dashmap::mapref::entry::Entry::Occupied(_) => {
                    info!("[PreExecution] block was pre-executed, epoch {} round {} id {}", block.epoch(), block.round(), block.id());
                    NUM_PREEXECUTED_BLOCKS.inc();
                }
                dashmap::mapref::entry::Entry::Vacant(entry) => {
                    info!("[PreExecution] block was not pre-executed, epoch {} round {} id {}", block.epoch(), block.round(), block.id());
                    let fut = self
                        .execution_proxy
                        .schedule_compute(block.block(), block.parent_id(), block.randomness().cloned(), lifetime_guard.spawn(()), ExecutionType::Execution)
                        .await;
                    entry.insert(fut);
                    NUM_NON_PREEXECUTED_BLOCKS.inc();
                }
            }
        }

        let execution_futures = self.execution_futures.clone();
        let execution_proxy = self.execution_proxy.clone();

        // In the future being returned, wait for the compute results in order.
        let fut = async move {
            let mut results = vec![];
            // wait for all futs so that lifetime_guard is guaranteed to be dropped only
            // after all executor calls are over
            for block in &ordered_blocks {
                debug!("[Execution] try to receive compute result for block, epoch {} round {} id {}", block.epoch(), block.round(), block.id());
                match execution_futures.entry(block.id()) {
                    dashmap::mapref::entry::Entry::Occupied(mut entry) => {
                        let fut = entry.get_mut();
                        let result = match fut.await {
                            Ok(mut result) => {
                                if result.pre_commit_fut.is_some() {
                                    Ok(result)
                                } else {
                                    info!("[PreExecution] pre-executed block directly forward to pre-commit, epoch {} round {} id {}", block.epoch(), block.round(), block.id());
                                    NUM_PREEXECUTED_BLOCKS_SENT_TO_PRECOMMIT.inc();
                                    let pre_commit_fut = execution_proxy
                                        .schedule_pre_commit(block.id(), block.parent_id(), lifetime_guard.spawn(()))
                                        .await;
                                    result.set_pre_commit_fut(pre_commit_fut.clone());
                                    Ok(result)
                                }
                            },
                            Err(e) => {
                                info!("[PreExecution] block is re-executed due to error {:?}, epoch {} round {} id {}", e, block.epoch(), block.round(), block.id());
                                NUM_RE_EXECUTED_BLOCKS.inc();
                                let fut = execution_proxy
                                    .schedule_compute(block.block(), block.parent_id(), block.randomness().cloned(), lifetime_guard.spawn(()), ExecutionType::Execution)
                                    .await;
                                entry.insert(fut.clone());
                                fut.await
                            }
                        };
                        results.push(result);
                    }
                    dashmap::mapref::entry::Entry::Vacant(_) => {
                        return Err(ExecutorError::internal_err(format!(
                            "Failed to find compute result for block {}",
                            block.id()
                        )));
                    }
                }
            }
            let results = itertools::zip_eq(ordered_blocks, results)
                .map(|(block, res)| {
                    info!("[Execution] set execution result for block, epoch {} round {} id {}", block.epoch(), block.round(), block.id());
                    Ok(block.set_execution_result(res?))
                })
                .collect::<ExecutorResult<Vec<_>>>()?;
            drop(lifetime_guard);
            Ok(results)
        }.boxed();

        ExecutionWaitRequest {
            block_id,
            fut,
        }
    }
}
