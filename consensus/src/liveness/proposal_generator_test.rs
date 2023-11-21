// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::BlockReader,
    liveness::{
        proposal_generator::{
            ChainHealthBackoffConfig, PipelineBackpressureConfig, ProposalGenerator,
        },
        rotating_proposer_election::RotatingProposer,
        unequivocal_proposer_election::UnequivocalProposerElection,
    },
    sys_txn_provider::SysTxnProvider,
    test_utils::{build_empty_tree, MockPayloadManager, TreeInserter},
    util::mock_time_service::SimulatedTimeService,
};
use aptos_consensus_types::{
    block::{block_test_utils::certificate_for_genesis, Block},
    common::Author,
};
use aptos_crypto::hash::CryptoHash;
use aptos_types::{system_txn::SystemTransaction, validator_signer::ValidatorSigner};
use futures::{future::BoxFuture, FutureExt};
use std::{collections::HashSet, sync::Arc, time::Duration};

fn empty_callback() -> BoxFuture<'static, ()> {
    async move {}.boxed()
}

#[tokio::test]
async fn test_proposal_generation_empty_tree() {
    let signer = ValidatorSigner::random(None);
    let block_store = build_empty_tree();
    let mut proposal_generator = ProposalGenerator::new(
        signer.author(),
        block_store.clone(),
        Arc::new(MockPayloadManager::new(None)),
        Arc::new(SimulatedTimeService::new()),
        Duration::ZERO,
        1,
        10,
        10,
        PipelineBackpressureConfig::new_no_backoff(),
        ChainHealthBackoffConfig::new_no_backoff(),
        false,
        vec![],
        false,
    );
    let mut proposer_election =
        UnequivocalProposerElection::new(Arc::new(RotatingProposer::new(vec![signer.author()], 1)));
    let genesis = block_store.ordered_root();

    // Generate proposals for an empty tree.
    let proposal_data = proposal_generator
        .generate_proposal(1, &mut proposer_election, empty_callback())
        .await
        .unwrap();
    let proposal = Block::new_proposal_from_block_data(proposal_data, &signer).unwrap();
    assert_eq!(proposal.parent_id(), genesis.id());
    assert_eq!(proposal.round(), 1);
    assert_eq!(proposal.quorum_cert().certified_block().id(), genesis.id());
    assert_eq!(proposal.block_data().failed_authors().unwrap().len(), 0);

    // Duplicate proposals on the same round are not allowed
    let proposal_err = proposal_generator
        .generate_proposal(1, &mut proposer_election, empty_callback())
        .await
        .err();
    assert!(proposal_err.is_some());
}

