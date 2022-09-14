// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::block_storage::tracing::{observe_block, BlockStage};
use crate::{
    counters,
    logging::LogEvent,
    monitor,
    network_interface::{ConsensusMsg, ConsensusNetworkEvents, ConsensusNetworkSender},
};
use anyhow::{anyhow, ensure};
use aptos_logger::prelude::*;
use aptos_types::{
    account_address::AccountAddress, epoch_change::EpochChangeProof,
    ledger_info::LedgerInfoWithSignatures, validator_verifier::ValidatorVerifier,
};
use bytes::Bytes;
use channel::{self, aptos_channel, message_queues::QueueStyle};
use consensus_types::{
    block_retrieval::{BlockRetrievalRequest, BlockRetrievalResponse, MAX_BLOCKS_PER_REQUEST},
    common::Author,
    experimental::{commit_decision::CommitDecision, commit_vote::CommitVote},
    proposal_msg::ProposalMsg,
    sync_info::SyncInfo,
    vote_msg::VoteMsg,
};
use fail::fail_point;
use futures::{channel::oneshot, stream::select, SinkExt, Stream, StreamExt};
use network::{
    protocols::{
        network::{ApplicationNetworkSender, Event},
        rpc::error::RpcError,
    },
    ProtocolId,
};
use std::{
    mem::{discriminant, Discriminant},
    time::Duration,
};

/// The block retrieval request is used internally for implementing RPC: the callback is executed
/// for carrying the response
#[derive(Debug)]
pub struct IncomingBlockRetrievalRequest {
    pub req: BlockRetrievalRequest,
    pub protocol: ProtocolId,
    pub response_sender: oneshot::Sender<Result<Bytes, RpcError>>,
}

/// Just a convenience struct to keep all the network proxy receiving queues in one place.
/// Will be returned by the NetworkTask upon startup.
pub struct NetworkReceivers {
    /// Provide a LIFO buffer for each (Author, MessageType) key
    pub consensus_messages: aptos_channel::Receiver<
        (AccountAddress, Discriminant<ConsensusMsg>),
        (AccountAddress, ConsensusMsg),
    >,
    pub block_retrieval:
        aptos_channel::Receiver<AccountAddress, (AccountAddress, IncomingBlockRetrievalRequest)>,
}

/// Implements the actual networking support for all consensus messaging.
#[derive(Clone)]
pub struct NetworkSender {
    author: Author,
    network_sender: ConsensusNetworkSender,
    // Self sender and self receivers provide a shortcut for sending the messages to itself.
    // (self sending is not supported by the networking API).
    // Note that we do not support self rpc requests as it might cause infinite recursive calls.
    self_sender: channel::Sender<Event<ConsensusMsg>>,
    validators: ValidatorVerifier,
}

impl NetworkSender {
    pub fn new(
        author: Author,
        network_sender: ConsensusNetworkSender,
        self_sender: channel::Sender<Event<ConsensusMsg>>,
        validators: ValidatorVerifier,
    ) -> Self {
        NetworkSender {
            author,
            network_sender,
            self_sender,
            validators,
        }
    }

    /// Tries to retrieve num of blocks backwards starting from id from the given peer: the function
    /// returns a future that is fulfilled with BlockRetrievalResponse.
    pub async fn request_block(
        &mut self,
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
            self.network_sender.send_rpc(from, msg, timeout).await
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

    /// Tries to send the given msg to all the participants.
    ///
    /// The future is fulfilled as soon as the message put into the mpsc channel to network
    /// internal(to provide back pressure), it does not indicate the message is delivered or sent
    /// out. It does not give indication about when the message is delivered to the recipients,
    /// as well as there is no indication about the network failures.
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
            .network_sender
            .send_to_many(other_validators.into_iter(), msg)
        {
            error!(error = ?err, "Error broadcasting message");
        }
    }

