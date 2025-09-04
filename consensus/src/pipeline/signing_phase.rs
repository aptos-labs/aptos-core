// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::pipeline::pipeline_phase::StatelessPipeline;
use velor_consensus_types::pipelined_block::PipelinedBlock;
use velor_crypto::bls12381;
use velor_safety_rules::Error;
use velor_types::ledger_info::{LedgerInfo, LedgerInfoWithSignatures};
use async_trait::async_trait;
use std::{
    fmt::{Debug, Display, Formatter},
    sync::Arc,
};

/// [ This class is used when consensus.decoupled = true ]
/// SigningPhase is a singleton that receives executed blocks from
/// the buffer manager and sign them. After getting the signature from
/// the safety rule, SigningPhase sends the signature and error (if any) back.

pub struct SigningRequest {
    pub ordered_ledger_info: LedgerInfoWithSignatures,
    pub commit_ledger_info: LedgerInfo,
    pub blocks: Vec<Arc<PipelinedBlock>>,
}

impl Debug for SigningRequest {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for SigningRequest {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "SigningRequest({}, {})",
            self.ordered_ledger_info, self.commit_ledger_info
        )
    }
}

pub trait CommitSignerProvider: Send + Sync {
    fn sign_commit_vote(
        &self,
        ledger_info: LedgerInfoWithSignatures,
        new_ledger_info: LedgerInfo,
    ) -> Result<bls12381::Signature, Error>;
}

pub struct SigningResponse {
    pub signature_result: Result<bls12381::Signature, Error>,
    pub commit_ledger_info: LedgerInfo,
}

pub struct SigningPhase {
    safety_rule_handle: Arc<dyn CommitSignerProvider>,
}

impl SigningPhase {
    pub fn new(safety_rule_handle: Arc<dyn CommitSignerProvider>) -> Self {
        Self { safety_rule_handle }
    }
}

#[async_trait]
impl StatelessPipeline for SigningPhase {
    type Request = SigningRequest;
    type Response = SigningResponse;

    const NAME: &'static str = "signing";

    async fn process(&self, req: SigningRequest) -> SigningResponse {
        let SigningRequest {
            ordered_ledger_info,
            commit_ledger_info,
            blocks,
        } = req;

        let signature_result = if let Some(fut) = blocks
            .last()
            .expect("Blocks can't be empty")
            .pipeline_futs()
        {
            fut.commit_vote_fut
                .clone()
                .await
                .map(|vote| vote.signature().clone())
                .map_err(|e| Error::InternalError(e.to_string()))
        } else {
            self.safety_rule_handle
                .sign_commit_vote(ordered_ledger_info, commit_ledger_info.clone())
        };

        SigningResponse {
            signature_result,
            commit_ledger_info,
        }
    }
}
