// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{experimental::pipeline_phase::StatelessPipeline, state_replication::StateComputer};
use velor_consensus_types::executed_block::ExecutedBlock;
use velor_crypto::HashValue;
use velor_executor_types::{ExecutorError, ExecutorResult};
use async_trait::async_trait;
use std::{
    fmt::{Debug, Display, Formatter},
    sync::Arc,
};

/// [ This class is used when consensus.decoupled = true ]
/// ExecutionPhase is a singleton that receives ordered blocks from
/// the buffer manager and execute them. After the execution is done,
/// ExecutionPhase sends the ordered blocks back to the buffer manager.
///

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
        write!(f, "ExecutionRequest({:?})", self.ordered_blocks)
    }
}

pub struct ExecutionResponse {
    pub block_id: HashValue,
    pub inner: ExecutorResult<Vec<ExecutedBlock>>,
}

pub struct ExecutionPhase {
    execution_proxy: Arc<dyn StateComputer>,
}

impl ExecutionPhase {
    pub fn new(execution_proxy: Arc<dyn StateComputer>) -> Self {
        Self { execution_proxy }
    }
}

#[async_trait]
impl StatelessPipeline for ExecutionPhase {
    type Request = ExecutionRequest;
    type Response = ExecutionResponse;

    const NAME: &'static str = "execution";

    async fn process(&self, req: ExecutionRequest) -> ExecutionResponse {
        let ExecutionRequest { ordered_blocks } = req;

        if ordered_blocks.is_empty() {
            // return err when the blocks are empty
            return ExecutionResponse {
                block_id: HashValue::zero(),
                inner: Err(ExecutorError::EmptyBlocks),
            };
        }

        let block_id = ordered_blocks.last().unwrap().id();
        let mut result = vec![];

        for b in ordered_blocks {
            match self.execution_proxy.compute(b.block(), b.parent_id()).await {
                Ok(compute_result) => {
                    result.push(ExecutedBlock::new(b.block().clone(), compute_result));
                },
                Err(e) => {
                    return ExecutionResponse {
                        block_id,
                        inner: Err(e),
                    }
                },
            }
        }

        ExecutionResponse {
            block_id,
            inner: Ok(result),
        }
    }
}
