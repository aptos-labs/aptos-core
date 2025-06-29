// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
// Parts of the project are originally copyright © Meta Platforms, Inc.

use crate::{
    block_storage::BlockReader,
    counters,
    metrics_safety_rules::MetricsSafetyRules,
    network::IncomingBlockRetrievalRequest,
    network_interface::ConsensusMsg,
    network_tests::{NetworkPlayground, TwinId},
    round_manager::round_manager_tests::{
        config_with_round_timeout_msg_disabled, start_replying_to_block_retreival, NodeSetup,
        ProposalMsgType,
    },
    test_utils::{consensus_runtime, timed_block_on, TreeInserter},
};
use aptos_config::config::ConsensusConfig;
use aptos_consensus_types::{
    block::{
        block_test_utils::{certificate_for_genesis, gen_test_certificate},
        Block,
    },
    block_retrieval::{BlockRetrievalRequest, BlockRetrievalRequestV1, BlockRetrievalStatus},
    common::{Author, Payload, Round},
    opt_proposal_msg::OptProposalMsg,
    proposal_msg::ProposalMsg,
    sync_info::SyncInfo,
    timeout_2chain::{TwoChainTimeout, TwoChainTimeoutWithPartialSignatures},
};
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_logger::prelude::info;
use aptos_network::{protocols::network::Event, ProtocolId};
use aptos_safety_rules::{PersistentSafetyStorage, SafetyRulesManager};
use aptos_secure_storage::Storage;
use aptos_types::validator_verifier::generate_validator_verifier;
use futures::{channel::oneshot, StreamExt};
use std::{sync::Arc, time::Duration};
use tokio::{runtime::Runtime, time::timeout};

pub(super) fn process_and_vote_opt_proposal(
    runtime: &Runtime,
    node: &mut NodeSetup,
    opt_proposal_msg: OptProposalMsg,
    expected_round: Round,
    expected_qc_ordered_round: Round,
    expected_qc_committed_round: Round,
) {
    info!("Processing opt proposal on {}", node.identity_desc());

    assert_eq!(opt_proposal_msg.round(), expected_round);
    assert_eq!(
        opt_proposal_msg.sync_info().highest_ordered_round(),
        expected_qc_ordered_round.saturating_sub(1)
    );
    assert_eq!(
        opt_proposal_msg.sync_info().highest_commit_round(),
        expected_qc_committed_round
    );

    timed_block_on(
        runtime,
        node.round_manager
            .process_opt_proposal_msg(opt_proposal_msg),
    )
    .unwrap();
    info!("Finish process opt proposal on {}", node.identity_desc());

    info!(
        "Processing proposal (from opt proposal) on {}",
        node.identity_desc()
    );

    let opt_block_data = timed_block_on(runtime, node.processed_opt_proposal_rx.next()).unwrap();
    timed_block_on(
        runtime,
        node.round_manager.process_opt_proposal(opt_block_data),
    )
    .unwrap();

    info!(
        "Finish process proposal (from opt proposal) on {}",
        node.identity_desc()
    );
}

pub(super) fn process_and_vote_on_proposal(
    runtime: &Runtime,
    nodes: &mut [NodeSetup],
    next_proposer: usize,
    down_nodes: &[usize],
    process_votes: bool,
    apply_commit_prev_proposer: Option<usize>,
    apply_commit_on_votes: bool,
    expected_round: u64,
    expected_qc_ordered_round: u64,
    expected_qc_committed_round: u64,
) {
    info!(
        "Called {} with current {} and apply commit prev {:?}",
        expected_round, next_proposer, apply_commit_prev_proposer
    );
    let mut num_votes = 0;

    for node in nodes.iter_mut() {
        info!("Waiting on next_proposal on node {}", node.identity_desc());
        if down_nodes.contains(&node.id) {
            // Drop the proposal on down nodes
            timed_block_on(runtime, node.next_opt_or_normal_proposal());
            info!("Dropping proposal on down node {}", node.identity_desc());
        } else {
            // Proccess proposal on other nodes
            let proposal_msg_type = timed_block_on(runtime, node.next_opt_or_normal_proposal());
            match proposal_msg_type {
                ProposalMsgType::Normal(proposal_msg) => {
                    info!("Processing proposal on {}", node.identity_desc());

                    assert_eq!(proposal_msg.proposal().round(), expected_round);
                    assert_eq!(
                        proposal_msg.sync_info().highest_ordered_round(),
                        expected_qc_ordered_round
                    );
                    assert_eq!(
                        proposal_msg.sync_info().highest_commit_round(),
                        expected_qc_committed_round
                    );

                    timed_block_on(
                        runtime,
                        node.round_manager.process_proposal_msg(proposal_msg),
                    )
                    .unwrap();
                    info!("Finish process proposal on {}", node.identity_desc());
                },
                ProposalMsgType::Optimistic(opt_proposal_msg) => process_and_vote_opt_proposal(
                    runtime,
                    node,
                    opt_proposal_msg,
                    expected_round,
                    expected_qc_ordered_round,
                    expected_qc_committed_round,
                ),
            }
            num_votes += 1;
        }
    }

    for node in nodes.iter_mut() {
        info!(
            "Fetching {} votes in round {} on node {}",
            num_votes,
            expected_round,
            node.identity_desc()
        );
        if down_nodes.contains(&node.id) {
            // Drop the votes on down nodes
            info!("Dropping votes on down node {}", node.identity_desc());
            for _ in 0..num_votes {
                timed_block_on(runtime, node.next_vote());
            }
        } else {
            let mut votes = Vec::new();
            for _ in 0..num_votes {
                votes.push(timed_block_on(runtime, node.next_vote()));
            }

            info!("Processing votes on node {}", node.identity_desc());
            if process_votes {
                for vote_msg in votes {
                    timed_block_on(runtime, node.round_manager.process_vote_msg(vote_msg)).unwrap();
                }
                if apply_commit_prev_proposer.is_some()
                    && expected_round > 1
                    && apply_commit_on_votes
                {
                    info!(
                        "Applying next commit {} on proposer node {}",
                        expected_round - 2,
                        node.identity_desc()
                    );
                    timed_block_on(runtime, node.commit_next_ordered(&[expected_round - 1]));
                }
            }
        }
    }
}

