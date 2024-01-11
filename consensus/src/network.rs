// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::tracing::{observe_block, BlockStage},
    counters,
    dag::{
        DAGMessage, DAGNetworkMessage, DAGRpcResult, ProofNotifier, RpcWithFallback,
        TDAGNetworkSender,
    },
    logging::{LogEvent, LogSchema},
    monitor,
    network_interface::{ConsensusMsg, ConsensusNetworkClient, RPC},
    pipeline::commit_reliable_broadcast::CommitMessage,
    quorum_store::types::{Batch, BatchMsg, BatchRequest, BatchResponse},
    rand::rand_gen::RandGenMessage,
};
use anyhow::{anyhow, bail, ensure};
use aptos_channels::{self, aptos_channel, message_queues::QueueStyle};
use aptos_config::network_id::NetworkId;
use aptos_consensus_types::{
    block_retrieval::{BlockRetrievalRequest, BlockRetrievalResponse},
    common::Author,
    pipeline::{commit_decision::CommitDecision, commit_vote::CommitVote},
    proof_of_store::{ProofOfStore, ProofOfStoreMsg, SignedBatchInfo, SignedBatchInfoMsg},
    proposal_msg::ProposalMsg,
    sync_info::SyncInfo,
    vote_msg::VoteMsg,
};
use aptos_logger::prelude::*;
use aptos_network::{
    application::interface::{NetworkClient, NetworkServiceEvents},
    protocols::{network::Event, rpc::error::RpcError},
    ProtocolId,
};
use aptos_reliable_broadcast::{RBMessage, RBNetworkSender};
use aptos_types::{
    account_address::AccountAddress, epoch_change::EpochChangeProof,
    ledger_info::LedgerInfoWithSignatures, validator_verifier::ValidatorVerifier,
};
use async_trait::async_trait;
use bytes::Bytes;
use fail::fail_point;
use futures::{
    channel::oneshot,
    stream::{select, select_all},
    SinkExt, Stream, StreamExt,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    mem::{discriminant, Discriminant},
    sync::Arc,
    time::Duration,
};
use tokio::time::timeout;

pub trait TConsensusMsg: Sized + Serialize + DeserializeOwned {
    fn epoch(&self) -> u64;

    fn from_network_message(msg: ConsensusMsg) -> anyhow::Result<Self>;

    fn into_network_message(self) -> ConsensusMsg;
}

#[derive(Debug)]
pub struct RpcResponder {
    pub protocol: ProtocolId,
    pub response_sender: oneshot::Sender<Result<Bytes, RpcError>>,
}

impl RpcResponder {
    pub fn respond<R>(self, response: R) -> anyhow::Result<()>
    where
        R: TConsensusMsg,
    {
        let rpc_response = self
            .protocol
            .to_bytes(&response.into_network_message())
            .map(Bytes::from)
            .map_err(RpcError::Error);

        self.response_sender
            .send(rpc_response)
            .map_err(|_| anyhow::anyhow!("unable to respond to rpc"))
    }
}

/// The block retrieval request is used internally for implementing RPC: the callback is executed
/// for carrying the response
#[derive(Debug)]
pub struct IncomingBlockRetrievalRequest {
    pub req: BlockRetrievalRequest,
    pub protocol: ProtocolId,
    pub response_sender: oneshot::Sender<Result<Bytes, RpcError>>,
}

#[derive(Debug)]
pub struct IncomingBatchRetrievalRequest {
    pub req: BatchRequest,
    pub protocol: ProtocolId,
    pub response_sender: oneshot::Sender<Result<Bytes, RpcError>>,
}

#[derive(Debug)]
pub struct IncomingDAGRequest {
    pub req: DAGNetworkMessage,
    pub sender: Author,
    pub responder: RpcResponder,
}

#[derive(Debug)]
pub struct IncomingCommitRequest {
    pub req: CommitMessage,
    pub protocol: ProtocolId,
    pub response_sender: oneshot::Sender<Result<Bytes, RpcError>>,
}

#[derive(Debug)]
pub struct IncomingRandGenRequest {
    pub req: RandGenMessage,
    pub sender: Author,
    pub protocol: ProtocolId,
    pub response_sender: oneshot::Sender<Result<Bytes, RpcError>>,
}

