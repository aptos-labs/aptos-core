// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    network::{IncomingCommitRequest, NetworkSender},
    network_interface::{ConsensusNetworkClient, DIRECT_SEND, RPC},
    pipeline::{
        commit_pipeline::{CommitPipeline, OrderedBlocks, ResetAck, ResetRequest, ResetSignal},
        commit_reliable_broadcast::CommitMessage,
        tests::test_utils::prepare_executed_blocks_with_ledger_info,
    },
};
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::{config::ConsensusObserverConfig, network_id::NetworkId};
use aptos_consensus_types::{
    block::block_test_utils::certificate_for_genesis,
    pipeline::{commit_decision::CommitDecision, commit_vote::CommitVote},
    pipelined_block::{
        DecryptionOutcome, DecryptionResult, PipelineFutures, PipelineInputTx, PipelinedBlock,
        TaskFuture,
    },
};
use aptos_crypto::HashValue;
use aptos_network::{
    application::{interface::NetworkClient, storage::PeersAndMetadata},
    peer_manager::{ConnectionRequestSender, PeerManagerRequestSender},
    protocols::{
        network::{self, NewNetworkSender},
        wire::handshake::v1::ProtocolId,
    },
};
use aptos_types::{
    epoch_state::EpochState,
    ledger_info::{generate_ledger_info_with_sig, LedgerInfo, LedgerInfoWithSignatures},
    validator_signer::ValidatorSigner,
    validator_verifier::random_validator_verifier,
};
use futures::{channel::mpsc::unbounded, FutureExt, SinkExt};
use maplit::hashmap;
use std::{sync::Arc, time::Duration};
use tokio::sync::oneshot;

fn ready<T: Send + Clone + 'static>(value: T) -> TaskFuture<T> {
    async move { Ok(value) }.boxed().shared()
}

fn attach_pipeline(
    block: &Arc<PipelinedBlock>,
    parent: Option<TaskFuture<Option<LedgerInfoWithSignatures>>>,
) -> TaskFuture<Option<LedgerInfoWithSignatures>> {
    let (proof_tx, proof_rx) = oneshot::channel();
    let result = block.compute_result().clone();
    let id = block.id();
    let commit_task = tokio::spawn(async move {
        if let Some(parent) = parent {
            parent.await?;
        }
        let proof: LedgerInfoWithSignatures = proof_rx
            .await
            .map_err(|_| anyhow::anyhow!("proof cancelled"))?;
        Ok::<_, aptos_consensus_types::pipelined_block::TaskError>(
            (proof.commit_info().id() == id).then_some(proof),
        )
    });
    let commit_abort = commit_task.abort_handle();
    let commit = async move { commit_task.await.map_err(|e| anyhow::anyhow!(e))? }
        .boxed()
        .shared();
    // A remote certificate must work even when our own signing has not completed.
    let vote_task = tokio::spawn(std::future::pending::<
        aptos_consensus_types::pipelined_block::TaskResult<CommitVote>,
    >());
    let vote_abort = vote_task.abort_handle();
    let vote = async move { vote_task.await.map_err(|e| anyhow::anyhow!(e))? }
        .boxed()
        .shared();
    block.set_pipeline_futs(PipelineFutures {
        decryption_fut: ready(DecryptionResult {
            decrypted_txns: vec![],
            retry_txns: vec![],
            regular_txns: vec![],
            max_txns_from_block_to_execute: None,
            block_gas_limit: None,
            outcome: DecryptionOutcome::Disabled,
        }),
        prepare_fut: ready((Arc::new(vec![]), None, DecryptionOutcome::Disabled)),
        has_rand_txns_fut: ready(false),
        rand_check_fut: ready((None, false)),
        execute_fut: ready(Duration::ZERO),
        ledger_update_fut: ready((result.clone(), Duration::ZERO, None)),
        observer_publish_fut: ready(()),
        post_ledger_update_fut: ready(()),
        commit_vote_fut: vote,
        pre_commit_fut: ready(result),
        notify_state_sync_fut: ready(()),
        commit_ledger_fut: commit.clone(),
        post_commit_fut: ready(()),
        secret_sharing_derive_self_fut: ready(None),
    });
    block.set_pipeline_tx(PipelineInputTx {
        qc_tx: None,
        rand_tx: None,
        order_vote_tx: None,
        order_proof_tx: None,
        ordered_blocks_for_observer_tx: None,
        commit_proof_tx: Some(proof_tx),
        secret_shared_key_tx: None,
    });
    block.set_pipeline_abort_handles(vec![commit_abort, vote_abort]);
    commit
}

