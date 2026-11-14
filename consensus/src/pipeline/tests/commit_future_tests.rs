// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{
    commit_future_test_utils::{attach_pipeline, network},
    test_utils::prepare_executed_blocks_with_ledger_info,
};
use crate::{
    block_storage::block_tree::BlockTree,
    pipeline::{
        commit_pipeline_futures::CommitPipelineRuntime, commit_reliable_broadcast::CommitMessage,
        pipeline_builder::spawn_shared_fut,
    },
};
use aptos_bounded_executor::BoundedExecutor;
use aptos_consensus_types::{
    block::{block_test_utils::certificate_for_genesis, Block},
    pipeline::{commit_decision::CommitDecision, commit_vote::CommitVote},
    pipelined_block::{CommitInput, PipelinedBlock, TaskFuture},
};
use aptos_crypto::HashValue;
use aptos_executor_types::state_compute_result::StateComputeResult;
use aptos_infallible::RwLock;
use aptos_types::{
    epoch_state::EpochState,
    ledger_info::{generate_ledger_info_with_sig, LedgerInfo},
    validator_signer::ValidatorSigner,
    validator_verifier::random_validator_verifier,
};
use futures::FutureExt;
use std::{sync::Arc, time::Duration};

fn ready<T: Send + Clone + 'static>(value: T) -> TaskFuture<T> {
    async move { Ok(value) }.boxed().shared()
}

fn setup() -> (
    Arc<CommitPipelineRuntime>,
    Arc<RwLock<BlockTree>>,
    Vec<ValidatorSigner>,
) {
    let (signers, verifier) = random_validator_verifier(4, None, false);
    let verifier = Arc::new(verifier);
    let runtime = CommitPipelineRuntime::new(
        Arc::new(EpochState {
            epoch: 1,
            verifier: verifier.clone(),
        }),
        network(&signers[0], verifier),
        BoundedExecutor::new(1, tokio::runtime::Handle::current()),
        None,
        100,
        0,
        signers[0].author(),
    );
    let qc = certificate_for_genesis();
    let root = Block::make_genesis_block();
    let tree = Arc::new(RwLock::new(BlockTree::new(
        root.id(),
        PipelinedBlock::new(root, vec![], StateComputeResult::new_dummy()),
        qc.clone(),
        qc.clone().into_wrapped_ledger_info(),
        qc.into_wrapped_ledger_info(),
        100,
        None,
    )));
    runtime.bind_tree(&tree);
    (runtime, tree, signers)
}

fn insert_chain(
    tree: &Arc<RwLock<BlockTree>>,
    signer: &ValidatorSigner,
    count: u64,
) -> Vec<Arc<PipelinedBlock>> {
    let (blocks, _, _) = prepare_executed_blocks_with_ledger_info(
        signer,
        count,
        HashValue::zero(),
        HashValue::zero(),
        None,
        Some(certificate_for_genesis()),
        1,
    );
    let mut parent = None;
    blocks
        .into_iter()
        .map(|block| {
            let block = tree
                .write()
                .insert_block(Arc::try_unwrap(block).ok().unwrap())
                .unwrap();
            parent = Some(attach_pipeline(&block, parent.take()));
            block
        })
        .collect()
}

#[tokio::test]
async fn certificate_future_accepts_early_decision_without_a_local_vote() {
    let (runtime, tree, signers) = setup();
    let blocks = insert_chain(&tree, &signers[0], 22);
    let target = blocks.last().unwrap();
    let proof = generate_ledger_info_with_sig(
        &signers,
        LedgerInfo::new(target.block_info(), HashValue::zero()),
    );
    let (tx, rx) = tokio::sync::mpsc::channel(100);
    target
        .pipeline_tx()
        .lock()
        .as_mut()
        .unwrap()
        .commit_messages_tx = Some(tx);
    let ordered = aptos_consensus_types::wrapped_ledger_info::WrappedLedgerInfo::new(
        aptos_consensus_types::vote_data::VoteData::dummy(),
        proof.clone(),
    );
    let (execution_tx, execution_rx) = tokio::sync::oneshot::channel();
    let execution = async move { Ok(execution_rx.await.unwrap()) }
        .boxed()
        .shared();
    let certificate_runtime = runtime.clone();
    let block = Arc::new(target.block().clone());
    let certificate = spawn_shared_fut(
        async move {
            certificate_runtime
                .certificate(block, execution, ready(ordered), rx, false)
                .await
        },
        None,
    );
    assert!(runtime.route_for_test(CommitMessage::Decision(CommitDecision::new(proof.clone()))));
    assert!(
        tokio::time::timeout(Duration::from_millis(20), certificate.clone())
            .await
            .is_err()
    );
    execution_tx
        .send((target.compute_result().clone(), Duration::ZERO, None))
        .unwrap();
    assert_eq!(certificate.clone().await.unwrap(), proof);
    runtime.certify_ancestors(certificate).await.unwrap();
    for (index, block) in blocks.iter().enumerate() {
        let futures = block.pipeline_futs().unwrap();
        assert!(futures.commit_vote_fut.clone().now_or_never().is_none());
        let result = tokio::time::timeout(Duration::from_secs(5), futures.commit_ledger_fut)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result, (index == 21).then_some(proof.clone()));
    }
    runtime.drain(true).await;
}