#[derive(Debug)]
pub enum IncomingRpcRequest {
    BlockRetrieval(IncomingBlockRetrievalRequest),
    BatchRetrieval(IncomingBatchRetrievalRequest),
    DAGRequest(IncomingDAGRequest),
    CommitRequest(IncomingCommitRequest),
    RandGenRequest(IncomingRandGenRequest),
}

/// Just a convenience struct to keep all the network proxy receiving queues in one place.
/// Will be returned by the NetworkTask upon startup.
pub struct NetworkReceivers {
    /// Provide a LIFO buffer for each (Author, MessageType) key
    pub consensus_messages: aptos_channel::Receiver<
        (AccountAddress, Discriminant<ConsensusMsg>),
        (AccountAddress, ConsensusMsg),
    >,
    pub quorum_store_messages: aptos_channel::Receiver<
        (AccountAddress, Discriminant<ConsensusMsg>),
        (AccountAddress, ConsensusMsg),
    >,
    pub rpc_rx: aptos_channel::Receiver<
        (AccountAddress, Discriminant<IncomingRpcRequest>),
        (AccountAddress, IncomingRpcRequest),
    >,
}

#[async_trait::async_trait]
pub trait QuorumStoreSender: Send + Clone {
    async fn send_batch_request(&self, request: BatchRequest, recipients: Vec<Author>);

    async fn request_batch(
        &self,
        request: BatchRequest,
        recipient: Author,
        timeout: Duration,
    ) -> anyhow::Result<BatchResponse>;

    async fn send_batch(&self, batch: Batch, recipients: Vec<Author>);

    async fn send_signed_batch_info_msg(
        &self,
        signed_batch_infos: Vec<SignedBatchInfo>,
        recipients: Vec<Author>,
    );

    async fn broadcast_batch_msg(&mut self, batches: Vec<Batch>);

    async fn broadcast_proof_of_store_msg(&mut self, proof_of_stores: Vec<ProofOfStore>);

    async fn send_proof_of_store_msg_to_self(&mut self, proof_of_stores: Vec<ProofOfStore>);
}

/// Implements the actual networking support for all consensus messaging.
#[derive(Clone)]
pub struct NetworkSender {
    author: Author,
    consensus_network_client: ConsensusNetworkClient<NetworkClient<ConsensusMsg>>,
    // Self sender and self receivers provide a shortcut for sending the messages to itself.
    // (self sending is not supported by the networking API).
    self_sender: aptos_channels::Sender<Event<ConsensusMsg>>,
    validators: ValidatorVerifier,
    time_service: aptos_time_service::TimeService,
}

impl NetworkSender {
    pub fn new(
        author: Author,
        consensus_network_client: ConsensusNetworkClient<NetworkClient<ConsensusMsg>>,
        self_sender: aptos_channels::Sender<Event<ConsensusMsg>>,
        validators: ValidatorVerifier,
    ) -> Self {
        NetworkSender {
            author,
            consensus_network_client,
            self_sender,
            validators,
            time_service: aptos_time_service::TimeService::real(),
        }
    }

    /// Tries to retrieve num of blocks backwards starting from id from the given peer: the function
    /// returns a future that is fulfilled with BlockRetrievalResponse.
    pub async fn request_block(
        &self,
        retrieval_request: BlockRetrievalRequest,
        from: Author,
        timeout: Duration,
    ) -> anyhow::Result<BlockRetrievalResponse> {
        fail_point!("consensus::send::any", |_| {
            Err(anyhow::anyhow!("Injected error in request_block"))
        });
        fail_point!("consensus::send::block_retrieval", |_| {
            Err(anyhow::anyhow!("Injected error in request_block"))
        });

        ensure!(from != self.author, "Retrieve block from self");
        let msg = ConsensusMsg::BlockRetrievalRequest(Box::new(retrieval_request.clone()));
        counters::CONSENSUS_SENT_MSGS
            .with_label_values(&[msg.name()])
            .inc();
        let response_msg = monitor!(
            "block_retrieval",
            self.consensus_network_client
                .send_rpc(from, msg, timeout)
                .await
        )?;
        let response = match response_msg {
            ConsensusMsg::BlockRetrievalResponse(resp) => *resp,
            _ => return Err(anyhow!("Invalid response to request")),
        };
        response
            .verify(retrieval_request, &self.validators)
            .map_err(|e| {
                error!(
                    SecurityEvent::InvalidRetrievedBlock,
                    request_block_response = response,
                    error = ?e,
                );
                e
            })?;

        Ok(response)
    }