fn network(
    signer: &ValidatorSigner,
    verifier: Arc<aptos_types::validator_verifier::ValidatorVerifier>,
) -> Arc<NetworkSender> {
    let (requests, _) = aptos_channel::new(QueueStyle::FIFO, 8, None);
    let (connections, _) = aptos_channel::new(QueueStyle::FIFO, 8, None);
    let sender = network::NetworkSender::new(
        PeerManagerRequestSender::new(requests),
        ConnectionRequestSender::new(connections),
    );
    let client = NetworkClient::new(
        DIRECT_SEND.into(),
        RPC.into(),
        hashmap! {NetworkId::Validator => sender},
        PeersAndMetadata::new(&[NetworkId::Validator]),
    );
    let (self_sender, _) = aptos_channels::new_unbounded_test();
    Arc::new(NetworkSender::new(
        signer.author(),
        ConsensusNetworkClient::new(client),
        self_sender,
        verifier,
    ))
}

#[tokio::test]
async fn cached_descendant_proof_commits_twenty_two_block_prefix_without_local_votes() {
    let (signers, verifier) = random_validator_verifier(1, None, false);
    let verifier = Arc::new(verifier);
    let signer = &signers[0];
    let (mut block_tx, block_rx) = unbounded();
    let (mut reset_tx, reset_rx) = unbounded();
    let (message_tx, message_rx) = aptos_channel::new(QueueStyle::FIFO, 100, None);
    let service = CommitPipeline::new(
        signer.author(),
        network(signer, verifier.clone()),
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
    let task = tokio::spawn(service.start());
    let (blocks, proof, _) = prepare_executed_blocks_with_ledger_info(
        signer,
        22,
        HashValue::zero(),
        HashValue::zero(),
        None,
        Some(certificate_for_genesis()),
        1,
    );
    let (ack_tx, ack_rx) = futures::channel::oneshot::channel();
    message_tx
        .push(
            signer.author(),
            (signer.author(), IncomingCommitRequest {
                req: CommitMessage::Decision(CommitDecision::new(proof.clone())),
                protocol: ProtocolId::ConsensusRpcBcs,
                response_sender: ack_tx,
            }),
        )
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), ack_rx)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let mut parent = None;
    let mut commits = vec![];
    for block in blocks {
        let commit = attach_pipeline(&block, parent);
        parent = Some(commit.clone());
        commits.push(commit);
        let ordered_proof = generate_ledger_info_with_sig(
            &signers,
            LedgerInfo::new(block.block_info(), HashValue::zero()),
        );
        block_tx
            .send(OrderedBlocks {
                ordered_blocks: vec![block],
                ordered_proof,
            })
            .await
            .unwrap();
    }
    for (i, commit) in commits.into_iter().enumerate() {
        let result = tokio::time::timeout(Duration::from_secs(5), commit)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result, (i == 21).then_some(proof.clone()));
    }
    let (tx, rx) = futures::channel::oneshot::channel::<ResetAck>();
    reset_tx
        .send(ResetRequest {
            tx,
            signal: ResetSignal::Stop,
        })
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), rx)
        .await
        .unwrap()
        .unwrap();
    task.await.unwrap();
}

#[tokio::test]
async fn reset_cancels_uncommitted_pipeline_and_acknowledges_drain() {
    let (signers, verifier) = random_validator_verifier(1, None, false);
    let verifier = Arc::new(verifier);
    let signer = &signers[0];
    let (mut block_tx, block_rx) = unbounded();
    let (mut reset_tx, reset_rx) = unbounded();
    let (_message_tx, message_rx) = aptos_channel::new(QueueStyle::FIFO, 100, None);
    let service = CommitPipeline::new(
        signer.author(),
        network(signer, verifier.clone()),
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
    let task = tokio::spawn(service.start());
    let (blocks, proof, _) = prepare_executed_blocks_with_ledger_info(
        signer,
        1,
        HashValue::zero(),
        HashValue::zero(),
        None,
        Some(certificate_for_genesis()),
        1,
    );
    let block = blocks[0].clone();
    let commit = attach_pipeline(&block, None);
    block_tx
        .send(OrderedBlocks {
            ordered_blocks: blocks,
            ordered_proof: proof,
        })
        .await
        .unwrap();
    let (tx, rx) = futures::channel::oneshot::channel();
    reset_tx
        .send(ResetRequest {
            tx,
            signal: ResetSignal::TargetRound(1),
        })
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), rx)
        .await
        .unwrap()
        .unwrap();
    assert!(commit.await.is_err());
    assert!(block.pipeline_futs().is_none());
    let (tx, rx) = futures::channel::oneshot::channel();
    reset_tx
        .send(ResetRequest {
            tx,
            signal: ResetSignal::Stop,
        })
        .await
        .unwrap();
    rx.await.unwrap();
    task.await.unwrap();
}
