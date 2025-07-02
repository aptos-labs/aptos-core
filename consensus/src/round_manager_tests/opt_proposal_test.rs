// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::BlockReader,
    liveness::round_state::{NewRoundEvent, NewRoundReason},
    network_tests::NetworkPlayground,
    round_manager::{
        round_manager_tests::{
            consensus_test::{process_and_vote_on_proposal, process_and_vote_opt_proposal},
            NodeSetup,
        },
        RoundManager,
    },
    test_utils::{consensus_runtime, timed_block_on},
};
use aptos_config::config::ConsensusConfig;
use aptos_consensus_types::{
    common::Payload, opt_block_data::OptBlockData, opt_proposal_msg::OptProposalMsg,
};
use futures::StreamExt;

fn config_with_opt_proposal_enabled() -> ConsensusConfig {
    ConsensusConfig {
        enable_optimistic_proposal_rx: true,
        enable_optimistic_proposal_tx: true,
        ..Default::default()
    }
}

/// Asserts that optimistic proposal is proposed and votes for rounds 2 to n
/// in the absence of failures
#[test]
fn test_opt_proposal_proposed_no_failures() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        Some(config_with_opt_proposal_enabled()),
        None,
        None,
    );
    let genesis = nodes[0].block_store.ordered_root();

    // Process and vote on a normal proposal for round 1
    process_and_vote_on_proposal(&runtime, &mut nodes, 0, &[], true, Some(0), true, 1, 0, 0);

    let node = &mut nodes[0];
    let mut expected_grandparent_qc = genesis.id();
    for round in 2..10 {
        let opt_proposal_msg = timed_block_on(&runtime, async { node.next_opt_proposal().await });
        assert_eq!(opt_proposal_msg.round(), round);
        assert_eq!(
            opt_proposal_msg
                .block_data()
                .grandparent_qc()
                .certified_block()
                .id(),
            expected_grandparent_qc
        );
        expected_grandparent_qc = opt_proposal_msg.block_data().parent_id();
        // process and vote on the optimistic proposal only
        process_and_vote_opt_proposal(
            &runtime,
            node,
            opt_proposal_msg,
            round,
            round.saturating_sub(2),
            0,
        );
        // process vote to gather QC and enter the next round
        timed_block_on(&runtime, async {
            let vote_msg = node.next_vote().await;
            // Adding vote to form a QC
            node.round_manager.process_vote_msg(vote_msg).await.unwrap();
        });
    }
}

/// Asserts that two consecutive opt-proposal rounds timeout and that
/// the round after the timeout rounds is always a normal round.
#[test]
fn test_normal_proposal_after_opt_proposal_timeout() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        Some(config_with_opt_proposal_enabled()),
        None,
        None,
    );
    let genesis = nodes[0].block_store.ordered_root();

    // Process and vote on a normal proposal for round 1
    process_and_vote_on_proposal(&runtime, &mut nodes, 0, &[], true, Some(0), true, 1, 0, 0);

    let node = &mut nodes[0];
    let expected_grandparent_qc = genesis.id();

    let round = 2;
    let opt_proposal_msg = timed_block_on(&runtime, async { node.next_opt_proposal().await });
    assert_eq!(opt_proposal_msg.round(), round);
    assert_eq!(
        opt_proposal_msg
            .block_data()
            .grandparent_qc()
            .certified_block()
            .id(),
        expected_grandparent_qc
    );
    // process and vote on the optimistic proposal only
    process_and_vote_opt_proposal(
        &runtime,
        node,
        opt_proposal_msg,
        round,
        round.saturating_sub(2),
        0,
    );

    timed_block_on(&runtime, async {
        // process round 2 timeout.
        let round = 2;
        node.round_manager
            .process_local_timeout(round)
            .await
            .unwrap_err();
        let timeout_msg = node.next_timeout().await;
        node.round_manager
            .process_round_timeout_msg(timeout_msg)
            .await
            .unwrap();

        // process round 3 timeout.
        let round = 3;
        node.round_manager
            .process_local_timeout(round)
            .await
            .unwrap_err();
        let timeout_msg = node.next_timeout().await;
        node.round_manager
            .process_round_timeout_msg(timeout_msg)
            .await
            .unwrap();

        node.next_proposal().await;
    });
}