#[test]
fn new_round_on_quorum_cert() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        None,
        None,
        None,
    );
    let node = &mut nodes[0];
    let genesis = node.block_store.ordered_root();
    timed_block_on(&runtime, async {
        // round 1 should start
        let proposal_msg = node.next_proposal().await;
        assert_eq!(
            proposal_msg.proposal().quorum_cert().certified_block().id(),
            genesis.id()
        );
        let b1_id = proposal_msg.proposal().id();
        assert_eq!(proposal_msg.proposer(), node.signer.author());

        node.round_manager
            .process_proposal_msg(proposal_msg)
            .await
            .unwrap();

        let vote_msg = node.next_vote().await;
        // Adding vote to form a QC
        node.round_manager.process_vote_msg(vote_msg).await.unwrap();

        // round 2 should start
        let proposal_msg_type = node.next_opt_or_normal_proposal().await;
        match proposal_msg_type {
            ProposalMsgType::Normal(proposal_msg) => {
                let proposal = proposal_msg.proposal();
                assert_eq!(proposal.round(), 2);
                assert_eq!(proposal.parent_id(), b1_id);
                assert_eq!(proposal.quorum_cert().certified_block().id(), b1_id);
            },
            ProposalMsgType::Optimistic(opt_proposal_msg) => {
                let proposal = opt_proposal_msg.block_data();
                assert_eq!(proposal.round(), 2);
                assert_eq!(proposal.parent_id(), b1_id);
                assert_eq!(
                    proposal.grandparent_qc().certified_block().id(),
                    genesis.id()
                );
            },
        }
    });
}

#[test]
/// If the proposal is valid, a vote should be sent
fn vote_on_successful_proposal() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    // In order to observe the votes we're going to check proposal processing on the non-proposer
    // node (which will send the votes to the proposer).
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        None,
        None,
        None,
    );
    let node = &mut nodes[0];

    let genesis_qc = certificate_for_genesis();
    timed_block_on(&runtime, async {
        // Start round 1 and clear the message queue
        node.next_proposal().await;

        let proposal = Block::new_proposal(
            Payload::empty(false, true),
            1,
            1,
            genesis_qc.clone(),
            &node.signer,
            Vec::new(),
        )
        .unwrap();
        let proposal_id = proposal.id();
        node.round_manager.process_proposal(proposal).await.unwrap();
        let vote_msg = node.next_vote().await;
        assert_eq!(vote_msg.vote().author(), node.signer.author());
        assert_eq!(vote_msg.vote().vote_data().proposed().id(), proposal_id);
        let consensus_state = node.round_manager.consensus_state();
        assert_eq!(consensus_state.epoch(), 1);
        assert_eq!(consensus_state.last_voted_round(), 1);
        assert_eq!(consensus_state.preferred_round(), 0);
        assert!(consensus_state.in_validator_set());
    });
}

#[test]
/// In back pressure mode, verify that the proposals are processed after we get out of back pressure.
fn delay_proposal_processing_in_sync_only() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    // In order to observe the votes we're going to check proposal processing on the non-proposer
    // node (which will send the votes to the proposer).
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        None,
        None,
        None,
    );
    let node = &mut nodes[0];

    let genesis_qc = certificate_for_genesis();
    timed_block_on(&runtime, async {
        // Start round 1 and clear the message queue
        node.next_proposal().await;

        // Set sync only to true so that new proposal processing is delayed.
        node.block_store.set_back_pressure_for_test(true);
        let proposal = Block::new_proposal(
            Payload::empty(false, true),
            1,
            1,
            genesis_qc.clone(),
            &node.signer,
            Vec::new(),
        )
        .unwrap();
        let proposal_id = proposal.id();
        node.round_manager
            .process_proposal(proposal.clone())
            .await
            .unwrap();

        // Wait for some time to ensure that the proposal was not processed
        timeout(Duration::from_millis(200), node.next_vote())
            .await
            .unwrap_err();

        // Clear the sync only mode and process verified proposal and ensure it is processed now
        node.block_store.set_back_pressure_for_test(false);

        node.round_manager
            .process_verified_proposal(proposal)
            .await
            .unwrap();

        let vote_msg = node.next_vote().await;
        assert_eq!(vote_msg.vote().author(), node.signer.author());
        assert_eq!(vote_msg.vote().vote_data().proposed().id(), proposal_id);
        let consensus_state = node.round_manager.consensus_state();
        assert_eq!(consensus_state.epoch(), 1);
        assert_eq!(consensus_state.last_voted_round(), 1);
        assert_eq!(consensus_state.preferred_round(), 0);
        assert!(consensus_state.in_validator_set());
    });
}

#[test]
/// If the proposal does not pass voting rules,
/// No votes are sent, but the block is still added to the block tree.
fn no_vote_on_old_proposal() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    // In order to observe the votes we're going to check proposal processing on the non-proposer
    // node (which will send the votes to the proposer).
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        None,
        None,
        None,
    );
    let node = &mut nodes[0];
    let genesis_qc = certificate_for_genesis();
    let new_block = Block::new_proposal(
        Payload::empty(false, true),
        1,
        1,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();
    let new_block_id = new_block.id();
    let old_block = Block::new_proposal(
        Payload::empty(false, true),
        1,
        2,
        genesis_qc,
        &node.signer,
        Vec::new(),
    )
    .unwrap();
    timed_block_on(&runtime, async {
        // clear the message queue
        node.next_proposal().await;

        node.round_manager
            .process_proposal(new_block)
            .await
            .unwrap();
        node.round_manager
            .process_proposal(old_block)
            .await
            .unwrap_err();
        let vote_msg = node.next_vote().await;
        assert_eq!(vote_msg.vote().vote_data().proposed().id(), new_block_id);
    });
}