    pub async fn send_rpc(
        &self,
        receiver: Author,
        msg: ConsensusMsg,
        timeout_duration: Duration,
    ) -> anyhow::Result<ConsensusMsg> {
        counters::CONSENSUS_SENT_MSGS
            .with_label_values(&[msg.name()])
            .inc();
        if receiver == self.author() {
            let (tx, rx) = oneshot::channel();
            let protocol = RPC[0];
            let self_msg = Event::RpcRequest(receiver, msg.clone(), RPC[0], tx);
            self.self_sender.clone().send(self_msg).await?;
            if let Ok(Ok(Ok(bytes))) = timeout(timeout_duration, rx).await {
                Ok(protocol.from_bytes(&bytes)?)
            } else {
                bail!("self rpc failed");
            }
        } else {
            Ok(monitor!(
                "send_rpc",
                self.consensus_network_client
                    .send_rpc(receiver, msg, timeout_duration)
                    .await
            )?)
        }
    }

    /// Tries to send the given msg to all the participants.
    ///
    /// The future is fulfilled as soon as the message is put into the mpsc channel to network
    /// internal (to provide back pressure), it does not indicate the message is delivered or sent
    /// out.
    async fn broadcast(&mut self, msg: ConsensusMsg) {
        fail_point!("consensus::send::any", |_| ());
        // Directly send the message to ourself without going through network.
        let self_msg = Event::Message(self.author, msg.clone());
        if let Err(err) = self.self_sender.send(self_msg).await {
            error!("Error broadcasting to self: {:?}", err);
        }

        // Get the list of validators excluding our own account address. Note the
        // ordering is not important in this case.
        let self_author = self.author;
        let other_validators: Vec<_> = self
            .validators
            .get_ordered_account_addresses_iter()
            .filter(|author| author != &self_author)
            .collect();

        counters::CONSENSUS_SENT_MSGS
            .with_label_values(&[msg.name()])
            .inc_by(other_validators.len() as u64);
        // Broadcast message over direct-send to all other validators.
        if let Err(err) = self
            .consensus_network_client
            .send_to_many(other_validators.into_iter(), msg)
        {
            warn!(error = ?err, "Error broadcasting message");
        }
    }

    pub fn broadcast_without_self(&self, msg: ConsensusMsg) {
        let self_author = self.author;
        let other_validators: Vec<_> = self
            .validators
            .get_ordered_account_addresses_iter()
            .filter(|author| author != &self_author)
            .collect();

        counters::CONSENSUS_SENT_MSGS
            .with_label_values(&[msg.name()])
            .inc_by(other_validators.len() as u64);
        // Broadcast message over direct-send to all other validators.
        if let Err(err) = self
            .consensus_network_client
            .send_to_many(other_validators.into_iter(), msg)
        {
            warn!(error = ?err, "Error broadcasting message");
        }
    }

    /// Tries to send msg to given recipients.
    async fn send(&self, msg: ConsensusMsg, recipients: Vec<Author>) {
        fail_point!("consensus::send::any", |_| ());
        let network_sender = self.consensus_network_client.clone();
        let mut self_sender = self.self_sender.clone();
        for peer in recipients {
            if self.author == peer {
                let self_msg = Event::Message(self.author, msg.clone());
                if let Err(err) = self_sender.send(self_msg).await {
                    error!(error = ?err, "Error delivering a self msg");
                }
                continue;
            }
            counters::CONSENSUS_SENT_MSGS
                .with_label_values(&[msg.name()])
                .inc();
            if let Err(e) = network_sender.send_to(peer, msg.clone()) {
                warn!(
                    remote_peer = peer,
                    error = ?e, "Failed to send a msg to peer",
                );
            }
        }
    }

    pub async fn broadcast_proposal(&mut self, proposal_msg: ProposalMsg) {
        fail_point!("consensus::send::broadcast_proposal", |_| ());
        let msg = ConsensusMsg::ProposalMsg(Box::new(proposal_msg));
        self.broadcast(msg).await
    }