#[tokio::test]
async fn test_proposal_generation_parent() {
    let mut inserter = TreeInserter::default();
    let block_store = inserter.block_store();
    let mut proposal_generator = ProposalGenerator::new(
        inserter.signer().author(),
        block_store.clone(),
        Arc::new(MockPayloadManager::new(None)),
        Arc::new(SimulatedTimeService::new()),
        Duration::ZERO,
        1,
        1000,
        10,
        PipelineBackpressureConfig::new_no_backoff(),
        ChainHealthBackoffConfig::new_no_backoff(),
        false,
        vec![],
        false,
    );
    let mut proposer_election = UnequivocalProposerElection::new(Arc::new(RotatingProposer::new(
        vec![inserter.signer().author()],
        1,
    )));
    let genesis = block_store.ordered_root();
    let a1 = inserter
        .insert_block_with_qc(certificate_for_genesis(), &genesis, 1)
        .await;
    let b1 = inserter
        .insert_block_with_qc(certificate_for_genesis(), &genesis, 2)
        .await;

    let original_res = proposal_generator
        .generate_proposal(10, &mut proposer_election, empty_callback())
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
        .generate_proposal(11, &mut proposer_election, empty_callback())
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
        .generate_proposal(15, &mut proposer_election, empty_callback())
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
    let mut proposal_generator = ProposalGenerator::new(
        inserter.signer().author(),
        block_store.clone(),
        Arc::new(MockPayloadManager::new(None)),
        Arc::new(SimulatedTimeService::new()),
        Duration::ZERO,
        1,
        1000,
        10,
        PipelineBackpressureConfig::new_no_backoff(),
        ChainHealthBackoffConfig::new_no_backoff(),
        false,
        vec![],
        false,
    );
    let mut proposer_election = UnequivocalProposerElection::new(Arc::new(RotatingProposer::new(
        vec![inserter.signer().author()],
        1,
    )));
    let genesis = block_store.ordered_root();
    let a1 = inserter
        .insert_block_with_qc(certificate_for_genesis(), &genesis, 1)
        .await;
    inserter.insert_qc_for_block(a1.as_ref(), None);

    let proposal_err = proposal_generator
        .generate_proposal(1, &mut proposer_election, empty_callback())
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
    let mut proposal_generator = ProposalGenerator::new(
        author,
        block_store.clone(),
        Arc::new(MockPayloadManager::new(None)),
        Arc::new(SimulatedTimeService::new()),
        Duration::ZERO,
        1,
        1000,
        10,
        PipelineBackpressureConfig::new_no_backoff(),
        ChainHealthBackoffConfig::new_no_backoff(),
        false,
        vec![],
        false,
    );
    let mut proposer_election = UnequivocalProposerElection::new(Arc::new(RotatingProposer::new(
        vec![author, peer1, peer2],
        1,
    )));
    let genesis = block_store.ordered_root();

    let result = proposal_generator
        .generate_proposal(6, &mut proposer_election, empty_callback())
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

#[test]
fn get_sys_txns_basic() {
    let sys_txn_providers = vec![
        new_dummy_txn_provider(Some(0)),
        new_dummy_txn_provider(None),
        new_dummy_txn_provider(Some(2)),
    ];
    let mut max_block_txns = 99;
    let mut max_block_bytes = 2048;
    let pending_sys_txns = HashSet::new();
    let sys_txns = ProposalGenerator::propose_sys_txns(
        true,
        sys_txn_providers.as_slice(),
        &mut max_block_txns,
        &mut max_block_bytes,
        &pending_sys_txns,
    );
    // Proposing 2 txns, 32 bytes in total.
    assert_eq!(
        vec![SystemTransaction::dummy(0), SystemTransaction::dummy(2)],
        sys_txns
    );
    assert_eq!(97, max_block_txns);
    assert_eq!(2016, max_block_bytes);
}

#[test]
fn get_sys_txns_should_respect_feature_flag() {
    let sys_txn_providers = vec![
        new_dummy_txn_provider(Some(0)),
        new_dummy_txn_provider(None),
        new_dummy_txn_provider(Some(2)),
    ];
    let mut max_block_txns = 99;
    let mut max_block_bytes = 2048;
    let pending_sys_txns = HashSet::new();
    let sys_txns = ProposalGenerator::propose_sys_txns(
        false,
        sys_txn_providers.as_slice(),
        &mut max_block_txns,
        &mut max_block_bytes,
        &pending_sys_txns,
    );
    // Proposing 0 txns.
    assert_eq!(0, sys_txns.len());
    assert_eq!(99, max_block_txns);
    assert_eq!(2048, max_block_bytes);
}

#[test]
fn get_sys_txns_should_respect_txn_count_limit() {
    let sys_txn_providers = vec![
        new_dummy_txn_provider(Some(0)),
        new_dummy_txn_provider(None),
        new_dummy_txn_provider(Some(2)),
    ];
    let mut max_block_txns = 1;
    let mut max_block_bytes = 2048;
    let pending_sys_txns = HashSet::new();
    let sys_txns = ProposalGenerator::propose_sys_txns(
        true,
        sys_txn_providers.as_slice(),
        &mut max_block_txns,
        &mut max_block_bytes,
        &pending_sys_txns,
    );
    // Proposing 1 txn, 16 bytes in total.
    assert_eq!(vec![SystemTransaction::dummy(0)], sys_txns);
    assert_eq!(0, max_block_txns);
    assert_eq!(2032, max_block_bytes);
}

#[test]
fn get_sys_txns_should_respect_block_size_limit() {
    let sys_txn_providers = vec![
        new_dummy_txn_provider(Some(0)),
        new_dummy_txn_provider(None),
        new_dummy_txn_provider(Some(2)),
    ];
    let mut max_block_txns = 99;
    let mut max_block_bytes = 20;
    let pending_sys_txns = HashSet::new();
    let sys_txns = ProposalGenerator::propose_sys_txns(
        true,
        sys_txn_providers.as_slice(),
        &mut max_block_txns,
        &mut max_block_bytes,
        &pending_sys_txns,
    );
    // Proposing 1 txn, 16 bytes in total.
    assert_eq!(vec![SystemTransaction::dummy(0)], sys_txns);
    assert_eq!(98, max_block_txns);
    assert_eq!(4, max_block_bytes);
}

#[test]
fn get_sys_txns_should_respect_pending_list() {
    let sys_txn_providers = vec![
        new_dummy_txn_provider(Some(0)),
        new_dummy_txn_provider(None),
        new_dummy_txn_provider(Some(2)),
    ];
    let mut max_block_txns = 99;
    let mut max_block_bytes = 2048;
    let pending_sys_txns = HashSet::from([SystemTransaction::dummy(0).hash()]);
    let sys_txns = ProposalGenerator::propose_sys_txns(
        true,
        sys_txn_providers.as_slice(),
        &mut max_block_txns,
        &mut max_block_bytes,
        &pending_sys_txns,
    );
    // Proposing 1 txn, 16 bytes in total.
    assert_eq!(vec![SystemTransaction::dummy(2)], sys_txns);
    assert_eq!(98, max_block_txns);
    assert_eq!(2032, max_block_bytes);
}

#[cfg(test)]
struct DummySysTxnProvider {
    txn: Option<Arc<SystemTransaction>>,
}

#[cfg(test)]
impl SysTxnProvider for DummySysTxnProvider {
    fn get(&self) -> Option<Arc<SystemTransaction>> {
        self.txn.clone()
    }
}

#[cfg(test)]
fn new_dummy_txn_provider(nonce: Option<u64>) -> Arc<dyn SysTxnProvider> {
    Arc::new(DummySysTxnProvider {
        txn: nonce.map(|x| Arc::new(SystemTransaction::dummy(x))),
    })
}