#[test]
/// We don't vote for proposals that 'skips' rounds
/// After that when we then receive proposal for correct round, we vote for it
/// Basically it checks that adversary can not send proposal and skip rounds violating round_state
/// rules
fn no_vote_on_mismatch_round() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    // In order to observe the votes we're going to check proposal processing on the non-proposer
    // node (which will send the votes to the proposer).
    let mut node = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        None,
        None,
        None,
    )
    .pop()
    .unwrap();
    let genesis_qc = certificate_for_genesis();
    let correct_block = Block::new_proposal(
        Payload::empty(false, true),
        1,
        1,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();
    let block_skip_round = Block::new_proposal(
        Payload::empty(false, true),
        2,
        2,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();
    timed_block_on(&runtime, async {
        let bad_proposal = ProposalMsg::new(
            block_skip_round,
            SyncInfo::new(
                genesis_qc.clone(),
                genesis_qc.into_wrapped_ledger_info(),
                None,
            ),
        );
        assert!(node
            .round_manager
            .process_proposal_msg(bad_proposal)
            .await
            .is_err());
        let good_proposal = ProposalMsg::new(
            correct_block.clone(),
            SyncInfo::new(
                genesis_qc.clone(),
                genesis_qc.into_wrapped_ledger_info(),
                None,
            ),
        );
        node.round_manager
            .process_proposal_msg(good_proposal)
            .await
            .unwrap();
    });
}

#[test]
/// Ensure that after the vote messages are broadcasted upon timeout, the receivers
/// have the highest quorum certificate (carried by the SyncInfo of the vote message)
fn sync_info_carried_on_timeout_vote() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        Some(config_with_round_timeout_msg_disabled()),
        None,
        None,
    );
    let mut node = nodes.pop().unwrap();

    timed_block_on(&runtime, async {
        let proposal_msg = node.next_proposal().await;
        let block_0 = proposal_msg.proposal().clone();
        node.round_manager
            .process_proposal_msg(proposal_msg)
            .await
            .unwrap();
        node.next_vote().await;
        let parent_block_info = block_0.quorum_cert().certified_block();
        // Populate block_0 and a quorum certificate for block_0 on non_proposer
        let block_0_quorum_cert = gen_test_certificate(
            &[node.signer.clone()],
            // Follow MockStateComputer implementation
            block_0.gen_block_info(
                parent_block_info.executed_state_id(),
                parent_block_info.version(),
                parent_block_info.next_epoch_state().cloned(),
            ),
            parent_block_info.clone(),
            None,
        );
        node.block_store
            .insert_single_quorum_cert(block_0_quorum_cert.clone())
            .unwrap();

        node.round_manager.round_state.process_certificates(
            SyncInfo::new(
                block_0_quorum_cert.clone(),
                block_0_quorum_cert.into_wrapped_ledger_info(),
                None,
            ),
            &generate_validator_verifier(&[node.signer.clone()]),
        );
        node.round_manager
            .process_local_timeout(2)
            .await
            .unwrap_err();
        let vote_msg_on_timeout = node.next_vote().await;
        assert!(vote_msg_on_timeout.vote().is_timeout());
        assert_eq!(
            *vote_msg_on_timeout.sync_info().highest_quorum_cert(),
            block_0_quorum_cert
        );
    });
}

#[test]
/// We don't vote for proposals that comes from proposers that are not valid proposers for round
fn no_vote_on_invalid_proposer() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    // In order to observe the votes we're going to check proposal processing on the non-proposer
    // node (which will send the votes to the proposer).
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        2,
        None,
        None,
        None,
        None,
        None,
    );
    let incorrect_proposer = nodes.pop().unwrap();
    let mut node = nodes.pop().unwrap();
    let genesis_qc = certificate_for_genesis();
    let correct_block = Block::new_proposal(
        Payload::empty(false, true),
        1,
        1,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();
    let block_incorrect_proposer = Block::new_proposal(
        Payload::empty(false, true),
        1,
        1,
        genesis_qc.clone(),
        &incorrect_proposer.signer,
        Vec::new(),
    )
    .unwrap();
    timed_block_on(&runtime, async {
        let bad_proposal = ProposalMsg::new(
            block_incorrect_proposer,
            SyncInfo::new(
                genesis_qc.clone(),
                genesis_qc.into_wrapped_ledger_info(),
                None,
            ),
        );
        assert!(node
            .round_manager
            .process_proposal_msg(bad_proposal)
            .await
            .is_err());
        let good_proposal = ProposalMsg::new(
            correct_block.clone(),
            SyncInfo::new(
                genesis_qc.clone(),
                genesis_qc.into_wrapped_ledger_info(),
                None,
            ),
        );

        node.round_manager
            .process_proposal_msg(good_proposal.clone())
            .await
            .unwrap();
    });
}

#[test]
/// We allow to 'skip' round if proposal carries timeout certificate for next round
fn new_round_on_timeout_certificate() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    // In order to observe the votes we're going to check proposal processing on the non-proposer
    // node (which will send the votes to the proposer).
    let mut node = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        None,
        None,
        None,
    )
    .pop()
    .unwrap();
    let genesis_qc = certificate_for_genesis();
    let correct_block = Block::new_proposal(
        Payload::empty(false, true),
        1,
        1,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();
    let block_skip_round = Block::new_proposal(
        Payload::empty(false, true),
        2,
        2,
        genesis_qc.clone(),
        &node.signer,
        vec![(1, node.signer.author())],
    )
    .unwrap();
    let timeout = TwoChainTimeout::new(1, 1, genesis_qc.clone());
    let timeout_signature = timeout.sign(&node.signer).unwrap();

    let mut tc_partial = TwoChainTimeoutWithPartialSignatures::new(timeout.clone());
    tc_partial.add(node.signer.author(), timeout, timeout_signature);

    let tc = tc_partial
        .aggregate_signatures(&generate_validator_verifier(&[node.signer.clone()]))
        .unwrap();
    timed_block_on(&runtime, async {
        let skip_round_proposal = ProposalMsg::new(
            block_skip_round,
            SyncInfo::new(
                genesis_qc.clone(),
                genesis_qc.into_wrapped_ledger_info(),
                Some(tc),
            ),
        );
        node.round_manager
            .process_proposal_msg(skip_round_proposal)
            .await
            .unwrap();
        let old_good_proposal = ProposalMsg::new(
            correct_block.clone(),
            SyncInfo::new(
                genesis_qc.clone(),
                genesis_qc.into_wrapped_ledger_info(),
                None,
            ),
        );
        let before = counters::ERROR_COUNT.get();
        assert!(node
            .round_manager
            .process_proposal_msg(old_good_proposal)
            .await
            .is_ok()); // we eat the error
        assert_eq!(counters::ERROR_COUNT.get(), before + 1); // but increase the counter
    });
}