    pub async fn broadcast_sync_info(&mut self, sync_info_msg: SyncInfo) {
        fail_point!("consensus::send::broadcast_sync_info", |_| ());
        let msg = ConsensusMsg::SyncInfo(Box::new(sync_info_msg));
        self.broadcast(msg).await
    }

    pub async fn broadcast_timeout_vote(&mut self, timeout_vote_msg: VoteMsg) {
        fail_point!("consensus::send::broadcast_timeout_vote", |_| ());
        let msg = ConsensusMsg::VoteMsg(Box::new(timeout_vote_msg));
        self.broadcast(msg).await
    }

    pub async fn broadcast_epoch_change(&mut self, epoch_change_proof: EpochChangeProof) {
        fail_point!("consensus::send::broadcast_epoch_change", |_| ());
        let msg = ConsensusMsg::EpochChangeProof(Box::new(epoch_change_proof));
        self.broadcast(msg).await
    }

    pub async fn send_commit_vote(
        &self,
        commit_vote: CommitVote,
        recipient: Author,
    ) -> anyhow::Result<()> {
        fail_point!("consensus::send::commit_vote", |_| Ok(()));
        let msg = ConsensusMsg::CommitMessage(Box::new(CommitMessage::Vote(commit_vote)));
        self.send_rpc(recipient, msg, Duration::from_millis(500))
            .await
            .map(|_| ())
    }

    /// Sends the vote to the chosen recipients (typically that would be the recipients that
    /// we believe could serve as proposers in the next round). The recipients on the receiving
    /// end are going to be notified about a new vote in the vote queue.
    ///
    /// The future is fulfilled as soon as the message put into the mpsc channel to network
    /// internal(to provide back pressure), it does not indicate the message is delivered or sent
    /// out. It does not give indication about when the message is delivered to the recipients,
    /// as well as there is no indication about the network failures.
    pub async fn send_vote(&self, vote_msg: VoteMsg, recipients: Vec<Author>) {
        fail_point!("consensus::send::vote", |_| ());
        let msg = ConsensusMsg::VoteMsg(Box::new(vote_msg));
        self.send(msg, recipients).await
    }

    #[cfg(feature = "failpoints")]
    pub async fn send_proposal(&self, proposal_msg: ProposalMsg, recipients: Vec<Author>) {
        fail_point!("consensus::send::proposal", |_| ());
        let msg = ConsensusMsg::ProposalMsg(Box::new(proposal_msg));
        self.send(msg, recipients).await
    }

    pub async fn send_epoch_change(&self, proof: EpochChangeProof) {
        fail_point!("consensus::send::epoch_change", |_| ());
        let msg = ConsensusMsg::EpochChangeProof(Box::new(proof));
        self.send(msg, vec![self.author]).await
    }

    /// Sends the ledger info to self buffer manager
    pub async fn send_commit_proof(&self, ledger_info: LedgerInfoWithSignatures) {
        fail_point!("consensus::send::commit_decision", |_| ());
        let msg = ConsensusMsg::CommitMessage(Box::new(CommitMessage::Decision(
            CommitDecision::new(ledger_info),
        )));
        let _ = self
            .send_rpc(self.author, msg, Duration::from_millis(500))
            .await;
    }

    pub fn author(&self) -> Author {
        self.author
    }
}

#[async_trait::async_trait]
impl QuorumStoreSender for NetworkSender {
    async fn send_batch_request(&self, request: BatchRequest, recipients: Vec<Author>) {
        fail_point!("consensus::send::batch_request", |_| ());
        let msg = ConsensusMsg::BatchRequestMsg(Box::new(request));
        self.send(msg, recipients).await
    }

    async fn request_batch(
        &self,
        request: BatchRequest,
        recipient: Author,
        timeout: Duration,
    ) -> anyhow::Result<BatchResponse> {
        let request_digest = request.digest();
        let msg = ConsensusMsg::BatchRequestMsg(Box::new(request));
        let response = self
            .consensus_network_client
            .send_rpc(recipient, msg, timeout)
            .await?;
        match response {
            // TODO: deprecated, remove after two releases
            ConsensusMsg::BatchResponse(batch) => {
                batch.verify_with_digest(request_digest)?;
                Ok(BatchResponse::Batch(*batch))
            },
            ConsensusMsg::BatchResponseV2(maybe_batch) => {
                if let BatchResponse::Batch(batch) = maybe_batch.as_ref() {
                    batch.verify_with_digest(request_digest)?;
                }
                // Note BatchResponse::NotFound(ledger_info) is verified later with a ValidatorVerifier
                Ok(*maybe_batch)
            },
            _ => Err(anyhow!("Invalid batch response")),
        }
    }

