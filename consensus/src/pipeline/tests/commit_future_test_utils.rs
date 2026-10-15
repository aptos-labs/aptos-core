// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    network::NetworkSender,
    network_interface::{ConsensusNetworkClient, DIRECT_SEND, RPC},
};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::network_id::NetworkId;
use aptos_consensus_types::{
    pipeline::commit_vote::CommitVote,
    pipelined_block::{
        DecryptionOutcome, DecryptionResult, PipelineFutures, PipelineInputTx, PipelinedBlock,
        TaskFuture,
    },
};
use aptos_network::{
    application::{interface::NetworkClient, storage::PeersAndMetadata},
    peer_manager::{ConnectionRequestSender, PeerManagerRequestSender},
    protocols::network::{self, NewNetworkSender},
};
use aptos_types::{ledger_info::LedgerInfoWithSignatures, validator_signer::ValidatorSigner};
use futures::FutureExt;
use maplit::hashmap;
use std::{sync::Arc, time::Duration};
use tokio::sync::oneshot;

fn ready<T: Send + Clone + 'static>(value: T) -> TaskFuture<T> {
    async move { Ok(value) }.boxed().shared()
}

pub(in crate::pipeline) fn attach_pipeline(
    block: &Arc<PipelinedBlock>,
    parent: Option<TaskFuture<Option<LedgerInfoWithSignatures>>>,
) -> TaskFuture<Option<LedgerInfoWithSignatures>> {
    let (proof_tx, proof_rx) = oneshot::channel();
    // Match PipelineBuilder: the proof input is abortable, but storage itself
    // must finish and may still be waiting on its parent's storage future.
    let proof_task = tokio::spawn(proof_rx);
    let proof_abort = proof_task.abort_handle();
    let result = block.compute_result().clone();
    let id = block.id();
    let commit_task = tokio::spawn(async move {
        if let Some(parent) = parent {
            parent.await?;
        }
        let proof: LedgerInfoWithSignatures = proof_task
            .await
            .map_err(|e| anyhow::anyhow!(e))?
            .map_err(|_| anyhow::anyhow!("proof cancelled"))?;
        Ok::<_, aptos_consensus_types::pipelined_block::TaskError>(
            (proof.commit_info().id() == id).then_some(proof),
        )
    });
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
        commit_pipeline: None,
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
        commit_messages_tx: None,
        qc_tx: None,
        rand_tx: None,
        order_vote_tx: None,
        order_proof_tx: None,
        ordered_blocks_for_observer_tx: None,
        commit_proof_tx: Some(proof_tx),
        secret_shared_key_tx: None,
    });
    block.set_pipeline_abort_handles(vec![proof_abort, vote_abort]);
    commit
}

pub(in crate::pipeline) fn network(
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