#[test]
/// We allow to 'skip' round if proposal carries timeout certificate for next round
fn reject_invalid_failed_authors() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    // In order to observe the votes we're going to check proposal processing on the non-proposer
    // node (which will send the votes to the proposer).
    let mut node = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        None,
        None,
        None,
    )
    .pop()
    .unwrap();
    let genesis_qc = certificate_for_genesis();

    let create_timeout = |round: Round| {
        let timeout = TwoChainTimeout::new(1, round, genesis_qc.clone());
        let timeout_signature = timeout.sign(&node.signer).unwrap();
        let mut tc_partial = TwoChainTimeoutWithPartialSignatures::new(timeout.clone());
        tc_partial.add(node.signer.author(), timeout, timeout_signature);

        tc_partial
            .aggregate_signatures(&generate_validator_verifier(&[node.signer.clone()]))
            .unwrap()
    };

    let create_proposal = |round: Round, failed_authors: Vec<(Round, Author)>| {
        let block = Block::new_proposal(
            Payload::empty(false, true),
            round,
            2,
            genesis_qc.clone(),
            &node.signer,
            failed_authors,
        )
        .unwrap();
        ProposalMsg::new(
            block,
            SyncInfo::new(
                genesis_qc.clone(),
                genesis_qc.into_wrapped_ledger_info(),
                if round > 1 {
                    Some(create_timeout(round - 1))
                } else {
                    None
                },
            ),
        )
    };

    let extra_failed_authors_proposal = create_proposal(2, vec![(1, Author::random())]);
    let missing_failed_authors_proposal = create_proposal(2, vec![]);
    let wrong_failed_authors_proposal = create_proposal(2, vec![(1, Author::random())]);
    let not_enough_failed_proposal = create_proposal(3, vec![(2, node.signer.author())]);
    let valid_proposal = create_proposal(
        4,
        (1..4).map(|i| (i as Round, node.signer.author())).collect(),
    );

    timed_block_on(&runtime, async {
        assert!(node
            .round_manager
            .process_proposal_msg(extra_failed_authors_proposal)
            .await
            .is_err());

        assert!(node
            .round_manager
            .process_proposal_msg(missing_failed_authors_proposal)
            .await
            .is_err());
    });

    timed_block_on(&runtime, async {
        assert!(node
            .round_manager
            .process_proposal_msg(wrong_failed_authors_proposal)
            .await
            .is_err());

        assert!(node
            .round_manager
            .process_proposal_msg(not_enough_failed_proposal)
            .await
            .is_err());

        node.round_manager
            .process_proposal_msg(valid_proposal)
            .await
            .unwrap()
    });
}

#[test]
fn response_on_block_retrieval() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut node = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        None,
        None,
        None,
    )
    .pop()
    .unwrap();

    let genesis_qc = certificate_for_genesis();
    let block = Block::new_proposal(
        Payload::empty(false, true),
        1,
        1,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();
    let block_id = block.id();
    let proposal = ProposalMsg::new(
        block,
        SyncInfo::new(
            genesis_qc.clone(),
            genesis_qc.into_wrapped_ledger_info(),
            None,
        ),
    );

    timed_block_on(&runtime, async {
        node.round_manager
            .process_proposal_msg(proposal)
            .await
            .unwrap();

        // first verify that we can retrieve the block if it's in the tree
        let (tx1, rx1) = oneshot::channel();
        let single_block_request = IncomingBlockRetrievalRequest {
            req: BlockRetrievalRequest::V1(BlockRetrievalRequestV1::new(block_id, 1)),
            protocol: ProtocolId::ConsensusRpcBcs,
            response_sender: tx1,
        };
        node.block_store
            .process_block_retrieval(single_block_request)
            .await
            .unwrap();
        match rx1.await {
            Ok(Ok(bytes)) => {
                let response = match bcs::from_bytes(&bytes) {
                    Ok(ConsensusMsg::BlockRetrievalResponse(resp)) => *resp,
                    _ => panic!("block retrieval failure"),
                };
                assert_eq!(response.status(), BlockRetrievalStatus::Succeeded);
                assert_eq!(response.blocks().first().unwrap().id(), block_id);
            },
            _ => panic!("block retrieval failure"),
        }

        // verify that if a block is not there, return ID_NOT_FOUND
        let (tx2, rx2) = oneshot::channel();
        let missing_block_request = IncomingBlockRetrievalRequest {
            req: BlockRetrievalRequest::V1(BlockRetrievalRequestV1::new(HashValue::random(), 1)),
            protocol: ProtocolId::ConsensusRpcBcs,
            response_sender: tx2,
        };

        node.block_store
            .process_block_retrieval(missing_block_request)
            .await
            .unwrap();
        match rx2.await {
            Ok(Ok(bytes)) => {
                let response = match bcs::from_bytes(&bytes) {
                    Ok(ConsensusMsg::BlockRetrievalResponse(resp)) => *resp,
                    _ => panic!("block retrieval failure"),
                };
                assert_eq!(response.status(), BlockRetrievalStatus::IdNotFound);
                assert!(response.blocks().is_empty());
            },
            _ => panic!("block retrieval failure"),
        }

        // if asked for many blocks, return NOT_ENOUGH_BLOCKS
        let (tx3, rx3) = oneshot::channel();
        let many_block_request = IncomingBlockRetrievalRequest {
            req: BlockRetrievalRequest::V1(BlockRetrievalRequestV1::new(block_id, 3)),
            protocol: ProtocolId::ConsensusRpcBcs,
            response_sender: tx3,
        };
        node.block_store
            .process_block_retrieval(many_block_request)
            .await
            .unwrap();
        match rx3.await {
            Ok(Ok(bytes)) => {
                let response = match bcs::from_bytes(&bytes) {
                    Ok(ConsensusMsg::BlockRetrievalResponse(resp)) => *resp,
                    _ => panic!("block retrieval failure"),
                };
                assert_eq!(response.status(), BlockRetrievalStatus::NotEnoughBlocks);
                assert_eq!(block_id, response.blocks().first().unwrap().id());
                assert_eq!(
                    node.block_store.ordered_root().id(),
                    response.blocks().get(1).unwrap().id()
                );
            },
            _ => panic!("block retrieval failure"),
        }
    });
}