    async fn send_batch(&self, batch: Batch, recipients: Vec<Author>) {
        fail_point!("consensus::send::batch", |_| ());
        let msg = ConsensusMsg::BatchResponse(Box::new(batch));
        self.send(msg, recipients).await
    }

    async fn send_signed_batch_info_msg(
        &self,
        signed_batch_infos: Vec<SignedBatchInfo>,
        recipients: Vec<Author>,
    ) {
        fail_point!("consensus::send::signed_batch_info", |_| ());
        let msg =
            ConsensusMsg::SignedBatchInfo(Box::new(SignedBatchInfoMsg::new(signed_batch_infos)));
        self.send(msg, recipients).await
    }

    async fn broadcast_batch_msg(&mut self, batches: Vec<Batch>) {
        fail_point!("consensus::send::broadcast_batch", |_| ());
        let msg = ConsensusMsg::BatchMsg(Box::new(BatchMsg::new(batches)));
        self.broadcast(msg).await
    }

    async fn broadcast_proof_of_store_msg(&mut self, proofs: Vec<ProofOfStore>) {
        fail_point!("consensus::send::proof_of_store", |_| ());
        let msg = ConsensusMsg::ProofOfStoreMsg(Box::new(ProofOfStoreMsg::new(proofs)));
        self.broadcast(msg).await
    }

    async fn send_proof_of_store_msg_to_self(&mut self, proofs: Vec<ProofOfStore>) {
        fail_point!("consensus::send::proof_of_store", |_| ());
        let msg = ConsensusMsg::ProofOfStoreMsg(Box::new(ProofOfStoreMsg::new(proofs)));
        self.send(msg, vec![self.author]).await
    }
}

#[async_trait]
impl TDAGNetworkSender for NetworkSender {
    async fn send_rpc(
        &self,
        receiver: Author,
        message: DAGMessage,
        timeout: Duration,
    ) -> anyhow::Result<DAGRpcResult> {
        self.send_rpc(receiver, message.into_network_message(), timeout)
            .await
            .map_err(|e| anyhow!("invalid rpc response: {}", e))
            .and_then(TConsensusMsg::from_network_message)
    }

    /// Given a list of potential responders, sending rpc to get response from any of them and could
    /// fallback to more in case of failures.
    async fn send_rpc_with_fallbacks(
        self: Arc<Self>,
        responders: Vec<Author>,
        message: DAGMessage,
        retry_interval: Duration,
        rpc_timeout: Duration,
        min_concurrent_responders: u32,
        max_concurrent_responders: u32,
    ) -> RpcWithFallback {
        RpcWithFallback::new(
            responders,
            message,
            retry_interval,
            rpc_timeout,
            self.clone(),
            self.time_service.clone(),
            min_concurrent_responders,
            max_concurrent_responders,
        )
    }
}

#[async_trait]
impl<Req: TConsensusMsg + RBMessage + 'static, Res: TConsensusMsg + RBMessage + 'static>
    RBNetworkSender<Req, Res> for NetworkSender
{
    async fn send_rb_rpc(
        &self,
        receiver: Author,
        message: Req,
        timeout: Duration,
    ) -> anyhow::Result<Res> {
        self.send_rpc(receiver, message.into_network_message(), timeout)
            .await
            .map_err(|e| anyhow!("invalid rpc response: {}", e))
            .and_then(TConsensusMsg::from_network_message)
    }
}

#[async_trait]
impl ProofNotifier for NetworkSender {
    async fn send_epoch_change(&self, proof: EpochChangeProof) {
        self.send_epoch_change(proof).await
    }

    async fn send_commit_proof(&self, ledger_info: LedgerInfoWithSignatures) {
        self.send_commit_proof(ledger_info).await
    }
}

