// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    block_storage::tracing::{observe_block, BlockStage},
    liveness::round_state::RoundState,
    metrics_safety_rules::MetricsSafetyRules,
    persistent_liveness_storage::PersistentLivenessStorage,
};
use anyhow::{ensure, Context, Result};
use aptos_consensus_types::{
    order_vote::OrderVote, pipelined_block::PipelinedBlock, quorum_cert::QuorumCert,
    timeout_2chain::TwoChainTimeoutCertificate, vote::Vote,
};
use aptos_infallible::Mutex;
use aptos_safety_rules::TSafetyRules;
use fail::fail_point;
use std::sync::Arc;

/// Invoked only by the serialized round controller, before publication is released
/// into a block future. Do not move these checks/signing calls to independent tasks.
pub fn authorize_proposal_vote(
    block: &PipelinedBlock,
    round_state: &RoundState,
    sync_only: bool,
    timeout_cert: Option<&TwoChainTimeoutCertificate>,
    safety_rules: &Mutex<MetricsSafetyRules>,
    storage: &dyn PersistentLivenessStorage,
) -> Result<Vote> {
    ensure!(
        round_state.vote_sent().is_none(),
        "[RoundManager] Already vote on this round {}",
        round_state.current_round()
    );
    ensure!(
        !sync_only,
        "[RoundManager] sync_only flag is set, stop voting"
    );
    let vote = safety_rules
        .lock()
        .construct_and_sign_vote_two_chain(&block.vote_proposal(), timeout_cert)
        .context(format!(
            "[RoundManager] SafetyRules Rejected {}",
            block.block()
        ))?;
    if !block.block().is_nil_block() {
        observe_block(block.timestamp_usecs(), BlockStage::VOTED);
    }
    if block.block().is_opt_block() {
        observe_block(block.timestamp_usecs(), BlockStage::VOTED_OPT_BLOCK);
    }
    storage
        .save_vote(&vote)
        .context("[RoundManager] Fail to persist last vote")?;
    Ok(vote)
}

pub fn authorize_order_vote(
    block: &PipelinedBlock,
    qc: Arc<QuorumCert>,
    safety_rules: &Mutex<MetricsSafetyRules>,
) -> Result<OrderVote> {
    let vote = safety_rules
        .lock()
        .construct_and_sign_order_vote(&block.order_vote_proposal(qc))
        .context(format!(
            "[RoundManager] SafetyRules Rejected {} for order vote",
            block.block()
        ))?;
    fail_point!("consensus::create_invalid_order_vote", |_| {
        Ok(OrderVote::new_with_signature(
            vote.author(),
            vote.ledger_info().clone(),
            aptos_crypto::bls12381::Signature::dummy_signature(),
        ))
    });
    Ok(vote)
}
