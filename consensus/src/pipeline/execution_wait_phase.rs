// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::pipeline::{buffer_manager::Sender, pipeline_phase::StatelessPipeline};
use aptos_consensus_types::pipelined_block::PipelinedBlock;
use aptos_crypto::HashValue;
use aptos_executor_types::{ExecutorError, ExecutorResult};
use aptos_logger::{debug, error};
use async_trait::async_trait;
use std::{
    fmt::{Debug, Display, Formatter},
    sync::Arc,
};

/// [ This class is used when consensus.decoupled = true ]
/// ExecutionWaitPhase is a singleton that receives ordered blocks from
/// ExecutionSchedulePhase and waits for their results from the ExecutionPipeline.
/// It sends one response per executed segment of the ordered blocks.

pub struct ExecutionWaitRequest {
    pub block_id: HashValue,
    pub ordered_blocks: Vec<Arc<PipelinedBlock>>,
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

/// Result of executing a prefix of the ordered blocks keyed by `block_id`. The
/// prefix is shorter than the ordered blocks when execution was cut after a
/// block that requires cache invalidation.
pub struct ExecutionResponse {
    pub block_id: HashValue,
    pub inner: ExecutorResult<Vec<Arc<PipelinedBlock>>>,
}

pub struct ExecutionWaitPhase {
    response_tx: Sender<ExecutionResponse>,
    order_vote_enabled: bool,
}

impl ExecutionWaitPhase {
    pub fn new(response_tx: Sender<ExecutionResponse>, order_vote_enabled: bool) -> Self {
        Self {
            response_tx,
            order_vote_enabled,
        }
    }

    fn send(&self, block_id: HashValue, inner: ExecutorResult<Vec<Arc<PipelinedBlock>>>) {
        if self
            .response_tx
            .unbounded_send(ExecutionResponse { block_id, inner })
            .is_err()
        {
            debug!("Failed to send response, buffer manager probably dropped");
        }
    }
}

#[async_trait]
impl StatelessPipeline for ExecutionWaitPhase {
    type Request = ExecutionWaitRequest;
    type Response = ();

    const NAME: &'static str = "execution";

    async fn process(&self, req: ExecutionWaitRequest) {
        let ExecutionWaitRequest {
            block_id,
            ordered_blocks,
        } = req;

        if ordered_blocks.is_empty() {
            self.send(block_id, Err(ExecutorError::EmptyBlocks));
            return;
        }

        let num_blocks = ordered_blocks.len();
        let mut segment = vec![];
        for (idx, b) in ordered_blocks.into_iter().enumerate() {
            let (compute_result, execution_time) = match b.wait_for_compute_result().await {
                Ok(result) => result,
                Err(err) => {
                    self.send(block_id, Err(err));
                    return;
                },
            };
            // Descendants of a block that requires cache invalidation execute
            // only after it commits, so it has to commit on its own.
            let needs_cut =
                idx + 1 < num_blocks && compute_result.execution_output.needs_invalidation();
            // Without order votes, validators would sign different ledger infos
            // for the block, so the remaining blocks wait for a commit that
            // never happens.
            if needs_cut && !self.order_vote_enabled {
                error!(
                    "[Pipeline] Block {} requires cache invalidation, but order votes are disabled",
                    b.id()
                );
            }
            let cut = needs_cut && self.order_vote_enabled;
            b.set_compute_result(compute_result, execution_time);
            segment.push(b);

            // Send the prefix before waiting on the rest, because the rest
            // depends on the prefix's commit.
            if cut {
                self.send(block_id, Ok(std::mem::take(&mut segment)));
            }
        }
        self.send(block_id, Ok(segment));
    }
}
