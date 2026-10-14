// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    block_storage::tracing::{observe_block, BlockStage},
    counters::{ORDER_VOTE_BROADCASTED, PROPOSAL_VOTE_BROADCASTED},
    logging::{LogEvent, LogSchema},
    network::NetworkSender,
    pipeline::pipeline_builder::{spawn_ready_fut, spawn_shared_fut, Tracker},
};
use anyhow::{anyhow, Result};
use aptos_consensus_types::{
    block::Block,
    order_vote::OrderVote,
    order_vote_msg::OrderVoteMsg,
    pipelined_block::{OrderVoteInput, PipelinedBlock, ProposalVoteInput, TaskFuture},
    vote::Vote,
    vote_msg::VoteMsg,
};
use aptos_logger::info;
use std::sync::Arc;
use tokio::{sync::oneshot, task::AbortHandle};

pub(super) fn build_publication_futures(
    network: Arc<NetworkSender>,
    block: Arc<Block>,
    proposal_rx: oneshot::Receiver<ProposalVoteInput>,
    order_rx: oneshot::Receiver<OrderVoteInput>,
    observer: bool,
    abort_handles: &mut Vec<AbortHandle>,
) -> (TaskFuture<Option<Vote>>, TaskFuture<Option<OrderVote>>) {
    if observer {
        return (spawn_ready_fut(None), spawn_ready_fut(None));
    }
    let proposal_network = network.clone();
    let proposal_block = block.clone();
    let proposal = spawn_shared_fut(
        async move {
            let mut tracker = Tracker::start_waiting("proposal_vote", &proposal_block);
            let input = proposal_rx
                .await
                .map_err(|_| anyhow!("Proposal vote authorization cancelled"))?;
            tracker.start_working();
            Ok(Some(publish_proposal_vote(&proposal_network, input).await))
        },
        Some(abort_handles),
    );
    let order = spawn_shared_fut(
        async move {
            let mut tracker = Tracker::start_waiting("order_vote", &block);
            let input = order_rx
                .await
                .map_err(|_| anyhow!("Order vote authorization cancelled"))?;
            tracker.start_working();
            Ok(Some(publish_order_vote(&network, &block, input).await))
        },
        Some(abort_handles),
    );
    (proposal, order)
}

/// Publication consumes already-authorized votes. No execution or certificate
/// prerequisite is added to ordinary proposal voting.
pub async fn publish_proposal_vote(network: &NetworkSender, input: ProposalVoteInput) -> Vote {
    let ProposalVoteInput {
        vote,
        sync_info,
        recipient,
    } = input;
    let message = VoteMsg::new(vote.clone(), sync_info);
    let log = LogSchema::new(LogEvent::Vote)
        .epoch(vote.vote_data().proposed().epoch())
        .round(vote.vote_data().proposed().round())
        .author(vote.author());
    if let Some(recipient) = recipient {
        info!(log.remote_peer(recipient), "{}", vote);
        network.send_vote(message, vec![recipient]).await;
    } else {
        info!(log, "{}", vote);
        PROPOSAL_VOTE_BROADCASTED.inc();
        network.broadcast_vote(message).await;
    }
    vote
}

pub async fn publish_order_vote(
    network: &NetworkSender,
    block: &Block,
    input: OrderVoteInput,
) -> OrderVote {
    let OrderVoteInput { vote, qc } = input;
    if !block.is_nil_block() {
        observe_block(block.timestamp_usecs(), BlockStage::ORDER_VOTED);
    }
    if block.is_opt_block() {
        observe_block(block.timestamp_usecs(), BlockStage::ORDER_VOTED_OPT_BLOCK);
    }
    let message = OrderVoteMsg::new(vote.clone(), qc.as_ref().clone());
    info!(
        LogSchema::new(LogEvent::BroadcastOrderVote)
            .epoch(block.epoch())
            .round(block.round()),
        "{}", message
    );
    network.broadcast_order_vote(message).await;
    ORDER_VOTE_BROADCASTED.inc();
    vote
}

/// Release publication only after the round controller has persisted and recorded
/// the proposal vote. The non-pipelined adapter is used by test/fuzzing block stores.
pub async fn release_proposal_vote(
    block: &PipelinedBlock,
    network: &NetworkSender,
    input: ProposalVoteInput,
) -> Result<()> {
    if let Some(futures) = block.pipeline_futs() {
        let sender = block
            .pipeline_tx()
            .lock()
            .as_mut()
            .and_then(|tx| tx.proposal_vote_tx.take())
            .ok_or_else(|| anyhow!("Proposal vote publication already released"))?;
        sender
            .send(input)
            .map_err(|_| anyhow!("Proposal vote pipeline cancelled"))?;
        futures
            .proposal_vote_fut
            .await
            .map_err(|error| anyhow!(error))?;
    } else {
        publish_proposal_vote(network, input).await;
    }
    Ok(())
}

pub async fn release_order_vote(
    block: &PipelinedBlock,
    network: &NetworkSender,
    input: OrderVoteInput,
) -> Result<()> {
    if let Some(futures) = block.pipeline_futs() {
        let sender = block
            .pipeline_tx()
            .lock()
            .as_mut()
            .and_then(|tx| tx.order_vote_tx.take())
            .ok_or_else(|| anyhow!("Order vote publication already released"))?;
        sender
            .send(input)
            .map_err(|_| anyhow!("Order vote pipeline cancelled"))?;
        futures
            .order_vote_fut
            .await
            .map_err(|error| anyhow!(error))?;
    } else {
        publish_order_vote(network, block.block(), input).await;
    }
    Ok(())
}