#[tokio::test]
async fn unknown_block_inputs_are_bounded_and_replayed_at_insertion() {
    let (runtime, tree, signers) = setup();
    let (blocks, _, _) = prepare_executed_blocks_with_ledger_info(
        &signers[0],
        1,
        HashValue::zero(),
        HashValue::zero(),
        None,
        Some(certificate_for_genesis()),
        1,
    );
    let block = blocks.into_iter().next().unwrap();
    let proof = generate_ledger_info_with_sig(
        &signers,
        LedgerInfo::new(block.block_info(), HashValue::zero()),
    );
    for _ in 0..1000 {
        let vote = CommitVote::new(
            signers[0].author(),
            proof.ledger_info().clone(),
            &signers[0],
        )
        .unwrap();
        assert!(runtime.route_for_test(CommitMessage::Vote(vote)));
    }
    assert!(runtime.route_for_test(CommitMessage::Decision(CommitDecision::new(proof.clone()))));
    // Inserting a competing same-round proposal must not discard already
    // acknowledged inputs for the block that has not arrived yet.
    let (forks, _, _) = prepare_executed_blocks_with_ledger_info(
        &signers[1],
        1,
        HashValue::zero(),
        HashValue::zero(),
        None,
        Some(certificate_for_genesis()),
        1,
    );
    let fork = tree
        .write()
        .insert_block(
            Arc::try_unwrap(forks.into_iter().next().unwrap())
                .ok()
                .unwrap(),
        )
        .unwrap();
    drop(attach_pipeline(&fork, None));
    let (fork_tx, mut fork_rx) = tokio::sync::mpsc::channel(100);
    fork.pipeline_tx()
        .lock()
        .as_mut()
        .unwrap()
        .commit_messages_tx = Some(fork_tx);
    runtime.deliver_cached(&fork);
    assert!(fork_rx.try_recv().is_err());
    let block = tree
        .write()
        .insert_block(Arc::try_unwrap(block).ok().unwrap())
        .unwrap();
    drop(attach_pipeline(&block, None));
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    block
        .pipeline_tx()
        .lock()
        .as_mut()
        .unwrap()
        .commit_messages_tx = Some(tx);
    runtime.deliver_cached(&block);
    assert!(matches!(rx.try_recv().unwrap(), CommitInput::Vote(_)));
    assert!(matches!(rx.try_recv().unwrap(), CommitInput::Decision(p) if p == proof));
    assert!(rx.try_recv().is_err());
    runtime.drain(false).await;
    assert!(block.pipeline_futs().is_none());
}

#[tokio::test]
async fn reset_drains_uncertified_storage_dependencies_from_the_tree() {
    let (runtime, tree, signers) = setup();
    let blocks = insert_chain(&tree, &signers[0], 22);
    tokio::time::timeout(Duration::from_secs(5), runtime.drain(false))
        .await
        .unwrap();
    assert!(blocks.iter().all(|block| block.pipeline_futs().is_none()));
}

#[tokio::test]
async fn reset_preserves_certified_prefix_and_cancels_the_suffix() {
    let (runtime, tree, signers) = setup();
    let blocks = insert_chain(&tree, &signers[0], 22);
    let proof = generate_ledger_info_with_sig(
        &signers,
        LedgerInfo::new(blocks[4].block_info(), HashValue::zero()),
    );
    runtime
        .certify_ancestors(ready(proof.clone()))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), runtime.drain(true))
        .await
        .unwrap();
    for (index, block) in blocks.iter().enumerate() {
        if index < 5 {
            let commit = block
                .pipeline_futs()
                .unwrap()
                .commit_ledger_fut
                .await
                .unwrap();
            assert_eq!(commit, (index == 4).then_some(proof.clone()));
        } else {
            assert!(block.pipeline_futs().is_none());
        }
    }
}

#[tokio::test]
async fn reset_waits_for_certified_storage_before_cancelling_suffix() {
    let (runtime, tree, signers) = setup();
    let blocks = insert_chain(&tree, &signers[0], 3);
    let target = &blocks[0];
    let mut futures = target.pipeline_futs().unwrap();
    let commit = futures.commit_ledger_fut.clone();
    let (release, wait) = tokio::sync::oneshot::channel();
    futures.commit_ledger_fut = spawn_shared_fut(
        async move {
            wait.await.unwrap();
            commit.await
        },
        None,
    );
    target.set_pipeline_futs(futures);
    let proof = generate_ledger_info_with_sig(
        &signers,
        LedgerInfo::new(target.block_info(), HashValue::zero()),
    );
    runtime
        .certify_ancestors(ready(proof.clone()))
        .await
        .unwrap();
    let draining_runtime = runtime.clone();
    let drain = tokio::spawn(async move { draining_runtime.drain(true).await });
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(!drain.is_finished());
    assert!(blocks[1].pipeline_futs().is_some());
    release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), drain)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        target
            .pipeline_futs()
            .unwrap()
            .commit_ledger_fut
            .await
            .unwrap(),
        Some(proof)
    );
    assert!(blocks[1..]
        .iter()
        .all(|block| block.pipeline_futs().is_none()));
}

