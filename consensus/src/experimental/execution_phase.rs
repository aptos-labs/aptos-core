// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::state_replication::StateComputer;
use anyhow::Result;
use consensus_types::{block::Block, executed_block::ExecutedBlock};
use diem_crypto::HashValue;
use executor_types::Error as ExecutionError;
use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    SinkExt, StreamExt,
};
use std::{
    fmt::{Debug, Display, Formatter},
    sync::Arc,
};
use thiserror::Error;

/// [ This class is used when consensus.decoupled = true ]
/// ExecutionPhase is a singleton that receives ordered blocks from
/// the buffer manager and execute them. After the execution is done,
/// ExecutionPhase sends the ordered blocks back to the buffer manager.
///

pub struct ExecutionRequest {
    pub blocks: Vec<Block>,
}

impl Debug for ExecutionRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for ExecutionRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "ExecutionChannelType({:?})", self.blocks)
    }
}

pub struct ExecutionResponse {
    pub inner: Result<Vec<ExecutedBlock>, ExecutionPhaseError>,
}

pub struct ExecutionPhase {
    rx: UnboundedReceiver<ExecutionRequest>,
    tx: UnboundedSender<ExecutionResponse>,
    execution_proxy: Arc<dyn StateComputer>,
}

#[derive(Error)]
pub struct ExecutionPhaseError {
    pub block_id: HashValue,
    pub error: ExecutionError,
}

impl Debug for ExecutionPhaseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for ExecutionPhaseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Execution Phase Error at {}: {:?}",
            self.block_id, self.error
        )
    }
}

impl ExecutionPhase {
    pub fn new(
        rx: UnboundedReceiver<ExecutionRequest>,
        tx: UnboundedSender<ExecutionResponse>,
        execution_proxy: Arc<dyn StateComputer>,
    ) -> Self {
        Self {
            rx,
            tx,
            execution_proxy,
        }
    }

    pub async fn process(&self, item: ExecutionRequest) -> ExecutionResponse {
        let ExecutionRequest { blocks } = item;

        // execute the blocks with execution_correctness_client
        let out_item = blocks
            .into_iter()
            .map(|b| {
                let state_compute_result = self
                    .execution_proxy
                    .compute(&b, b.parent_id())
                    .map_err(|e| ExecutionPhaseError {
                        block_id: b.id(),
                        error: e,
                    })?;
                Ok(ExecutedBlock::new(b, state_compute_result))
            })
            .collect::<Result<Vec<ExecutedBlock>, ExecutionPhaseError>>();

        ExecutionResponse { inner: out_item }
    }

    pub async fn start(mut self) {
        // main loop
        while let Some(in_item) = self.rx.next().await {
            let out_item = self.process(in_item).await;
            if self.tx.send(out_item).await.is_err() {
                break;
            }
        }
    }
}