#[test]
/// rebuild a node from previous storage without violating safety guarantees.
fn recover_on_restart() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut node = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        None,
        None,
        None,
    )
    .pop()
    .unwrap();
    let inserter = TreeInserter::new_with_store(node.signer.clone(), node.block_store.clone());

    let genesis_qc = certificate_for_genesis();
    let mut data = Vec::new();
    let num_proposals = 100;
    // insert a few successful proposals
    for i in 1..=num_proposals {
        let proposal = inserter.create_block_with_qc(
            genesis_qc.clone(),
            i,
            i,
            Payload::empty(false, true),
            (std::cmp::max(1, i.saturating_sub(10))..i)
                .map(|i| (i, inserter.signer().author()))
                .collect(),
        );
        let timeout = TwoChainTimeout::new(1, i - 1, genesis_qc.clone());
        let mut tc_partial = TwoChainTimeoutWithPartialSignatures::new(timeout.clone());
        tc_partial.add(
            inserter.signer().author(),
            timeout.clone(),
            timeout.sign(inserter.signer()).unwrap(),
        );

        let tc = tc_partial
            .aggregate_signatures(&generate_validator_verifier(&[node.signer.clone()]))
            .unwrap();

        data.push((proposal, tc));
    }

    timed_block_on(&runtime, async {
        for (proposal, tc) in &data {
            let proposal_msg = ProposalMsg::new(
                proposal.clone(),
                SyncInfo::new(
                    proposal.quorum_cert().clone(),
                    genesis_qc.into_wrapped_ledger_info(),
                    Some(tc.clone()),
                ),
            );
            node.round_manager
                .process_proposal_msg(proposal_msg)
                .await
                .unwrap();
        }
    });

    // verify after restart we recover the data
    node = node.restart(&mut playground, runtime.handle().clone());
    let consensus_state = node.round_manager.consensus_state();
    assert_eq!(consensus_state.epoch(), 1);
    assert_eq!(consensus_state.last_voted_round(), num_proposals);
    assert_eq!(consensus_state.preferred_round(), 0);
    assert!(consensus_state.in_validator_set());
    for (block, _) in data {
        assert!(node.block_store.block_exists(block.id()));
    }
}

#[test]
/// Generate a NIL vote extending HQC upon timeout if no votes have been sent in the round.
fn nil_vote_on_timeout() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        Some(config_with_round_timeout_msg_disabled()),
        None,
        None,
    );
    let node = &mut nodes[0];
    let genesis = node.block_store.ordered_root();
    timed_block_on(&runtime, async {
        node.next_proposal().await;
        // Process the outgoing vote message and verify that it contains a round signature
        // and that the vote extends genesis.
        node.round_manager
            .process_local_timeout(1)
            .await
            .unwrap_err();
        let vote_msg = node.next_vote().await;

        let vote = vote_msg.vote();

        assert!(vote.is_timeout());
        // NIL block doesn't change timestamp
        assert_eq!(
            vote.vote_data().proposed().timestamp_usecs(),
            genesis.timestamp_usecs()
        );
        assert_eq!(vote.vote_data().proposed().round(), 1);
        assert_eq!(
            vote.vote_data().parent().id(),
            node.block_store.ordered_root().id()
        );
    });
}

#[test]
/// Generate a Timeout upon timeout if no votes have been sent in the round.
fn timeout_round_on_timeout() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let local_config = ConsensusConfig {
        enable_round_timeout_msg: true,
        ..Default::default()
    };
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        Some(local_config),
        None,
        None,
    );
    let node = &mut nodes[0];
    let genesis = node.block_store.ordered_root();
    timed_block_on(&runtime, async {
        node.next_proposal().await;
        // Process the outgoing vote message and verify that it contains a round signature
        // and that the vote extends genesis.
        node.round_manager
            .process_local_timeout(1)
            .await
            .unwrap_err();
        let timeout_msg = node.next_timeout().await;

        let timeout = timeout_msg.timeout();

        assert_eq!(timeout.round(), 1);
        assert_eq!(timeout.author(), node.signer.author());
        assert_eq!(timeout.epoch(), 1);
        assert_eq!(timeout.two_chain_timeout().hqc_round(), genesis.round());
    });
}

#[test]
/// If the node votes in a round, upon timeout the same vote is re-sent with a timeout signature.
fn vote_resent_on_timeout() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        Some(config_with_round_timeout_msg_disabled()),
        None,
        None,
    );
    let node = &mut nodes[0];
    timed_block_on(&runtime, async {
        let proposal_msg = node.next_proposal().await;
        let id = proposal_msg.proposal().id();
        node.round_manager
            .process_proposal_msg(proposal_msg)
            .await
            .unwrap();
        let vote_msg = node.next_vote().await;
        let vote = vote_msg.vote();
        assert!(!vote.is_timeout());
        assert_eq!(vote.vote_data().proposed().id(), id);
        // Process the outgoing vote message and verify that it contains a round signature
        // and that the vote is the same as above.
        node.round_manager
            .process_local_timeout(1)
            .await
            .unwrap_err();
        let timeout_vote_msg = node.next_vote().await;
        let timeout_vote = timeout_vote_msg.vote();

        assert!(timeout_vote.is_timeout());
        assert_eq!(timeout_vote.vote_data(), vote.vote_data());
    });
}

