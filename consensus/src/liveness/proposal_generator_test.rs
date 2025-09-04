// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::BlockReader,
    liveness::{
        proposal_generator::{
            ChainHealthBackoffConfig, PipelineBackpressureConfig, ProposalGenerator,
        },
        proposal_status_tracker::TOptQSPullParamsProvider,
        rotating_proposer_election::RotatingProposer,
        unequivocal_proposer_election::UnequivocalProposerElection,
    },
    test_utils::{build_default_empty_tree, MockPayloadManager, TreeInserter},
    util::mock_time_service::SimulatedTimeService,
};
use velor_consensus_types::{
    block::{block_test_utils::certificate_for_genesis, Block},
    common::Author,
    payload_pull_params::OptQSPayloadPullParams,
    utils::PayloadTxnsSize,
};
use velor_types::{on_chain_config::ValidatorTxnConfig, validator_signer::ValidatorSigner};
use std::{sync::Arc, time::Duration};

const MAX_BLOCK_GAS_LIMIT: u64 = 30_000;

struct MockOptQSPayloadProvider {}

impl TOptQSPullParamsProvider for MockOptQSPayloadProvider {
    fn get_params(&self) -> Option<OptQSPayloadPullParams> {
        None
    }
}

#[tokio::test]
async fn test_proposal_generation_empty_tree() {
    let signer = ValidatorSigner::random(None);
    let block_store = build_default_empty_tree();
    let proposal_generator = ProposalGenerator::new(
        signer.author(),
        block_store.clone(),
        Arc::new(MockPayloadManager::new(None)),
        Arc::new(SimulatedTimeService::new()),
        Duration::ZERO,
        PayloadTxnsSize::new(1, 10),
        1,
        PayloadTxnsSize::new(1, 10),
        10,
        1,
        Some(MAX_BLOCK_GAS_LIMIT),
        PipelineBackpressureConfig::new_no_backoff(),
        ChainHealthBackoffConfig::new_no_backoff(),
        false,
        ValidatorTxnConfig::default_disabled(),
        true,
        Arc::new(MockOptQSPayloadProvider {}),
    );
    let proposer_election = Arc::new(UnequivocalProposerElection::new(Arc::new(
        RotatingProposer::new(vec![signer.author()], 1),
    )));
    let genesis = block_store.ordered_root();

    // Generate proposals for an empty tree.
    let proposal_data = proposal_generator
        .generate_proposal(1, proposer_election.clone())
        .await
        .unwrap();
    let proposal = Block::new_proposal_from_block_data(proposal_data, &signer).unwrap();
    assert_eq!(proposal.parent_id(), genesis.id());
    assert_eq!(proposal.round(), 1);
    assert_eq!(proposal.quorum_cert().certified_block().id(), genesis.id());
    assert_eq!(proposal.block_data().failed_authors().unwrap().len(), 0);

    // Duplicate proposals on the same round are not allowed
    let proposal_err = proposal_generator
        .generate_proposal(1, proposer_election.clone())
        .await
        .err();
    assert!(proposal_err.is_some());
}

#[tokio::test]
async fn test_proposal_generation_parent() {
    let mut inserter = TreeInserter::default();
    let block_store = inserter.block_store();
    let proposal_generator = ProposalGenerator::new(
        inserter.signer().author(),
        block_store.clone(),
        Arc::new(MockPayloadManager::new(None)),
        Arc::new(SimulatedTimeService::new()),
        Duration::ZERO,
        PayloadTxnsSize::new(1, 1000),
        1,
        PayloadTxnsSize::new(1, 500),
        10,
        1,
        Some(MAX_BLOCK_GAS_LIMIT),
        PipelineBackpressureConfig::new_no_backoff(),
        ChainHealthBackoffConfig::new_no_backoff(),
        false,
        ValidatorTxnConfig::default_disabled(),
        true,
        Arc::new(MockOptQSPayloadProvider {}),
    );
    let proposer_election = Arc::new(UnequivocalProposerElection::new(Arc::new(
        RotatingProposer::new(vec![inserter.signer().author()], 1),
    )));
    let genesis = block_store.ordered_root();
    let a1 = inserter
        .insert_block_with_qc(certificate_for_genesis(), &genesis, 1)
        .await;
    let b1 = inserter
        .insert_block_with_qc(certificate_for_genesis(), &genesis, 2)
        .await;

    let original_res = proposal_generator
        .generate_proposal(10, proposer_election.clone())
        .await
        .unwrap();
    // With no certifications the parent is genesis
    // generate proposals for an empty tree.
    assert_eq!(original_res.parent_id(), genesis.id());
    // test that we have authors for the skipped rounds (1, 2, .. 9)
    assert_eq!(original_res.failed_authors().unwrap().len(), 9);
    assert_eq!(original_res.failed_authors().unwrap().first().unwrap().0, 1);
    assert_eq!(original_res.failed_authors().unwrap().last().unwrap().0, 9);

    // Once a1 is certified, it should be the one to choose from
    inserter.insert_qc_for_block(a1.as_ref(), None);
    let a1_child_res = proposal_generator
        .generate_proposal(11, proposer_election.clone())
        .await
        .unwrap();
    assert_eq!(a1_child_res.parent_id(), a1.id());
    assert_eq!(a1_child_res.round(), 11);
    assert_eq!(a1_child_res.quorum_cert().certified_block().id(), a1.id());

    // test that we have authors for the skipped rounds (2, 3, .. 10)
    assert_eq!(a1_child_res.failed_authors().unwrap().len(), 9);
    assert_eq!(a1_child_res.failed_authors().unwrap().first().unwrap().0, 2);
    assert_eq!(a1_child_res.failed_authors().unwrap().last().unwrap().0, 10);

    // Once b1 is certified, it should be the one to choose from
    inserter.insert_qc_for_block(b1.as_ref(), None);
    let b1_child_res = proposal_generator
        .generate_proposal(15, proposer_election.clone())
        .await
        .unwrap();
    assert_eq!(b1_child_res.parent_id(), b1.id());
    assert_eq!(b1_child_res.round(), 15);
    assert_eq!(b1_child_res.quorum_cert().certified_block().id(), b1.id());

    // test that we have authors for the skipped rounds (5,  .. 14), as the limit of 10 has been reached
    assert_eq!(b1_child_res.failed_authors().unwrap().len(), 10);
    assert_eq!(b1_child_res.failed_authors().unwrap().first().unwrap().0, 5);
    assert_eq!(b1_child_res.failed_authors().unwrap().last().unwrap().0, 14);
}

