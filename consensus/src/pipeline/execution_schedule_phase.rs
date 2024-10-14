// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    pipeline::{
        execution_wait_phase::ExecutionWaitRequest,
        pipeline_phase::{CountedRequest, StatelessPipeline},
    },
    state_replication::StateComputer,
};
use aptos_consensus_types::pipelined_block::PipelinedBlock;
use aptos_crypto::HashValue;
use aptos_logger::debug;
use async_trait::async_trait;
use futures::FutureExt;
use std::{
    fmt::{Debug, Display, Formatter},
    sync::Arc,
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
}

impl ExecutionSchedulePhase {
    pub fn new(execution_proxy: Arc<dyn StateComputer>) -> Self {
        Self { execution_proxy }
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
        let mut futs = vec![];
        for block in &ordered_blocks {
            let prepare_fut = block.get_prepare_fut();
            let fut = if let Ok(Ok((input_txns, sig_verified_txns))) = prepare_fut.await {
                self.execution_proxy
                    .schedule_compute(
                        block,
                        (input_txns, sig_verified_txns),
                        lifetime_guard.spawn(()),
                    )
                    .await
            } else {
                async move {
                    Err(aptos_executor_types::ExecutorError::InternalError {
                        error: "Prepare failed".to_string(),
                    })
                }
                .boxed()
            };
            futs.push(fut)
        }

        // In the future being returned, wait for the compute results in order.
        let fut = async move {
            let mut results = vec![];
            for (block, fut) in itertools::zip_eq(ordered_blocks, futs) {
                debug!("try to receive compute result for block {}", block.id());
                results.push(block.set_execution_result(fut.await?));
            }
            Ok(results)
        };

        ExecutionWaitRequest {
            block_id,
            fut: Box::pin(fut),
        }
    }
}