#[tokio::test]
async fn epoch_ending_certificate_seals_the_uncertified_suffix() {
    let (runtime, tree, signers) = setup();
    let blocks = insert_chain(&tree, &signers[0], 3);
    let target = &blocks[0];
    let info = target.block_info();
    let next_verifier = Arc::new(random_validator_verifier(1, None, false).1);
    let info = aptos_types::block_info::BlockInfo::new(
        info.epoch(),
        info.round(),
        info.id(),
        info.executed_state_id(),
        info.version(),
        info.timestamp_usecs(),
        Some(EpochState {
            epoch: 2,
            verifier: next_verifier,
        }),
    );
    let proof = generate_ledger_info_with_sig(&signers, LedgerInfo::new(info, HashValue::zero()));
    runtime
        .certify_ancestors(ready(proof.clone()))
        .await
        .unwrap();
    let suffix_proof = generate_ledger_info_with_sig(
        &signers,
        LedgerInfo::new(blocks[2].block_info(), HashValue::zero()),
    );
    assert!(runtime
        .certify_ancestors(ready(suffix_proof.clone()))
        .await
        .is_err());
    assert!(!runtime.route_for_test(CommitMessage::Decision(CommitDecision::new(suffix_proof))));
    assert!(blocks[1]
        .pipeline_tx()
        .lock()
        .as_ref()
        .unwrap()
        .commit_proof_tx
        .is_some());
    tokio::time::timeout(Duration::from_secs(5), runtime.drain(true))
        .await
        .unwrap();
    assert_eq!(
        target
            .pipeline_futs()
            .unwrap()
            .commit_ledger_fut
            .await
            .unwrap(),
        Some(proof)
    );
    assert!(blocks[1..]
        .iter()
        .all(|block| block.pipeline_futs().is_none()));
}

#[tokio::test]
async fn reliable_broadcast_waits_for_batch_ordering_and_stops_on_remote_proof() {
    let (runtime, tree, signers) = setup();
    let blocks = insert_chain(&tree, &signers[0], 2);
    let info = LedgerInfo::new(blocks[0].block_info(), HashValue::zero());
    let vote = CommitVote::new(signers[0].author(), info.clone(), &signers[0]).unwrap();
    let proof = generate_ledger_info_with_sig(&signers, info);
    let (proof_tx, proof_rx) = tokio::sync::oneshot::channel();
    let proof_fut = async move { Ok(proof_rx.await.unwrap()) }.boxed().shared();
    let order = futures::future::pending().boxed().shared();
    let runtime_broadcast = runtime.clone();
    let broadcast_vote = vote.clone();
    let broadcast = spawn_shared_fut(
        async move {
            runtime_broadcast
                .broadcast_vote(ready(broadcast_vote), proof_fut, order)
                .await
        },
        None,
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(20), broadcast.clone())
            .await
            .is_err()
    );
    proof_tx.send(proof).unwrap();
    tokio::time::timeout(Duration::from_secs(5), broadcast)
        .await
        .unwrap()
        .unwrap();
    // An intermediate block does not start reliable retries for its batch.
    let ordered = generate_ledger_info_with_sig(
        &signers,
        LedgerInfo::new(blocks[1].block_info(), HashValue::zero()),
    );
    let ordered = aptos_consensus_types::wrapped_ledger_info::WrappedLedgerInfo::new(
        aptos_consensus_types::vote_data::VoteData::dummy(),
        ordered,
    );
    tokio::time::timeout(
        Duration::from_secs(5),
        runtime.broadcast_vote(
            ready(vote),
            futures::future::pending().boxed().shared(),
            ready(ordered),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    runtime.drain(true).await;
}

#[tokio::test]
async fn intermediate_execution_feedback_does_not_wait_for_a_certificate() {
    let (runtime, tree, signers) = setup();
    let blocks = insert_chain(&tree, &signers[0], 2);
    let intermediate = blocks[0].clone();
    assert!(intermediate.get_execution_summary().is_none());
    let ordered = generate_ledger_info_with_sig(
        &signers,
        LedgerInfo::new(blocks[1].block_info(), HashValue::zero()),
    );
    let ordered = aptos_consensus_types::wrapped_ledger_info::WrappedLedgerInfo::new(
        aptos_consensus_types::vote_data::VoteData::dummy(),
        ordered,
    );
    let (_tx, rx) = tokio::sync::mpsc::channel(1);
    let task_runtime = runtime.clone();
    let block = Arc::new(intermediate.block().clone());
    let execution = intermediate.pipeline_futs().unwrap().ledger_update_fut;
    let task = tokio::spawn(async move {
        task_runtime
            .certificate(block, execution, ready(ordered), rx, false)
            .await
    });
    tokio::time::timeout(Duration::from_secs(5), async {
        while intermediate.get_execution_summary().is_none() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(!task.is_finished());
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    runtime.drain(true).await;
}
