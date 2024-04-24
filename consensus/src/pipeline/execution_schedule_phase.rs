// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    pipeline::{
        execution_wait_phase::ExecutionWaitRequest,
        pipeline_phase::{CountedRequest, StatelessPipeline},
    },
    state_computer::PipelineExecutionResult,
    state_replication::StateComputer,
};
use aptos_consensus_types::pipelined_block::PipelinedBlock;
use aptos_crypto::HashValue;
use aptos_executor_types::ExecutorError;
use aptos_logger::debug;
use async_trait::async_trait;
use dashmap::DashMap;
use futures::TryFutureExt;
use std::{
    collections::HashMap, fmt::{Debug, Display, Formatter}, sync::Arc
};

/// [ This class is used when consensus.decoupled = true ]
/// ExecutionSchedulePhase is a singleton that receives ordered blocks from
/// the buffer manager and send them to the ExecutionPipeline.

pub struct ExecutionRequest {
    pub ordered_blocks: Vec<PipelinedBlock>,
    // Hold a CountedRequest to guarantee the executor doesn't get reset with pending tasks
    // stuck in the ExecutinoPipeline.
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
    pre_execution_results: Option<Arc<DashMap<HashValue, PipelineExecutionResult>>>,
}

impl ExecutionSchedulePhase {
    pub fn new(execution_proxy: Arc<dyn StateComputer>, pre_execution_results: Option<Arc<DashMap<HashValue, PipelineExecutionResult>>>) -> Self {
        Self { 
            execution_proxy,
            pre_execution_results,
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

        if ordered_blocks.is_empty() {
            return ExecutionWaitRequest {
                block_id: HashValue::zero(),
                fut: Box::pin(async { Err(aptos_executor_types::ExecutorError::EmptyBlocks) }),
            };
        }

        let block_id = ordered_blocks.last().unwrap().id();

        // Call schedule_compute() for each block here (not in the fut being returned) to
        // make sure they are scheduled in order.
        let mut futs = HashMap::new();
        let mut pre_execution_results: HashMap<HashValue, PipelineExecutionResult> = HashMap::new();
        for b in &ordered_blocks {
            if let Some(results) = &self.pre_execution_results {
                if results.contains_key(&b.id()) {
                    pre_execution_results.insert(b.id(), results.get(&b.id()).unwrap().value().clone());
                    continue;
                }
            }
            let fut = self
                .execution_proxy
                .schedule_compute(b.block(), b.parent_id(), b.randomness().cloned())
                .await;
            futs.insert(b.id(), fut);
        }

        // In the future being returned, wait for the compute results in order.
        // n.b. Must `spawn()` here to make sure lifetime_guard will be released even if
        //      ExecutionWait phase is never kicked off.
        let fut = tokio::task::spawn(async move {
            let mut results = vec![];
            for block in ordered_blocks {
                if pre_execution_results.contains_key(&block.id()) {
                    debug!("[PreExecution] block of epoch {} round {} was pre-executed", block.epoch(), block.round());
                    let execution_result = pre_execution_results.get(&block.id()).unwrap().clone();
                    let PipelineExecutionResult { input_txns, result } = execution_result;
                    results.push(block.set_execution_result(input_txns, result));
                } else {
                    debug!("[PreExecution] block of epoch {} round {} was not pre-executed", block.epoch(), block.round());
                    let fut = futs.remove(&block.id()).unwrap();
                    let PipelineExecutionResult { input_txns, result } = fut.await?;
                    results.push(block.set_execution_result(input_txns, result));
                }
            }
            drop(lifetime_guard);
            Ok(results)
        })
        .map_err(ExecutorError::internal_err)
        .and_then(|res| async { res });

        ExecutionWaitRequest {
            block_id,
            fut: Box::pin(fut),
        }
    }
}