#[test]
/// If the node votes in a round, upon timeout the same vote is re-sent with a timeout signature.
fn timeout_sent_on_timeout_after_vote() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let local_config = ConsensusConfig {
        enable_round_timeout_msg: true,
        ..Default::default()
    };
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        Some(local_config),
        None,
        None,
    );
    let node = &mut nodes[0];
    timed_block_on(&runtime, async {
        let proposal_msg = node.next_proposal().await;
        let id = proposal_msg.proposal().id();
        node.round_manager
            .process_proposal_msg(proposal_msg)
            .await
            .unwrap();
        let vote_msg = node.next_vote().await;
        let vote = vote_msg.vote();
        assert!(!vote.is_timeout());
        assert_eq!(vote.vote_data().proposed().id(), id);
        // Process the outgoing vote message and verify that it contains a round signature
        // and that the vote is the same as above.
        node.round_manager
            .process_local_timeout(1)
            .await
            .unwrap_err();
        let timeout_msg = node.next_timeout().await;

        assert_eq!(timeout_msg.round(), vote.vote_data().proposed().round());
        assert_eq!(timeout_msg.sync_info(), vote_msg.sync_info());
    });
}

#[test]
#[ignore] // TODO: this test needs to be fixed!
fn sync_on_partial_newer_sync_info() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        None,
        None,
        None,
    );
    let mut node = nodes.pop().unwrap();
    runtime.spawn(playground.start());
    timed_block_on(&runtime, async {
        // commit block 1 after 4 rounds
        for _ in 1..=4 {
            let proposal_msg = node.next_proposal().await;

            node.round_manager
                .process_proposal_msg(proposal_msg)
                .await
                .unwrap();
            let vote_msg = node.next_vote().await;
            // Adding vote to form a QC
            node.round_manager.process_vote_msg(vote_msg).await.unwrap();
        }
        let block_4 = node.next_proposal().await;
        node.round_manager
            .process_proposal_msg(block_4.clone())
            .await
            .unwrap();
        // commit genesis and block 1
        for i in 0..2 {
            node.commit_next_ordered(&[i]).await;
        }
        let vote_msg = node.next_vote().await;
        let vote_data = vote_msg.vote().vote_data();
        let block_4_qc = gen_test_certificate(
            &[node.signer.clone()],
            vote_data.proposed().clone(),
            vote_data.parent().clone(),
            None,
        );
        // Create a sync info with newer quorum cert but older commit cert
        let sync_info = SyncInfo::new(
            block_4_qc.clone(),
            certificate_for_genesis().into_wrapped_ledger_info(),
            None,
        );
        node.round_manager
            .ensure_round_and_sync_up(
                sync_info.highest_round() + 1,
                &sync_info,
                node.signer.author(),
            )
            .await
            .unwrap();
        // QuorumCert added
        assert_eq!(*node.block_store.highest_quorum_cert(), block_4_qc);
    });
}

#[test]
fn safety_rules_crash() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        Some(config_with_round_timeout_msg_disabled()),
        None,
        None,
    );
    let mut node = nodes.pop().unwrap();
    runtime.spawn(playground.start());

    fn reset_safety_rules(node: &mut NodeSetup) {
        let safety_storage = PersistentSafetyStorage::initialize(
            Storage::from(aptos_secure_storage::InMemoryStorage::new()),
            node.signer.author(),
            node.signer.private_key().clone(),
            node.round_manager.consensus_state().waypoint(),
            true,
        );

        node.safety_rules_manager = SafetyRulesManager::new_local(safety_storage);
        let safety_rules =
            MetricsSafetyRules::new(node.safety_rules_manager.client(), node.storage.clone());
        let safety_rules_container = Arc::new(Mutex::new(safety_rules));
        node.round_manager.set_safety_rules(safety_rules_container);
    }

    timed_block_on(&runtime, async {
        for _ in 0..2 {
            let proposal_msg_type = node.next_opt_or_normal_proposal().await;

            reset_safety_rules(&mut node);

            match proposal_msg_type {
                ProposalMsgType::Normal(proposal_msg) => {
                    // construct_and_sign_vote
                    node.round_manager
                        .process_proposal_msg(proposal_msg)
                        .await
                        .unwrap();
                },
                ProposalMsgType::Optimistic(opt_proposal_msg) => {
                    node.round_manager
                        .process_opt_proposal_msg(opt_proposal_msg)
                        .await
                        .unwrap();
                    let opt_block_data = node.processed_opt_proposal_rx.next().await.unwrap();
                    node.round_manager
                        .process_opt_proposal(opt_block_data)
                        .await
                        .unwrap();
                },
            }

            let vote_msg = node.next_vote().await;

            // sign_timeout
            reset_safety_rules(&mut node);
            let round = vote_msg.vote().vote_data().proposed().round();
            node.round_manager
                .process_local_timeout(round)
                .await
                .unwrap_err();
            let vote_msg = node.next_vote().await;

            // sign proposal
            reset_safety_rules(&mut node);
            node.round_manager.process_vote_msg(vote_msg).await.unwrap();
        }

        // verify the last sign proposal happened
        node.next_opt_or_normal_proposal().await;
    });
}

#[test]
fn echo_timeout() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        4,
        None,
        None,
        Some(config_with_round_timeout_msg_disabled()),
        None,
        None,
    );
    runtime.spawn(playground.start());
    timed_block_on(&runtime, async {
        // clear the message queue
        for node in &mut nodes {
            node.next_proposal().await;
        }
        // timeout 3 nodes
        for node in &mut nodes[1..] {
            node.round_manager
                .process_local_timeout(1)
                .await
                .unwrap_err();
        }
        let node_0 = &mut nodes[0];
        // node 0 doesn't timeout and should echo the timeout after 2 timeout message
        for i in 0..3 {
            let timeout_vote = node_0.next_vote().await;
            let result = node_0.round_manager.process_vote_msg(timeout_vote).await;
            // first and third message should not timeout
            if i == 0 || i == 2 {
                assert!(result.is_ok());
            }
            if i == 1 {
                // timeout is an Error
                assert!(result.is_err());
            }
        }

        let node_1 = &mut nodes[1];
        // it receives 4 timeout messages (1 from each) and doesn't echo since it already timeout
        for _ in 0..4 {
            let timeout_vote = node_1.next_vote().await;
            // Verifying only some vote messages to check that round manager can accept both
            // verified and unverified votes
            node_1
                .round_manager
                .process_vote_msg(timeout_vote)
                .await
                .unwrap();
        }
    });
}

