// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    block_storage::{block_store::sync_manager::NeedFetchResult, BlockReader},
    pending_votes::{PendingVotes, VoteReceptionResult},
    test_utils::{
        build_default_empty_tree, build_simple_tree, consensus_runtime, timed_block_on,
        TreeInserter,
    },
};
use aptos_consensus_types::{
    block::{
        block_test_utils::{
            self, certificate_for_genesis, gen_test_certificate, placeholder_certificate_for_block,
            placeholder_ledger_info,
        },
        Block,
    },
    common::{Author, Payload},
    vote::Vote,
    vote_data::VoteData,
};
use aptos_crypto::{HashValue, PrivateKey};
use aptos_types::{
    validator_signer::ValidatorSigner, validator_verifier::random_validator_verifier,
};
use proptest::prelude::*;
use std::{cmp::min, collections::HashSet};

#[tokio::test]
async fn test_highest_block_and_quorum_cert() {
    let mut inserter = TreeInserter::default();
    let block_store = inserter.block_store();
    assert_eq!(
        block_store.highest_certified_block().block(),
        &Block::make_genesis_block()
    );
    assert_eq!(
        block_store.highest_quorum_cert().as_ref(),
        &certificate_for_genesis()
    );

    let genesis = block_store.ordered_root();

    // Genesis block and quorum certificate is still the highest
    let block_round_1 = inserter
        .insert_block_with_qc(certificate_for_genesis(), &genesis, 1)
        .await;
    assert_eq!(
        block_store.highest_certified_block().block(),
        &Block::make_genesis_block()
    );
    assert_eq!(
        block_store.highest_quorum_cert().as_ref(),
        &certificate_for_genesis()
    );

    // block_round_1 block and quorum certificate is now the highest
    let block_round_3 = inserter.insert_block(&block_round_1, 3, None).await;
    assert_eq!(block_store.highest_certified_block(), block_round_1);
    assert_eq!(
        block_store.highest_quorum_cert().as_ref(),
        block_store
            .get_block(block_round_3.id())
            .expect("block_round_1 should exist")
            .quorum_cert()
    );

    // block_round_1 block and quorum certificate is still the highest, since block_round_4
    // also builds on block_round_1
    let block_round_4 = inserter.insert_block(&block_round_1, 4, None).await;
    assert_eq!(block_store.highest_certified_block(), block_round_1);
    assert_eq!(
        block_store.highest_quorum_cert().as_ref(),
        block_store
            .get_block(block_round_4.id())
            .expect("block_round_1 should exist")
            .quorum_cert()
    );
}

#[tokio::test]
async fn test_qc_ancestry() {
    let mut inserter = TreeInserter::default();
    let block_store = inserter.block_store();
    let genesis = block_store.ordered_root();
    let block_a_1 = inserter
        .insert_block_with_qc(certificate_for_genesis(), &genesis, 1)
        .await;
    let block_a_2 = inserter.insert_block(&block_a_1, 2, None).await;

    assert_eq!(
        block_store.get_block(genesis.quorum_cert().certified_block().id()),
        None
    );
    assert_eq!(
        block_store.get_block(block_a_1.quorum_cert().certified_block().id()),
        Some(genesis)
    );
    assert_eq!(
        block_store.get_block(block_a_2.quorum_cert().certified_block().id()),
        Some(block_a_1)
    );
}

// This test should be continuously extended to eventually become the
// single-page spec for the logic of our block storage.
proptest! {

    #[test]
    fn test_block_store_insert(
        (private_keys, blocks) in block_test_utils::block_forest_and_its_keys(
            // quorum size
            10,
            // recursion depth
            50)
    ){
        let authors: HashSet<Author> = private_keys.iter().map(
            // match the signer_strategy in validator_signer.rs
            |key| Author::from_bytes(&key.public_key().to_bytes()[0..32]).unwrap()
        ).collect();
        let runtime = consensus_runtime();
        let block_store = build_default_empty_tree();
        for block in blocks {
            if block.round() > 0 && authors.contains(&block.author().unwrap()) {
                let known_parent = block_store.block_exists(block.parent_id());
                let certified_parent = block.quorum_cert().certified_block().id() == block.parent_id();
                let verify_res = block.verify_well_formed();
                let res = timed_block_on(&runtime, block_store.insert_block(block.clone()));
                if !certified_parent {
                    prop_assert!(verify_res.is_err());
                } else if !known_parent {
                    // We cannot really bring blocks in this test because the block retrieval
                    // functionality invokes event processing, which is not setup here.
                    assert!(res.is_err());
                }
                else {
                    // The parent must be present if we get to this line.
                    let parent = block_store.get_block(block.parent_id()).unwrap();
                    if block.round() <= parent.round() {
                        prop_assert!(res.is_err());
                    } else {
                        let executed_block = res.unwrap();
                        prop_assert_eq!(executed_block.block(),
                             &block,
                            "expected ok on block: {:#?}, got {:#?}", block, executed_block.block());
                    }
                }
            }
        }
    }
}

