// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    pipeline::pipeline_phase::{CountedRequest, StatelessPipeline},
    state_computer::SyncStateComputeResultFut,
    state_replication::StateComputer,
};
use aptos_consensus_types::pipelined_block::PipelinedBlock;
use aptos_crypto::HashValue;
use aptos_executor_types::ExecutorError;
use aptos_logger::{debug, info, warn};
use async_trait::async_trait;
use dashmap::DashMap;
use futures::TryFutureExt;
use once_cell::sync::OnceCell;
use std::sync::{atomic::AtomicU64, Arc};

pub struct PreExecutionRequest {
    pub block: PipelinedBlock,
    pub lifetime_guard: CountedRequest<()>,
}

pub struct PreExecutionPhase {
    execution_proxy: Arc<dyn StateComputer>,
    execution_futures: Arc<DashMap<HashValue, SyncStateComputeResultFut>>,
}

impl PreExecutionPhase {
    pub fn new(execution_proxy: Arc<dyn StateComputer>, execution_futures: Arc<DashMap<HashValue, SyncStateComputeResultFut>>) -> Self {
        Self {
            execution_proxy,
            execution_futures,
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
            lifetime_guard,
        } = req;

        match self.execution_futures.entry(block.id()) {
            dashmap::mapref::entry::Entry::Occupied(_) => {}
            dashmap::mapref::entry::Entry::Vacant(entry) => {
                info!("[PreExecution] pre-execute block of epoch {} round {} id {}", block.epoch(), block.round(), block.id());
                let fut = self
                    .execution_proxy
                    .schedule_compute(block.block(), block.parent_id(), block.randomness().cloned(), lifetime_guard.spawn(()))
                    .await;
                entry.insert(fut);
            }
        }
    }
}