#[test]
fn echo_round_timeout_msg() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        4,
        None,
        None,
        None,
        None,
        None,
    );
    runtime.spawn(playground.start());
    timed_block_on(&runtime, async {
        // clear the message queue
        for node in &mut nodes {
            node.next_proposal().await;
        }
        // timeout 3 nodes
        for node in &mut nodes[1..] {
            node.round_manager
                .process_local_timeout(1)
                .await
                .unwrap_err();
        }
        let node_0 = &mut nodes[0];
        // node 0 doesn't timeout and should echo the timeout after 2 timeout message
        for i in 0..3 {
            let timeout_vote = node_0.next_timeout().await;
            let result = node_0
                .round_manager
                .process_round_timeout_msg(timeout_vote)
                .await;
            // first and third message should not timeout
            if i == 0 || i == 2 {
                assert!(result.is_ok());
            }
            if i == 1 {
                // timeout is an Error
                assert!(result.is_err());
            }
        }

        let node_1 = &mut nodes[1];
        // it receives 4 timeout messages (1 from each) and doesn't echo since it already timeout
        for _ in 0..4 {
            let timeout_vote = node_1.next_timeout().await;
            node_1
                .round_manager
                .process_round_timeout_msg(timeout_vote)
                .await
                .unwrap();
        }
    });
}

#[test]
fn no_next_test() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        4,
        None,
        None,
        None,
        None,
        None,
    );
    runtime.spawn(playground.start());

    timed_block_on(&runtime, async {
        // clear the message queue
        for node in &mut nodes {
            node.next_proposal().await;
        }

        tokio::time::sleep(Duration::from_secs(1)).await;

        for node in nodes.iter_mut() {
            node.no_next_msg();
        }
        tokio::time::sleep(Duration::from_secs(1)).await;

        for node in nodes.iter_mut() {
            node.no_next_msg();
        }
    });
}

#[test]
fn commit_pipeline_test() {
    let runtime = consensus_runtime();
    let proposers = vec![0, 0, 0, 0, 5];

    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        7,
        Some(proposers.clone()),
        None,
        None,
        None,
        None,
    );
    runtime.spawn(playground.start());
    let behind_node = 6;
    for i in 0..10 {
        let next_proposer = proposers[(i + 2) as usize % proposers.len()];
        let prev_proposer = proposers[(i + 1) as usize % proposers.len()];
        info!("processing {}", i);
        process_and_vote_on_proposal(
            &runtime,
            &mut nodes,
            next_proposer,
            &[behind_node],
            true,
            Some(prev_proposer),
            true,
            i + 1,
            i.saturating_sub(1),
            i.saturating_sub(2),
        );

        std::thread::sleep(Duration::from_secs(1));

        for node in nodes.iter_mut() {
            node.no_next_ordered();
        }
    }
}

#[test]
fn block_retrieval_test() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        4,
        Some(vec![0, 1]),
        None,
        None,
        None,
        None,
    );
    runtime.spawn(playground.start());

    for i in 0..4 {
        info!("processing {}", i);
        process_and_vote_on_proposal(
            &runtime,
            &mut nodes,
            i as usize % 2,
            &[3],
            true,
            None,
            true,
            i + 1,
            i.saturating_sub(1),
            0,
        );
    }

    timed_block_on(&runtime, async {
        let mut behind_node = nodes.pop().unwrap();

        // Drain the queue on other nodes
        for node in nodes.iter_mut() {
            let _ = node.next_opt_or_normal_proposal().await;
        }

        info!(
            "Processing proposals for behind node {}",
            behind_node.identity_desc()
        );
        let handle = start_replying_to_block_retreival(nodes);

        let proposal_msg_type = behind_node.next_opt_or_normal_proposal().await;
        info!("got proposal msg: {:?}", proposal_msg_type);
        match proposal_msg_type {
            ProposalMsgType::Normal(proposal_msg) => behind_node
                .round_manager
                .process_proposal_msg(proposal_msg)
                .await
                .unwrap(),
            ProposalMsgType::Optimistic(opt_proposal_msg) => {
                behind_node
                    .round_manager
                    .process_opt_proposal_msg(opt_proposal_msg)
                    .await
                    .unwrap();
            },
        }

        handle.join().await;
    });
}

#[test]
fn block_retrieval_timeout_test() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        4,
        Some(vec![0, 1]),
        None,
        None,
        None,
        None,
    );
    let timeout_config = playground.timeout_config();
    runtime.spawn(playground.start());

    for i in 0..4 {
        info!("processing {}", i);
        process_and_vote_on_proposal(
            &runtime,
            &mut nodes,
            i as usize % 2,
            &[3],
            true,
            None,
            true,
            i + 1,
            i.saturating_sub(1),
            0,
        );
    }

    timed_block_on(&runtime, async {
        let mut behind_node = nodes.pop().unwrap();

        for node in nodes.iter() {
            timeout_config.write().timeout_message_for(
                &TwinId {
                    id: behind_node.id,
                    author: behind_node.signer.author(),
                },
                &TwinId {
                    id: node.id,
                    author: node.signer.author(),
                },
            );
        }

        // Drain the queue on other nodes
        for node in nodes.iter_mut() {
            let _ = node.next_opt_or_normal_proposal().await;
        }

        info!(
            "Processing proposals for behind node {}",
            behind_node.identity_desc()
        );

        let proposal_msg_type = behind_node.next_opt_or_normal_proposal().await;
        match proposal_msg_type {
            ProposalMsgType::Normal(proposal_msg) => {
                behind_node
                    .round_manager
                    .process_proposal_msg(proposal_msg)
                    .await
                    .unwrap_err();
            },
            ProposalMsgType::Optimistic(opt_proposal_msg) => {
                behind_node
                    .round_manager
                    .process_opt_proposal_msg(opt_proposal_msg)
                    .await
                    .unwrap_err();
            },
        }
    });
}

