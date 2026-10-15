// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::*;
use crate::pipeline::tests::{
    commit_pipeline_tests::{attach_pipeline, network},
    test_utils::prepare_executed_blocks_with_ledger_info,
};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_consensus_types::block::block_test_utils::certificate_for_genesis;
use aptos_types::{
    ledger_info::generate_ledger_info_with_sig, validator_signer::ValidatorSigner,
    validator_verifier::random_validator_verifier,
};
use futures::channel::mpsc::UnboundedSender;

fn fixture() -> (
    CommitPipeline,
    UnboundedSender<OrderedBlocks>,
    Vec<ValidatorSigner>,
) {
    let (signers, verifier) = random_validator_verifier(1, None, false);
    let verifier = Arc::new(verifier);
    let (block_tx, block_rx) = unbounded();
    let (_reset_tx, reset_rx) = unbounded();
    let (_message_tx, message_rx) = aptos_channel::new(QueueStyle::FIFO, 100, None);
    let service = CommitPipeline::new(
        signers[0].author(),
        network(&signers[0], verifier.clone()),
        message_rx,
        block_rx,
        reset_rx,
        Arc::new(EpochState { epoch: 1, verifier }),
        BoundedExecutor::new(1, tokio::runtime::Handle::current()),
        false,
        0,
        ConsensusObserverConfig::default(),
        None,
        100,
    );
    (service, block_tx, signers)
}

fn ordered(block: &Arc<PipelinedBlock>, signers: &[ValidatorSigner]) -> OrderedBlocks {
    OrderedBlocks {
        ordered_blocks: vec![block.clone()],
        ordered_proof: generate_ledger_info_with_sig(
            signers,
            LedgerInfo::new(block.block_info(), HashValue::zero()),
        ),
    }
}

fn blocks(signers: &[ValidatorSigner], count: Round) -> Vec<Arc<PipelinedBlock>> {
    prepare_executed_blocks_with_ledger_info(
        &signers[0],
        count,
        HashValue::zero(),
        HashValue::zero(),
        None,
        Some(certificate_for_genesis()),
        1,
    )
    .0
}

fn visit_child_first(service: &mut CommitPipeline, child: HashValue, certified: Option<HashValue>) {
    // Select an allowed map layout that visits the child before its parent.
    // A reset must work for every HashMap seed, not just parent-first layouts.
    for _ in 0..128 {
        if service.entries.keys().find(|id| Some(**id) != certified) == Some(&child) {
            return;
        }
        let entries = std::mem::take(&mut service.entries);
        service.entries.extend(entries);
    }
    panic!("Could not select a child-first map layout");
}

#[tokio::test]
async fn reset_drains_child_first_without_aborting_storage_tasks() {
    let (mut service, _block_tx, signers) = fixture();
    let blocks = blocks(&signers, 2);
    let parent = attach_pipeline(&blocks[0], None);
    let child = attach_pipeline(&blocks[1], Some(parent.clone()));
    for block in &blocks {
        service.register(ordered(block, &signers));
    }
    visit_child_first(&mut service, blocks[1].id(), None);

    tokio::time::timeout(Duration::from_secs(5), service.drain())
        .await
        .expect("Child storage must not prevent cancellation of its parent");
    assert!(parent.await.is_err());
    assert!(child.await.is_err());
    assert!(service.entries.is_empty());
}

#[tokio::test]
async fn reset_drains_registered_and_queued_storage_dependencies() {
    let (mut service, block_tx, signers) = fixture();
    let blocks = blocks(&signers, 3);
    let mut parent = None;
    let mut commits = vec![];
    for (index, block) in blocks.iter().enumerate() {
        let commit = attach_pipeline(block, parent);
        parent = Some(commit.clone());
        commits.push(commit);
        let ordered = ordered(block, &signers);
        if index == 0 {
            service.register(ordered);
        } else {
            block_tx.unbounded_send(ordered).unwrap();
        }
    }

    tokio::time::timeout(Duration::from_secs(5), service.drain())
        .await
        .expect("Queued blocks must be cancelled and drained too");
    for (block, commit) in blocks.iter().zip(commits) {
        assert!(commit.await.is_err());
        assert!(block.pipeline_futs().is_none());
    }
}

#[tokio::test]
async fn epoch_ending_commit_drains_child_first_suffix_and_preserves_certified_prefix() {
    let (mut service, _block_tx, signers) = fixture();
    let blocks = blocks(&signers, 3);
    let mut parent = None;
    let mut commits = vec![];
    for block in &blocks {
        let commit = attach_pipeline(block, parent);
        parent = Some(commit.clone());
        commits.push(commit);
        service.register(ordered(block, &signers));
    }
    let result = blocks[0].compute_result();
    let info = blocks[0].block().gen_block_info(
        result.root_hash(),
        result.last_version_or_0(),
        Some(EpochState {
            epoch: 2,
            verifier: service.epoch_state.verifier.clone(),
        }),
    );
    let proof = generate_ledger_info_with_sig(&signers, LedgerInfo::new(info, HashValue::zero()));
    assert!(proof.ledger_info().ends_epoch());
    let certificate = &mut service
        .entries
        .get_mut(&blocks[0].id())
        .unwrap()
        .certificate;
    certificate.set_execution_result(proof.ledger_info().clone());
    certificate.add_decision(proof.clone());
    visit_child_first(&mut service, blocks[2].id(), Some(blocks[0].id()));

    tokio::time::timeout(Duration::from_secs(5), service.try_commit(blocks[0].id()))
        .await
        .expect("Epoch shutdown must drain the dependent suffix");
    assert_eq!(commits.remove(0).await.unwrap(), Some(proof));
    assert!(blocks[0].pipeline_futs().is_some());
    for (block, commit) in blocks[1..].iter().zip(commits) {
        assert!(commit.await.is_err());
        assert!(block.pipeline_futs().is_none());
    }
    assert_eq!(service.highest_committed_round, blocks[0].round());
    assert!(service.entries.is_empty());
    assert!(service.pending_commit_blocks.is_empty());
}