pub struct NetworkTask {
    consensus_messages_tx: aptos_channel::Sender<
        (AccountAddress, Discriminant<ConsensusMsg>),
        (AccountAddress, ConsensusMsg),
    >,
    quorum_store_messages_tx: aptos_channel::Sender<
        (AccountAddress, Discriminant<ConsensusMsg>),
        (AccountAddress, ConsensusMsg),
    >,
    rpc_tx: aptos_channel::Sender<
        (AccountAddress, Discriminant<IncomingRpcRequest>),
        (AccountAddress, IncomingRpcRequest),
    >,
    all_events: Box<dyn Stream<Item = Event<ConsensusMsg>> + Send + Unpin>,
}

impl NetworkTask {
    /// Establishes the initial connections with the peers and returns the receivers.
    pub fn new(
        network_service_events: NetworkServiceEvents<ConsensusMsg>,
        self_receiver: aptos_channels::Receiver<Event<ConsensusMsg>>,
    ) -> (NetworkTask, NetworkReceivers) {
        let (consensus_messages_tx, consensus_messages) = aptos_channel::new(
            QueueStyle::FIFO,
            10,
            Some(&counters::CONSENSUS_CHANNEL_MSGS),
        );
        let (quorum_store_messages_tx, quorum_store_messages) = aptos_channel::new(
            QueueStyle::FIFO,
            // TODO: tune this value based on quorum store messages with backpressure
            50,
            Some(&counters::QUORUM_STORE_CHANNEL_MSGS),
        );
        let (rpc_tx, rpc_rx) =
            aptos_channel::new(QueueStyle::FIFO, 10, Some(&counters::RPC_CHANNEL_MSGS));

        // Verify the network events have been constructed correctly
        let network_and_events = network_service_events.into_network_and_events();
        if (network_and_events.values().len() != 1)
            || !network_and_events.contains_key(&NetworkId::Validator)
        {
            panic!("The network has not been setup correctly for consensus!");
        }

        // Collect all the network events into a single stream
        let network_events: Vec<_> = network_and_events.into_values().collect();
        let network_events = select_all(network_events).fuse();
        let all_events = Box::new(select(network_events, self_receiver));

        (
            NetworkTask {
                consensus_messages_tx,
                quorum_store_messages_tx,
                rpc_tx,
                all_events,
            },
            NetworkReceivers {
                consensus_messages,
                quorum_store_messages,
                rpc_rx,
            },
        )
    }

    fn push_msg(
        peer_id: AccountAddress,
        msg: ConsensusMsg,
        tx: &aptos_channel::Sender<
            (AccountAddress, Discriminant<ConsensusMsg>),
            (AccountAddress, ConsensusMsg),
        >,
    ) {
        if let Err(e) = tx.push((peer_id, discriminant(&msg)), (peer_id, msg)) {
            warn!(
                remote_peer = peer_id,
                error = ?e, "Error pushing consensus msg",
            );
        }
    }

