// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::commit_pipeline_tests::{network, network_with_events};
use crate::{
    network_interface::ConsensusMsg, pipeline::ordering_pipeline::build_publication_futures,
};
use aptos_consensus_types::{
    block::{block_test_utils::certificate_for_genesis, Block},
    order_vote::OrderVote,
    pipelined_block::{OrderVoteInput, ProposalVoteInput},
    sync_info::SyncInfo,
    vote::Vote,
    vote_data::VoteData,
};
use aptos_network::protocols::network::Event;
use aptos_types::{ledger_info::LedgerInfo, validator_verifier::random_validator_verifier};
use futures::{FutureExt, StreamExt};
use std::{sync::Arc, time::Duration};
use tokio::sync::oneshot;

#[tokio::test]
async fn publication_waits_for_authorization_and_not_for_execution() {
    let (signers, verifier) = random_validator_verifier(1, None, false);
    let signer = &signers[0];
    let qc = certificate_for_genesis();
    let block = Arc::new(Block::new_nil(1, qc.clone(), vec![]));
    let (proposal_tx, proposal_rx) = oneshot::channel();
    let (order_tx, order_rx) = oneshot::channel();
    let mut abort_handles = vec![];
    let (sender, mut events) = network_with_events(signer, Arc::new(verifier));
    // The same graph constructor used by PipelineBuilder has no execution input.
    let (proposal, order) = build_publication_futures(
        sender,
        block,
        proposal_rx,
        order_rx,
        false,
        &mut abort_handles,
    );
    assert!(proposal.clone().now_or_never().is_none());
    assert!(order.clone().now_or_never().is_none());
    let vote = Vote::new(
        VoteData::dummy(),
        signer.author(),
        LedgerInfo::dummy(),
        signer,
    )
    .unwrap();
    proposal_tx
        .send(ProposalVoteInput {
            vote: vote.clone(),
            sync_info: SyncInfo::new(qc.clone(), qc.into_wrapped_ledger_info(), None),
            recipient: None,
        })
        .ok()
        .unwrap();
    let published = tokio::time::timeout(Duration::from_secs(5), proposal)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(published, vote);
    let Event::Message(author, ConsensusMsg::VoteMsg(message)) = events.next().await.unwrap()
    else {
        panic!("Expected published proposal vote");
    };
    assert_eq!(author, signer.author());
    assert_eq!(message.vote(), &vote);
    assert!(order.clone().now_or_never().is_none());
    let ledger_info = LedgerInfo::dummy();
    let order_vote = OrderVote::new_with_signature(
        signer.author(),
        ledger_info.clone(),
        signer.sign(&ledger_info).unwrap(),
    );
    order_tx
        .send(OrderVoteInput {
            vote: order_vote.clone(),
            qc: Arc::new(certificate_for_genesis()),
        })
        .ok()
        .unwrap();
    let published_order = tokio::time::timeout(Duration::from_secs(5), order)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(published_order.author(), order_vote.author());
    assert_eq!(published_order.ledger_info(), order_vote.ledger_info());
    let Event::Message(author, ConsensusMsg::OrderVoteMsg(message)) = events.next().await.unwrap()
    else {
        panic!("Expected published order vote");
    };
    assert_eq!(author, signer.author());
    assert_eq!(message.order_vote().author(), order_vote.author());
    assert_eq!(message.order_vote().ledger_info(), order_vote.ledger_info());
}

#[tokio::test]
async fn reset_cancels_waiting_publication_stages() {
    let (signers, verifier) = random_validator_verifier(1, None, false);
    let (_proposal_tx, proposal_rx) = oneshot::channel();
    let (_order_tx, order_rx) = oneshot::channel();
    let mut handles = vec![];
    let (proposal, order) = build_publication_futures(
        network(&signers[0], Arc::new(verifier)),
        Arc::new(Block::make_genesis_block()),
        proposal_rx,
        order_rx,
        false,
        &mut handles,
    );
    for handle in handles {
        handle.abort();
    }
    assert!(proposal.await.unwrap_err().is_cancellation());
    assert!(order.await.unwrap_err().is_cancellation());
}