#[tokio::test]
async fn test_block_store_prune() {
    //       ╭--> A1--> A2--> A3
    // Genesis--> B1--> B2
    //             ╰--> C1
    let (blocks, block_store) = build_simple_tree().await;
    // Attempt to prune genesis block (should be no-op)
    assert_eq!(block_store.prune_tree(blocks[0].id()).len(), 0);
    assert_eq!(block_store.len(), 7);
    assert_eq!(block_store.child_links(), block_store.len() - 1);
    assert_eq!(block_store.pruned_blocks_in_mem(), 0);

    let (blocks, block_store) = build_simple_tree().await;
    // Prune up to block A1
    assert_eq!(block_store.prune_tree(blocks[1].id()).len(), 4);
    assert_eq!(block_store.len(), 3);
    assert_eq!(block_store.child_links(), block_store.len() - 1);
    assert_eq!(block_store.pruned_blocks_in_mem(), 4);

    let (blocks, block_store) = build_simple_tree().await;
    // Prune up to block A2
    assert_eq!(block_store.prune_tree(blocks[2].id()).len(), 5);
    assert_eq!(block_store.len(), 2);
    assert_eq!(block_store.child_links(), block_store.len() - 1);
    assert_eq!(block_store.pruned_blocks_in_mem(), 5);

    let (blocks, block_store) = build_simple_tree().await;
    // Prune up to block A3
    assert_eq!(block_store.prune_tree(blocks[3].id()).len(), 6);
    assert_eq!(block_store.len(), 1);
    assert_eq!(block_store.child_links(), block_store.len() - 1);

    let (blocks, block_store) = build_simple_tree().await;
    // Prune up to block B1
    assert_eq!(block_store.prune_tree(blocks[4].id()).len(), 4);
    assert_eq!(block_store.len(), 3);
    assert_eq!(block_store.child_links(), block_store.len() - 1);

    let (blocks, block_store) = build_simple_tree().await;
    // Prune up to block B2
    assert_eq!(block_store.prune_tree(blocks[5].id()).len(), 6);
    assert_eq!(block_store.len(), 1);
    assert_eq!(block_store.child_links(), block_store.len() - 1);

    let (blocks, block_store) = build_simple_tree().await;
    // Prune up to block C1
    assert_eq!(block_store.prune_tree(blocks[6].id()).len(), 6);
    assert_eq!(block_store.len(), 1);
    assert_eq!(block_store.child_links(), block_store.len() - 1);

    // Prune the chain of Genesis -> B1 -> B2
    let (blocks, block_store) = build_simple_tree().await;
    // Prune up to block B1
    assert_eq!(block_store.prune_tree(blocks[4].id()).len(), 4);
    assert_eq!(block_store.len(), 3);
    assert_eq!(block_store.child_links(), block_store.len() - 1);
    // Prune up to block B2
    assert_eq!(block_store.prune_tree(blocks[5].id()).len(), 2);
    assert_eq!(block_store.len(), 1);
    assert_eq!(block_store.child_links(), block_store.len() - 1);
}

#[tokio::test]
async fn test_block_tree_gc() {
    // build a tree with 100 nodes, max_pruned_nodes_in_mem = 10
    let mut inserter = TreeInserter::default();
    let block_store = inserter.block_store();
    let genesis = block_store.ordered_root();
    let mut cur_node = block_store.get_block(genesis.id()).unwrap();
    let mut added_blocks = vec![];

    for round in 1..100 {
        if round == 1 {
            cur_node = inserter
                .insert_block_with_qc(certificate_for_genesis(), &cur_node, round)
                .await;
        } else {
            cur_node = inserter.insert_block(&cur_node, round, None).await;
        }
        added_blocks.push(cur_node.clone());
    }

    for (i, block) in added_blocks.iter().enumerate() {
        assert_eq!(block_store.len(), 100 - i);
        assert_eq!(block_store.pruned_blocks_in_mem(), min(i, 10));
        block_store.prune_tree(block.id());
    }
}

