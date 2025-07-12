// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{network::NetworkSender, pipeline::pipeline_phase::StatelessPipeline};
use aptos_consensus_types::{common::Round, pipelined_block::PipelinedBlock};
use aptos_executor_types::ExecutorResult;
use aptos_types::{epoch_change::EpochChangeProof, ledger_info::LedgerInfoWithSignatures};
use async_trait::async_trait;
use std::{
    fmt::{Debug, Display, Formatter},
    sync::Arc,
};

/// [ This class is used when consensus.decoupled = true ]
/// PersistingPhase is a singleton that receives aggregated blocks from
/// the buffer manager and persists them. Upon success, it returns
/// a response.

pub struct PersistingRequest {
    pub blocks: Vec<Arc<PipelinedBlock>>,
    pub commit_ledger_info: LedgerInfoWithSignatures,
}

impl Debug for PersistingRequest {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for PersistingRequest {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "PersistingRequest({:?}, {})",
            self.blocks, self.commit_ledger_info,
        )
    }
}

pub type PersistingResponse = ExecutorResult<Round>;

pub struct PersistingPhase {
    commit_msg_tx: Arc<NetworkSender>,
}

impl PersistingPhase {
    pub fn new(commit_msg_tx: Arc<NetworkSender>) -> Self {
        Self { commit_msg_tx }
    }
}

#[async_trait]
impl StatelessPipeline for PersistingPhase {
    type Request = PersistingRequest;
    type Response = PersistingResponse;

    const NAME: &'static str = "persisting";

    async fn process(&self, req: PersistingRequest) -> PersistingResponse {
        let PersistingRequest {
            blocks,
            commit_ledger_info,
        } = req;

        for b in &blocks {
            if let Some(tx) = b.pipeline_tx().lock().as_mut() {
                tx.commit_proof_tx
                    .take()
                    .map(|tx| tx.send(commit_ledger_info.clone()));
            }
            b.wait_for_commit_ledger().await;
        }

        let response = Ok(blocks.last().expect("Blocks can't be empty").round());
        if commit_ledger_info.ledger_info().ends_epoch() {
            self.commit_msg_tx
                .send_epoch_change(EpochChangeProof::new(vec![commit_ledger_info], false))
                .await;
        }
        response
    }
}