#[tokio::test]
async fn test_old_proposal_generation() {
    let mut inserter = TreeInserter::default();
    let block_store = inserter.block_store();
    let proposal_generator = ProposalGenerator::new(
        inserter.signer().author(),
        block_store.clone(),
        Arc::new(MockPayloadManager::new(None)),
        Arc::new(SimulatedTimeService::new()),
        Duration::ZERO,
        PayloadTxnsSize::new(1, 1000),
        1,
        PayloadTxnsSize::new(1, 500),
        10,
        1,
        Some(MAX_BLOCK_GAS_LIMIT),
        PipelineBackpressureConfig::new_no_backoff(),
        ChainHealthBackoffConfig::new_no_backoff(),
        false,
        ValidatorTxnConfig::default_disabled(),
        true,
        Arc::new(MockOptQSPayloadProvider {}),
    );
    let proposer_election = Arc::new(UnequivocalProposerElection::new(Arc::new(
        RotatingProposer::new(vec![inserter.signer().author()], 1),
    )));
    let genesis = block_store.ordered_root();
    let a1 = inserter
        .insert_block_with_qc(certificate_for_genesis(), &genesis, 1)
        .await;
    inserter.insert_qc_for_block(a1.as_ref(), None);

    let proposal_err = proposal_generator
        .generate_proposal(1, proposer_election.clone())
        .await
        .err();
    assert!(proposal_err.is_some());
}

#[tokio::test]
async fn test_correct_failed_authors() {
    let inserter = TreeInserter::default();
    let author = inserter.signer().author();
    let peer1 = Author::random();
    let peer2 = Author::random();
    let block_store = inserter.block_store();
    let proposal_generator = ProposalGenerator::new(
        author,
        block_store.clone(),
        Arc::new(MockPayloadManager::new(None)),
        Arc::new(SimulatedTimeService::new()),
        Duration::ZERO,
        PayloadTxnsSize::new(1, 1000),
        1,
        PayloadTxnsSize::new(1, 500),
        10,
        1,
        Some(MAX_BLOCK_GAS_LIMIT),
        PipelineBackpressureConfig::new_no_backoff(),
        ChainHealthBackoffConfig::new_no_backoff(),
        false,
        ValidatorTxnConfig::default_disabled(),
        true,
        Arc::new(MockOptQSPayloadProvider {}),
    );
    let proposer_election = Arc::new(UnequivocalProposerElection::new(Arc::new(
        RotatingProposer::new(vec![author, peer1, peer2], 1),
    )));
    let genesis = block_store.ordered_root();

    let result = proposal_generator
        .generate_proposal(6, proposer_election.clone())
        .await
        .unwrap();
    // With no certifications the parent is genesis
    // generate proposals for an empty tree.
    assert_eq!(result.parent_id(), genesis.id());
    // test that we have authors for the skipped rounds (1, 2, .. 5)
    assert_eq!(result.failed_authors().unwrap().len(), 5);
    assert_eq!(result.failed_authors().unwrap()[0], (1, peer1));
    assert_eq!(result.failed_authors().unwrap()[1], (2, peer2));
    assert_eq!(result.failed_authors().unwrap()[2], (3, author));
    assert_eq!(result.failed_authors().unwrap()[3], (4, peer1));
    assert_eq!(result.failed_authors().unwrap()[4], (5, peer2));
}