#[tokio::test]
async fn test_path_from_root() {
    let mut inserter = TreeInserter::default();
    let block_store = inserter.block_store();
    let genesis = block_store
        .get_block(block_store.ordered_root().id())
        .unwrap();
    let b1 = inserter
        .insert_block_with_qc(certificate_for_genesis(), &genesis, 1)
        .await;
    let b2 = inserter.insert_block(&b1, 2, None).await;
    let b3 = inserter.insert_block(&b2, 3, None).await;

    assert_eq!(
        block_store.path_from_ordered_root(b3.id()),
        Some(vec![b1, b2.clone(), b3.clone()])
    );
    assert_eq!(
        block_store.path_from_ordered_root(genesis.id()),
        Some(vec![])
    );

    block_store.prune_tree(b2.id());

    assert_eq!(
        block_store.path_from_ordered_root(b3.id()),
        Some(vec![b3.clone()])
    );
    assert_eq!(block_store.path_from_ordered_root(genesis.id()), None);
}

#[tokio::test]
async fn test_insert_vote() {
    ::aptos_logger::Logger::init_for_testing();
    // Set up enough different authors to support different votes for the same block.
    let (signers, validator_verifier) = random_validator_verifier(11, Some(10), false);
    let my_signer = signers[10].clone();
    let mut inserter = TreeInserter::new(my_signer);
    let block_store = inserter.block_store();
    let genesis = block_store.ordered_root();
    let block = inserter
        .insert_block_with_qc(certificate_for_genesis(), &genesis, 1)
        .await;

    let mut pending_votes = PendingVotes::new();

    assert!(block_store.get_quorum_cert_for_block(block.id()).is_none());
    for (i, voter) in signers.iter().enumerate().take(10).skip(1) {
        let vote = Vote::new(
            VoteData::new(
                block.block().gen_block_info(
                    block.compute_result().root_hash(),
                    block.compute_result().last_version_or_0(),
                    block.compute_result().epoch_state().clone(),
                ),
                block.quorum_cert().certified_block().clone(),
            ),
            voter.author(),
            placeholder_ledger_info(),
            voter,
        )
        .unwrap();
        let vote_res = pending_votes.insert_vote(&vote, &validator_verifier);

        // first vote of an author is accepted
        assert_eq!(vote_res, VoteReceptionResult::VoteAdded(i as u128));
        // filter out duplicates
        assert_eq!(
            pending_votes.insert_vote(&vote, &validator_verifier),
            VoteReceptionResult::DuplicateVote,
        );
        // qc is still not there
        assert!(block_store.get_quorum_cert_for_block(block.id()).is_none());
    }

    // Add the final vote to form a QC
    let final_voter = &signers[0];
    let vote = Vote::new(
        VoteData::new(
            block.block().gen_block_info(
                block.compute_result().root_hash(),
                block.compute_result().last_version_or_0(),
                block.compute_result().epoch_state().clone(),
            ),
            block.quorum_cert().certified_block().clone(),
        ),
        final_voter.author(),
        placeholder_ledger_info(),
        final_voter,
    )
    .unwrap();
    match pending_votes.insert_vote(&vote, &validator_verifier) {
        VoteReceptionResult::NewQuorumCertificate(qc) => {
            assert_eq!(qc.certified_block().id(), block.id());
            block_store
                .insert_single_quorum_cert(qc.as_ref().clone())
                .unwrap();
        },
        _ => {
            panic!("QC not formed!");
        },
    }

    let block_qc = block_store.get_quorum_cert_for_block(block.id()).unwrap();
    assert_eq!(block_qc.certified_block().id(), block.id());
}