    /// Tries to send msg to given recipients.
    async fn send(&self, msg: ConsensusMsg, recipients: Vec<Author>) {
        fail_point!("consensus::send::any", |_| ());
        let network_sender = self.network_sender.clone();
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
                error!(
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

    pub async fn broadcast_commit_vote(&mut self, commit_vote: CommitVote) {
        fail_point!("consensus::send::broadcast_commit_vote", |_| ());
        let msg = ConsensusMsg::CommitVoteMsg(Box::new(commit_vote));
        self.broadcast(msg).await
    }

    pub async fn send_commit_vote(&mut self, commit_vote: CommitVote, recipient: Author) {
        fail_point!("consensus::send::commit_vote", |_| ());
        let msg = ConsensusMsg::CommitVoteMsg(Box::new(commit_vote));
        self.send(msg, vec![recipient]).await
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

    pub async fn send_epoch_change(&mut self, proof: EpochChangeProof) {
        fail_point!("consensus::send::epoch_change", |_| ());
        let msg = ConsensusMsg::EpochChangeProof(Box::new(proof));
        self.send(msg, vec![self.author]).await
    }

    /// Sends the ledger info to self buffer manager
    pub async fn send_commit_proof(&self, ledger_info: LedgerInfoWithSignatures) {
        fail_point!("consensus::send::commit_proof", |_| ());

        // this requires re-verification of the ledger info we can probably optimize it later
        let msg = ConsensusMsg::CommitDecisionMsg(Box::new(CommitDecision::new(ledger_info)));
        self.send(msg, vec![self.author]).await
    }

    pub fn author(&self) -> Author {
        self.author
    }

    pub async fn broadcast_commit_proof(&mut self, ledger_info: LedgerInfoWithSignatures) {
        fail_point!("consensus::send::broadcast_commit_proof", |_| ());
        let msg = ConsensusMsg::CommitDecisionMsg(Box::new(CommitDecision::new(ledger_info)));
        self.broadcast(msg).await
    }
}

pub struct NetworkTask {
    consensus_messages_tx: aptos_channel::Sender<
        (AccountAddress, Discriminant<ConsensusMsg>),
        (AccountAddress, ConsensusMsg),
    >,
    block_retrieval_tx:
        aptos_channel::Sender<AccountAddress, (AccountAddress, IncomingBlockRetrievalRequest)>,
    all_events: Box<dyn Stream<Item = Event<ConsensusMsg>> + Send + Unpin>,
}

impl NetworkTask {
    /// Establishes the initial connections with the peers and returns the receivers.
    pub fn new(
        network_events: ConsensusNetworkEvents,
        self_receiver: channel::Receiver<Event<ConsensusMsg>>,
    ) -> (NetworkTask, NetworkReceivers) {
        let (consensus_messages_tx, consensus_messages) =
            aptos_channel::new(QueueStyle::LIFO, 1, Some(&counters::CONSENSUS_CHANNEL_MSGS));
        let (block_retrieval_tx, block_retrieval) = aptos_channel::new(
            QueueStyle::LIFO,
            1,
            Some(&counters::BLOCK_RETRIEVAL_CHANNEL_MSGS),
        );
        let all_events = Box::new(select(network_events, self_receiver));
        (
            NetworkTask {
                consensus_messages_tx,
                block_retrieval_tx,
                all_events,
            },
            NetworkReceivers {
                consensus_messages,
                block_retrieval,
            },
        )
    }

    pub async fn start(mut self) {
        while let Some(message) = self.all_events.next().await {
            match message {
                Event::Message(peer_id, msg) => {
                    counters::CONSENSUS_RECEIVED_MSGS
                        .with_label_values(&[msg.name()])
                        .inc();
                    if let ConsensusMsg::ProposalMsg(proposal) = &msg {
                        observe_block(
                            proposal.proposal().timestamp_usecs(),
                            BlockStage::NETWORK_RECEIVED,
                        );
                    }
                    if let Err(e) = self
                        .consensus_messages_tx
                        .push((peer_id, discriminant(&msg)), (peer_id, msg))
                    {
                        warn!(
                            remote_peer = peer_id,
                            error = ?e, "Error pushing consensus msg",
                        );
                    }
                }
                Event::RpcRequest(peer_id, msg, protocol, callback) => match msg {
                    ConsensusMsg::BlockRetrievalRequest(request) => {
                        counters::CONSENSUS_RECEIVED_MSGS
                            .with_label_values(&["BlockRetrievalRequest"])
                            .inc();
                        debug!(
                            remote_peer = peer_id,
                            event = LogEvent::ReceiveBlockRetrieval,
                            "{}",
                            request
                        );
                        if request.num_blocks() > MAX_BLOCKS_PER_REQUEST {
                            warn!(
                                remote_peer = peer_id,
                                "Ignore block retrieval with too many blocks: {}",
                                request.num_blocks()
                            );
                            continue;
                        }
                        let req_with_callback = IncomingBlockRetrievalRequest {
                            req: *request,
                            protocol,
                            response_sender: callback,
                        };
                        if let Err(e) = self
                            .block_retrieval_tx
                            .push(peer_id, (peer_id, req_with_callback))
                        {
                            warn!(error = ?e, "aptos channel closed");
                        }
                    }
                    _ => {
                        warn!(remote_peer = peer_id, "Unexpected msg: {:?}", msg);
                        continue;
                    }
                },
                _ => {
                    // Ignore `NewPeer` and `LostPeer` events
                }
            }
        }
    }
}