    pub async fn start(mut self) {
        while let Some(message) = self.all_events.next().await {
            monitor!("network_main_loop", match message {
                Event::Message(peer_id, msg) => {
                    counters::CONSENSUS_RECEIVED_MSGS
                        .with_label_values(&[msg.name()])
                        .inc();
                    match msg {
                        quorum_store_msg @ (ConsensusMsg::SignedBatchInfo(_)
                        | ConsensusMsg::BatchMsg(_)
                        | ConsensusMsg::ProofOfStoreMsg(_)) => {
                            Self::push_msg(
                                peer_id,
                                quorum_store_msg,
                                &self.quorum_store_messages_tx,
                            );
                        },
                        // Remove after migration to use rpc.
                        ConsensusMsg::CommitVoteMsg(commit_vote) => {
                            let (tx, _rx) = oneshot::channel();
                            let req_with_callback =
                                IncomingRpcRequest::CommitRequest(IncomingCommitRequest {
                                    req: CommitMessage::Vote(*commit_vote),
                                    protocol: RPC[0],
                                    response_sender: tx,
                                });
                            if let Err(e) = self.rpc_tx.push(
                                (peer_id, discriminant(&req_with_callback)),
                                (peer_id, req_with_callback),
                            ) {
                                warn!(error = ?e, "aptos channel closed");
                            };
                        },
                        ConsensusMsg::CommitDecisionMsg(commit_decision) => {
                            let (tx, _rx) = oneshot::channel();
                            let req_with_callback =
                                IncomingRpcRequest::CommitRequest(IncomingCommitRequest {
                                    req: CommitMessage::Decision(*commit_decision),
                                    protocol: RPC[0],
                                    response_sender: tx,
                                });
                            if let Err(e) = self.rpc_tx.push(
                                (peer_id, discriminant(&req_with_callback)),
                                (peer_id, req_with_callback),
                            ) {
                                warn!(error = ?e, "aptos channel closed");
                            };
                        },
                        consensus_msg @ (ConsensusMsg::ProposalMsg(_)
                        | ConsensusMsg::VoteMsg(_)
                        | ConsensusMsg::SyncInfo(_)
                        | ConsensusMsg::EpochRetrievalRequest(_)
                        | ConsensusMsg::EpochChangeProof(_)) => {
                            if let ConsensusMsg::ProposalMsg(proposal) = &consensus_msg {
                                observe_block(
                                    proposal.proposal().timestamp_usecs(),
                                    BlockStage::NETWORK_RECEIVED,
                                );
                                info!(
                                    LogSchema::new(LogEvent::NetworkReceiveProposal)
                                        .remote_peer(peer_id),
                                    block_round = proposal.proposal().round(),
                                    block_hash = proposal.proposal().id(),
                                );
                            }
                            Self::push_msg(peer_id, consensus_msg, &self.consensus_messages_tx);
                        },
                        // TODO: get rid of the rpc dummy value
                        ConsensusMsg::RandGenMessage(req) => {
                            let (tx, _rx) = oneshot::channel();
                            let req_with_callback =
                                IncomingRpcRequest::RandGenRequest(IncomingRandGenRequest {
                                    req,
                                    sender: peer_id,
                                    protocol: RPC[0],
                                    response_sender: tx,
                                });
                            if let Err(e) = self.rpc_tx.push(
                                (peer_id, discriminant(&req_with_callback)),
                                (peer_id, req_with_callback),
                            ) {
                                warn!(error = ?e, "aptos channel closed");
                            };
                        },
                        _ => {
                            warn!(remote_peer = peer_id, "Unexpected direct send msg");
                            continue;
                        },
                    }
                },
                Event::RpcRequest(peer_id, msg, protocol, callback) => {
                    counters::CONSENSUS_RECEIVED_MSGS
                        .with_label_values(&[msg.name()])
                        .inc();
                    let req = match msg {
                        ConsensusMsg::BlockRetrievalRequest(request) => {
                            debug!(
                                remote_peer = peer_id,
                                event = LogEvent::ReceiveBlockRetrieval,
                                "{}",
                                request
                            );
                            IncomingRpcRequest::BlockRetrieval(IncomingBlockRetrievalRequest {
                                req: *request,
                                protocol,
                                response_sender: callback,
                            })
                        },
                        ConsensusMsg::BatchRequestMsg(request) => {
                            debug!(
                                remote_peer = peer_id,
                                event = LogEvent::ReceiveBatchRetrieval,
                                "{}",
                                request
                            );
                            IncomingRpcRequest::BatchRetrieval(IncomingBatchRetrievalRequest {
                                req: *request,
                                protocol,
                                response_sender: callback,
                            })
                        },
                        ConsensusMsg::DAGMessage(req) => {
                            IncomingRpcRequest::DAGRequest(IncomingDAGRequest {
                                req,
                                sender: peer_id,
                                responder: RpcResponder {
                                    protocol,
                                    response_sender: callback,
                                },
                            })
                        },
                        ConsensusMsg::CommitMessage(req) => {
                            IncomingRpcRequest::CommitRequest(IncomingCommitRequest {
                                req: *req,
                                protocol,
                                response_sender: callback,
                            })
                        },
                        ConsensusMsg::RandGenMessage(req) => {
                            IncomingRpcRequest::RandGenRequest(IncomingRandGenRequest {
                                req,
                                sender: peer_id,
                                protocol,
                                response_sender: callback,
                            })
                        },
                        _ => {
                            warn!(remote_peer = peer_id, "Unexpected msg: {:?}", msg);
                            continue;
                        },
                    };
                    if let Err(e) = self
                        .rpc_tx
                        .push((peer_id, discriminant(&req)), (peer_id, req))
                    {
                        warn!(error = ?e, "aptos channel closed");
                    };
                },
                _ => {
                    // Ignore `NewPeer` and `LostPeer` events
                },
            });
        }
    }
}