#[tokio::test]
async fn test_illegal_timestamp() {
    let signer = ValidatorSigner::random(None);
    let block_store = build_default_empty_tree();
    let genesis = block_store.ordered_root();
    let block_with_illegal_timestamp = Block::new_proposal(
        Payload::empty(false),
        0,
        // This timestamp is illegal, it is the same as genesis
        genesis.timestamp_usecs(),
        certificate_for_genesis(),
        &signer,
        Vec::new(),
    )
    .unwrap();
    let result = block_store.insert_block(block_with_illegal_timestamp).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_highest_qc() {
    let mut inserter = TreeInserter::default();
    let block_store = inserter.block_store();

    // build a tree of the following form
    // genesis <- a1 <- a2 <- a3
    let genesis = block_store.ordered_root();
    let a1 = inserter
        .insert_block_with_qc(certificate_for_genesis(), &genesis, 1)
        .await;
    assert_eq!(block_store.highest_certified_block(), genesis);
    let a2 = inserter.insert_block(&a1, 2, None).await;
    assert_eq!(block_store.highest_certified_block(), a1);
    let _a3 = inserter.insert_block(&a2, 3, None).await;
    assert_eq!(block_store.highest_certified_block(), a2);
}

#[tokio::test]
async fn test_need_fetch_for_qc() {
    let mut inserter = TreeInserter::default();
    let block_store = inserter.block_store();

    // build a tree of the following form
    // genesis <- a1 <- a2 <- a3
    let genesis = block_store.ordered_root();
    let a1 = inserter
        .insert_block_with_qc(certificate_for_genesis(), &genesis, 1)
        .await;
    let a2 = inserter.insert_block(&a1, 2, None).await;
    let a3 = inserter.insert_block(&a2, 3, None).await;
    block_store.prune_tree(a2.id());
    let need_fetch_qc = placeholder_certificate_for_block(
        &[inserter.signer().clone()],
        HashValue::zero(),
        a3.round() + 1,
        HashValue::zero(),
        a3.round(),
    );
    let too_old_qc = certificate_for_genesis();
    let can_insert_qc = placeholder_certificate_for_block(
        &[inserter.signer().clone()],
        a3.id(),
        a3.round(),
        a2.id(),
        a2.round(),
    );
    let duplicate_qc = block_store.get_quorum_cert_for_block(a2.id()).unwrap();
    assert_eq!(
        block_store.need_fetch_for_quorum_cert(&need_fetch_qc),
        NeedFetchResult::NeedFetch
    );
    assert_eq!(
        block_store.need_fetch_for_quorum_cert(&too_old_qc),
        NeedFetchResult::QCRoundBeforeRoot,
    );
    assert_eq!(
        block_store.need_fetch_for_quorum_cert(&can_insert_qc),
        NeedFetchResult::QCBlockExist,
    );
    assert_eq!(
        block_store.need_fetch_for_quorum_cert(duplicate_qc.as_ref()),
        NeedFetchResult::QCAlreadyExist,
    );
}

#[tokio::test]
async fn test_need_sync_for_ledger_info() {
    let mut inserter = TreeInserter::default();
    let block_store = inserter.block_store();

    let mut prev = block_store.ordered_root();
    for i in 1..=30 {
        prev = inserter.insert_block(&prev, i, None).await;
    }
    inserter
        .insert_block(
            &prev,
            31,
            Some(prev.block().gen_block_info(HashValue::zero(), 1, None)),
        )
        .await;
    assert_eq!(block_store.ordered_root().round(), 30);
    assert_eq!(block_store.commit_root().round(), 0);

    let create_ledger_info = |round: u64| {
        let future_block = inserter.create_block_with_qc(
            certificate_for_genesis(),
            1,
            round,
            Payload::empty(false),
            vec![],
        );
        gen_test_certificate(
            &[inserter.signer().clone()],
            future_block.gen_block_info(HashValue::zero(), 0, None),
            future_block.quorum_cert().parent_block().clone(),
            Some(future_block.gen_block_info(HashValue::zero(), 0, None)),
        )
        .ledger_info()
        .clone()
    };
    // it's larger and the block doesn't exist in the tree
    let ordered_round_too_far = block_store.ordered_root().round() + 1;
    let ordered_too_far = create_ledger_info(ordered_round_too_far);
    assert!(block_store
        .need_sync_for_ledger_info(&ordered_too_far)
        .is_some());

    let committed_round_too_far =
        block_store.commit_root().round() + 60.max(block_store.vote_back_pressure_limit * 2) + 1;
    let committed_too_far = create_ledger_info(committed_round_too_far);
    assert!(block_store
        .need_sync_for_ledger_info(&committed_too_far)
        .is_some());

    let round_not_too_far =
        block_store.commit_root().round() + block_store.vote_back_pressure_limit + 1;
    let not_too_far = create_ledger_info(round_not_too_far);
    assert!(block_store
        .need_sync_for_ledger_info(&not_too_far)
        .is_none());
}

#[tokio::test]
async fn test_add_certs_retains_2chain_tc_quorum_cert() {
    use crate::{
        block_storage::block_store::sync_manager::BlockRetriever,
        network::NetworkSender,
        network_interface::{ConsensusNetworkClient, DIRECT_SEND, RPC},
        persistent_liveness_storage::{LedgerRecoveryData, RecoveryData, RootMetadata},
    };
    use aptos_channels::{aptos_channel, message_queues::QueueStyle};
    use aptos_config::network_id::NetworkId;
    use aptos_consensus_types::{quorum_cert::QuorumCert, sync_info::SyncInfo};
    use aptos_crypto::hash::{CryptoHash, ACCUMULATOR_PLACEHOLDER_HASH};
    use aptos_network::{
        application::{interface::NetworkClient, storage::PeersAndMetadata},
        peer_manager::{ConnectionRequestSender, PeerManagerRequestSender},
        protocols::network::NewNetworkSender,
    };
    use aptos_safety_rules::{test_utils, PersistentSafetyStorage, SafetyRules, TSafetyRules};
    use aptos_secure_storage::{InMemoryStorage, Storage};
    use aptos_types::{
        block_info::BlockInfo,
        ledger_info::{generate_ledger_info_with_sig, LedgerInfo},
        waypoint::Waypoint,
    };
    use maplit::hashmap;
    use std::sync::Arc;

    let (_, block_store) = build_simple_tree().await;
    let genesis_block = block_store.ordered_root().block().clone();
    let genesis_qc = block_store
        .get_quorum_cert_for_block(genesis_block.id())
        .unwrap()
        .as_ref()
        .clone();

    let signer = ValidatorSigner::from_int(0);
    let (proof, _) = test_utils::make_genesis(&signer);
    let waypoint =
        Waypoint::new_epoch_boundary(proof.ledger_info_with_sigs.first().unwrap().ledger_info())
            .unwrap();
    let safety_storage = PersistentSafetyStorage::initialize(
        Storage::from(InMemoryStorage::new()),
        signer.author(),
        signer.private_key().clone(),
        waypoint,
        true,
    );
    let mut safety_rules = SafetyRules::new(safety_storage, false);
    safety_rules.initialize(&proof).unwrap();

    let make_qc = |certified: &Block, parent: &Block, commit: Option<&Block>| -> QuorumCert {
        let vote_data = VoteData::new(
            certified.gen_block_info(*ACCUMULATOR_PLACEHOLDER_HASH, 0, None),
            parent.gen_block_info(*ACCUMULATOR_PLACEHOLDER_HASH, 0, None),
        );
        let commit_info = commit.map_or(BlockInfo::empty(), |b| {
            b.gen_block_info(*ACCUMULATOR_PLACEHOLDER_HASH, 0, None)
        });
        let ledger_info = LedgerInfo::new(commit_info, vote_data.hash());
        QuorumCert::new(
            vote_data,
            generate_ledger_info_with_sig(&[signer.clone()], ledger_info),
        )
    };

    // Committed branch B12 <- B13, then an uncommitted jump-round chain B15 <- B17 <- B19 <- B20
    // on top of it. (Issue #20801 roots the jump chain at genesis; that fork conflicts with the
    // committed B12 and could not be re-inserted after rebuilding at B12.)
    let b12 = test_utils::make_proposal_with_qc(12, genesis_qc.clone(), &signer);
    let qc12 = make_qc(b12.block(), &genesis_block, None);
    let b13 = test_utils::make_proposal_with_qc(13, qc12.clone(), &signer);
    let qc13 = make_qc(b13.block(), b12.block(), Some(b12.block()));
    let b15 = test_utils::make_proposal_with_qc(15, qc13.clone(), &signer);
    let qc15 = make_qc(b15.block(), b13.block(), None);
    let b17 = test_utils::make_proposal_with_qc(17, qc15.clone(), &signer);
    let qc17 = make_qc(b17.block(), b15.block(), None);
    let b19 = test_utils::make_proposal_with_qc(19, qc17.clone(), &signer);
    let qc19 = make_qc(b19.block(), b17.block(), None);
    let b20 = test_utils::make_proposal_with_qc(20, qc19.clone(), &signer);

    // Voting on B20 raises SafetyRules' preferred_round to 17 and one_chain_round to 19
    safety_rules
        .construct_and_sign_vote_two_chain(&b20, None)
        .unwrap();
    assert_eq!(
        safety_rules.consensus_state().unwrap().preferred_round(),
        17
    );
    assert_eq!(
        safety_rules.consensus_state().unwrap().one_chain_round(),
        19
    );

    // Rebuild at B12, dropping QC15/QC17/QC19 so the BlockStore HQC falls below round 17
    let (root, root_metadata, blocks, quorum_certs) = RecoveryData::new(
        None,
        LedgerRecoveryData::new(qc13.ledger_info().clone()),
        vec![
            b17.block().clone(),
            b15.block().clone(),
            b13.block().clone(),
            b12.block().clone(),
            genesis_block.clone(),
        ],
        RootMetadata::new_empty(),
        vec![qc13.clone(), qc12, genesis_qc],
        None,
        false,
        None,
    )
    .unwrap()
    .take();
    block_store
        .rebuild(root, root_metadata, blocks, quorum_certs)
        .await;
    let hqc_round_after_rebuild = block_store.highest_quorum_cert().certified_block().round();
    assert!(hqc_round_after_rebuild < 17);

    let (network_reqs_tx, _network_reqs_rx) = aptos_channel::new(QueueStyle::FIFO, 8, None);
    let (connection_reqs_tx, _connection_reqs_rx) = aptos_channel::new(QueueStyle::FIFO, 8, None);
    let network_client = NetworkClient::new(
        DIRECT_SEND.into(),
        RPC.into(),
        hashmap! {NetworkId::Validator => aptos_network::protocols::network::NetworkSender::new(
            PeerManagerRequestSender::new(network_reqs_tx),
            ConnectionRequestSender::new(connection_reqs_tx),
        )},
        PeersAndMetadata::new(&[NetworkId::Validator]),
    );
    let (self_loop_tx, _self_loop_rx) = aptos_channels::new_unbounded_test();
    let network = Arc::new(NetworkSender::new(
        signer.author(),
        ConsensusNetworkClient::new(network_client),
        self_loop_tx,
        Arc::new(random_validator_verifier(1, None, false).1),
    ));
    let retriever = || {
        BlockRetriever::new(
            network.clone(),
            signer.author(),
            vec![signer.author()],
            10,
            block_store.pending_blocks(),
        )
    };

    // Control: a TC whose hqc round does not exceed the SyncInfo HQC round leaves the HQC alone
    let tc14 = test_utils::make_timeout_cert(14, &qc13, &signer);
    let sync_info = SyncInfo::new(
        qc13.clone(),
        qc13.clone().into_wrapped_ledger_info(),
        Some(tc14),
    );
    block_store
        .add_certs(&sync_info, retriever())
        .await
        .unwrap();
    assert_eq!(
        block_store.highest_quorum_cert().certified_block().round(),
        hqc_round_after_rebuild
    );
    assert_eq!(
        block_store.highest_2chain_timeout_cert().unwrap().round(),
        14
    );

    // SyncInfo(HQC=QC13, TC20(hqc=17)): the TC's QC17 must be retained
    let tc20 = test_utils::make_timeout_cert(20, &qc17, &signer);
    let sync_info = SyncInfo::new(
        qc13.clone(),
        qc13.clone().into_wrapped_ledger_info(),
        Some(tc20),
    );
    block_store
        .add_certs(&sync_info, retriever())
        .await
        .unwrap();
    assert_eq!(
        block_store.highest_quorum_cert().certified_block().round(),
        17
    );

    // The round 21 leader can now sign a proposal on top of the retained HQC
    let hqc = block_store.highest_quorum_cert().as_ref().clone();
    let b21 = test_utils::make_proposal_with_qc(21, hqc, &signer);
    safety_rules
        .sign_proposal(b21.block().block_data())
        .unwrap();
}
