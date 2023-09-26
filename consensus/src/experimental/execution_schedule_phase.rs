// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    experimental::{execution_wait_phase::ExecutionWaitRequest, pipeline_phase::StatelessPipeline},
    state_replication::StateComputer,
};
use aptos_consensus_types::executed_block::ExecutedBlock;
use aptos_crypto::HashValue;
use aptos_logger::debug;
use async_trait::async_trait;
use std::{
    fmt::{Debug, Display, Formatter},
    sync::Arc,
};

/// [ This class is used when consensus.decoupled = true ]
/// ExecutionSchedulePhase is a singleton that receives ordered blocks from
/// the buffer manager and send them to the ExecutionPipeline.

pub struct ExecutionRequest {
    pub ordered_blocks: Vec<ExecutedBlock>,
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
        let ExecutionRequest { ordered_blocks } = req;

        if ordered_blocks.is_empty() {
            return ExecutionWaitRequest {
                block_id: HashValue::zero(),
                fut: Box::pin(async { Err(aptos_executor_types::ExecutorError::EmptyBlocks) }),
            };
        }

        let block_id = ordered_blocks.last().unwrap().id();

        // Call schedule_compute() for each block here (not in the fut being returned) to
        // make sure they are scheduled in order.
        let mut futs = vec![];
        for b in &ordered_blocks {
            let fut = self
                .execution_proxy
                .schedule_compute(b.block(), b.parent_id())
                .await;
            futs.push(fut)
        }

        // In the future being returned, wait for the compute results in order.
        let fut = Box::pin(async move {
            let mut results = vec![];
            for (block, fut) in itertools::zip_eq(ordered_blocks, futs) {
                debug!("try to receive compute result for block {}", block.id());
                results.push(block.replace_result(fut.await?));
            }
            Ok(results)
        });

        ExecutionWaitRequest { block_id, fut }
    }
}
