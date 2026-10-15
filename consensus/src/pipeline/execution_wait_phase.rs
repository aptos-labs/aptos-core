// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::pipeline::{buffer_item::ExecutionFut, pipeline_phase::StatelessPipeline};
use aptos_consensus_types::pipelined_block::PipelinedBlock;
use aptos_crypto::HashValue;
use aptos_executor_types::ExecutorResult;
use async_trait::async_trait;
use std::{
    fmt::{Debug, Display, Formatter},
    sync::Arc,
};

/// [ This class is used when consensus.decoupled = true ]
/// ExecutionWaitPhase is a singleton that receives scheduled execution futures
/// from ExecutionSchedulePhase and waits for the results from the ExecutionPipeline.

pub struct ExecutionWaitRequest {
    pub block_id: HashValue,
    pub fut: ExecutionFut,
}

impl Debug for ExecutionWaitRequest {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for ExecutionWaitRequest {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "ExecutionRequest({:?})", self.block_id)
    }
}

pub struct ExecutionResponse {
    pub block_id: HashValue,
    pub inner: ExecutorResult<Vec<Arc<PipelinedBlock>>>,
}

pub struct ExecutionWaitPhase;

#[async_trait]
impl StatelessPipeline for ExecutionWaitPhase {
    type Request = ExecutionWaitRequest;
    type Response = ExecutionResponse;

    const NAME: &'static str = "execution";

    async fn process(&self, req: ExecutionWaitRequest) -> ExecutionResponse {
        let ExecutionWaitRequest { block_id, fut } = req;

        ExecutionResponse {
            block_id,
            inner: fut.await,
        }
    }
}
