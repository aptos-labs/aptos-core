// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    pipeline::pipeline_phase::StatelessPipeline,
    state_computer::PipelineExecutionResult,
    state_replication::StateComputer,
};
use aptos_consensus_types::pipelined_block::PipelinedBlock;
use aptos_crypto::HashValue;
use aptos_executor_types::ExecutorError;
use aptos_logger::{debug, warn};
use async_trait::async_trait;
use dashmap::DashMap;
use futures::TryFutureExt;
use std::sync::Arc;

pub struct PreExecutionRequest {
    pub block: PipelinedBlock,
}

pub struct PreExecutionPhase {
    execution_proxy: Arc<dyn StateComputer>,
    pre_execution_results: Arc<DashMap<HashValue, PipelineExecutionResult>>,
}

impl PreExecutionPhase {
    pub fn new(execution_proxy: Arc<dyn StateComputer>, pre_execution_results: Arc<DashMap<HashValue, PipelineExecutionResult>>) -> Self {
        Self { 
            execution_proxy,
            pre_execution_results,
        }
    }
}

#[async_trait]
impl StatelessPipeline for PreExecutionPhase {
    type Request = PreExecutionRequest;
    type Response = ();

    const NAME: &'static str = "pre_execution";

    async fn process(&self, req: PreExecutionRequest) {
        let PreExecutionRequest {
            block,
        } = req;

        if self.pre_execution_results.contains_key(&block.id()) {
            return;
        }

        let fut = self
            .execution_proxy
            .schedule_compute(block.block(), block.parent_id(), block.randomness().cloned())
            .await;

        let epoch = block.epoch();
        let round = block.round();
        let execution_results = tokio::task::spawn(async move {
            debug!("[PreExecution] pre-execute block of epoch {} round {}", epoch, round);
            Ok(fut.await?)
        })
        .map_err(ExecutorError::internal_err)
        .and_then(|res| async { res });

        match execution_results.await {
            Ok(execution_result) => {
                self.pre_execution_results.insert(block.id(), execution_result);
            }
            Err(e) => {
                warn!("[PreExecution] pre-execution failed for block of epoch {} round {}: {:?}", epoch, round, e);
            }
        }
    }
}