#[ignore] // TODO: turn this test back on once the flakes have resolved.
#[test]
pub fn forking_retrieval_test() {
    let runtime = consensus_runtime();

    let proposal_node = 0;
    let behind_node = 6;
    let forking_node = 5;

    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        7,
        Some(vec![
            proposal_node,
            proposal_node,
            proposal_node,
            proposal_node,
            proposal_node,
            forking_node,
            proposal_node,
            proposal_node,
        ]),
        None,
        None,
        None,
        None,
    );
    runtime.spawn(playground.start());

    info!("Propose vote and commit on first block");
    process_and_vote_on_proposal(
        &runtime,
        &mut nodes,
        proposal_node,
        &[behind_node],
        true,
        Some(proposal_node),
        true,
        1,
        0,
        0,
    );

    info!("Propose vote and commit on second block");
    process_and_vote_on_proposal(
        &runtime,
        &mut nodes,
        proposal_node,
        &[behind_node],
        true,
        Some(proposal_node),
        true,
        2,
        0,
        0,
    );

    info!("Propose vote and commit on second block");
    process_and_vote_on_proposal(
        &runtime,
        &mut nodes,
        proposal_node,
        &[behind_node],
        true,
        Some(proposal_node),
        true,
        3,
        1,
        0,
    );

    info!("Propose vote and commit on third (dangling) block");
    process_and_vote_on_proposal(
        &runtime,
        &mut nodes,
        forking_node,
        &[behind_node, forking_node],
        false,
        Some(proposal_node),
        true,
        4,
        2,
        1,
    );

    timed_block_on(&runtime, async {
        info!("Insert local timeout to all nodes on next round");
        let mut timeout_votes = 0;
        for node in nodes.iter_mut() {
            if node.id != behind_node && node.id != forking_node {
                node.round_manager
                    .process_local_timeout(4)
                    .await
                    .unwrap_err();
                timeout_votes += 1;
            }
        }

        info!("Process all local timeouts");
        for node in nodes.iter_mut() {
            info!("Timeouts on {}", node.id);
            for i in 0..timeout_votes {
                info!("Timeout {} on {}", i, node.id);
                if node.id == forking_node && (2..4).contains(&i) {
                    info!("Got {}", node.next_commit_decision().await);
                }

                let vote_msg_on_timeout = node.next_vote().await;
                assert!(vote_msg_on_timeout.vote().is_timeout());
                if node.id != behind_node {
                    let result = node
                        .round_manager
                        .process_vote_msg(vote_msg_on_timeout)
                        .await;

                    if node.id == forking_node && i == 2 {
                        result.unwrap_err();
                    } else {
                        result.unwrap();
                    }
                }
            }
        }
    });

    timed_block_on(&runtime, async {
        for node in nodes.iter_mut() {
            let vote_msg_on_timeout = node.next_vote().await;
            assert!(vote_msg_on_timeout.vote().is_timeout());
        }

        info!("Got {}", nodes[forking_node].next_commit_decision().await);
    });

    info!("Create forked block");
    process_and_vote_on_proposal(
        &runtime,
        &mut nodes,
        proposal_node,
        &[behind_node],
        true,
        Some(forking_node),
        false,
        5,
        2,
        1,
    );

    process_and_vote_on_proposal(
        &runtime,
        &mut nodes,
        proposal_node,
        &[behind_node, forking_node],
        true,
        None,
        false,
        6,
        3,
        3,
    );

    process_and_vote_on_proposal(
        &runtime,
        &mut nodes,
        proposal_node,
        &[behind_node, forking_node],
        true,
        None,
        false,
        7,
        5,
        3,
    );

    let mut nodes = timed_block_on(&runtime, async {
        let mut behind_node_obj = nodes.pop().unwrap();

        // Drain the queue on other nodes
        let mut proposals = Vec::new();
        for node in nodes.iter_mut() {
            proposals.push(node.next_proposal().await);
        }

        info!(
            "Processing proposals for behind node {}",
            behind_node_obj.identity_desc()
        );
        let handle = start_replying_to_block_retreival(nodes);
        let proposal_msg = behind_node_obj.next_proposal().await;
        behind_node_obj
            .round_manager
            .process_proposal_msg(proposal_msg.clone())
            .await
            .unwrap();

        nodes = handle.join().await;
        behind_node_obj.no_next_msg();

        for (proposal, node) in proposals.into_iter().zip(nodes.iter_mut()) {
            node.pending_network_events.push(Event::Message(
                node.signer.author(),
                ConsensusMsg::ProposalMsg(Box::new(proposal)),
            ));
        }
        behind_node_obj.pending_network_events.push(Event::Message(
            behind_node_obj.signer.author(),
            ConsensusMsg::ProposalMsg(Box::new(proposal_msg)),
        ));

        nodes.push(behind_node_obj);
        nodes
    });

    // confirm behind node can participate in consensus after state sync
    process_and_vote_on_proposal(
        &runtime,
        &mut nodes,
        proposal_node,
        &[forking_node, behind_node],
        true,
        None,
        false,
        8,
        6,
        3,
    );

    // let next_message = timed_block_on(&runtime, nodes[proposal_node].next_network_message());
    // match next_message {
    //     ConsensusMsg::VoteMsg(_) => info!("Skip extra vote msg"),
    //     ConsensusMsg::ProposalMsg(msg) => {
    //         // put the message back in the queue.
    //         // actual peer doesn't matter, it is ignored, so use self.
    //         let peer = nodes[proposal_node].signer.author();
    //         nodes[proposal_node]
    //             .pending_network_events
    //             .push(Event::Message(peer, ConsensusMsg::ProposalMsg(msg)))
    //     },
    //     _ => panic!("unexpected network message {:?}", next_message),
    // }
    process_and_vote_on_proposal(
        &runtime,
        &mut nodes,
        proposal_node,
        &[forking_node],
        true,
        None,
        false,
        9,
        7,
        3,
    );
}