/// Asserts that either optimistic proposal or a normal proposal can be
/// created in a given round and not both.
#[test]
fn test_one_proposal_per_round_honest_proposer() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        Some(config_with_opt_proposal_enabled()),
        None,
        None,
    );
    let genesis = nodes[0].block_store.ordered_root();
    let node = &mut nodes[0];

    timed_block_on(&runtime, async {
        let round_manager = &node.round_manager;
        let epoch_state = round_manager.epoch_state.clone();
        let network = round_manager.network.clone();
        let sync_info = round_manager.block_store.sync_info();
        let proposal_generator = round_manager.proposal_generator.clone();
        let proposer_election = round_manager.proposer_election.clone();
        let safety_rules = round_manager.safety_rules.clone();

        // Ensure that an opt proposal cannot be created
        RoundManager::generate_and_send_opt_proposal(
            epoch_state.clone(),
            1,
            genesis.block_info(),
            genesis.quorum_cert().clone(),
            network.clone(),
            sync_info.clone(),
            proposal_generator.clone(),
            proposer_election.clone(),
        )
        .await
        .unwrap_err();

        // Ensure an opt proposal can be created.
        RoundManager::generate_and_send_opt_proposal(
            epoch_state.clone(),
            2,
            genesis.block_info(),
            genesis.quorum_cert().clone(),
            network.clone(),
            sync_info.clone(),
            proposal_generator.clone(),
            proposer_election.clone(),
        )
        .await
        .unwrap();

        // Ensure a proposal cannot be created after an opt proposal in same round
        let new_round_event = NewRoundEvent {
            round: 2,
            reason: NewRoundReason::QCReady,
            timeout: Default::default(),
            prev_round_votes: vec![],
            prev_round_timeout_votes: None,
        };
        RoundManager::generate_and_send_proposal(
            epoch_state.clone(),
            new_round_event,
            network.clone(),
            sync_info.clone(),
            proposal_generator.clone(),
            safety_rules.clone(),
            proposer_election.clone(),
        )
        .await
        .unwrap_err();

        // Ensure a proposal can be created if opt proposal is not
        let new_round_event = NewRoundEvent {
            round: 3,
            reason: NewRoundReason::QCReady,
            timeout: Default::default(),
            prev_round_votes: vec![],
            prev_round_timeout_votes: None,
        };
        RoundManager::generate_and_send_proposal(
            epoch_state,
            new_round_event,
            network,
            sync_info,
            proposal_generator,
            safety_rules,
            proposer_election,
        )
        .await
        .unwrap();
    });
}

/// Don't process an optimistic proposal if a normal proposal is processed.
#[test]
fn test_process_either_optimistic_or_normal_proposal() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        Some(config_with_opt_proposal_enabled()),
        None,
        None,
    );
    let genesis = nodes[0].block_store.ordered_root();
    let node = &mut nodes[0];

    timed_block_on(&runtime, async {
        let proposal_msg = node.next_proposal().await;
        node.round_manager
            .process_proposal_msg(proposal_msg.clone())
            .await
            .unwrap();

        let opt_block_data = OptBlockData::new(
            Vec::new(),
            Payload::empty(false, false),
            proposal_msg.proposer(),
            proposal_msg.proposal().epoch(),
            proposal_msg.proposal().round(),
            proposal_msg.proposal().timestamp_usecs(),
            proposal_msg.proposal().quorum_cert().parent_block().clone(),
            genesis.quorum_cert().clone(),
        );
        let opt_proposal_msg =
            OptProposalMsg::new(opt_block_data, proposal_msg.sync_info().clone());
        node.round_manager
            .process_opt_proposal_msg(opt_proposal_msg)
            .await
            .unwrap();
        let opt_block_data = node.processed_opt_proposal_rx.next().await.unwrap();

        let error = node
            .round_manager
            .process_opt_proposal(opt_block_data)
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Proposal has already been processed for round: 1"
        );
    })
}
