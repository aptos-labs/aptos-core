// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network::{NetworkReceivers, NetworkSender},
    network_interface::{ConsensusMsg, ConsensusNetworkClient},
    test_utils::{self, consensus_runtime, placeholder_ledger_info, timed_block_on},
};
use aptos_channels::{self, aptos_channel, message_queues::QueueStyle};
use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_consensus_types::{
    block::{block_test_utils::certificate_for_genesis, Block},
    common::Author,
    proposal_msg::ProposalMsg,
    sync_info::SyncInfo,
    vote::Vote,
    vote_data::VoteData,
    vote_msg::VoteMsg,
};
use aptos_infallible::{Mutex, RwLock};
use aptos_network::{
    application::storage::PeersAndMetadata,
    peer_manager::{ConnectionRequestSender, PeerManagerRequest, PeerManagerRequestSender},
    protocols::{
        network::{NewNetworkEvents, ReceivedMessage, RpcError, SerializedRequest},
        wire::{
            handshake::v1::ProtocolIdSet,
            messaging::v1::{DirectSendMsg, NetworkMessage, RpcRequest},
        },
    },
    ProtocolId,
};
use aptos_types::{block_info::BlockInfo, PeerId};
use futures::{channel::mpsc, SinkExt, StreamExt};
use std::{
    collections::{HashMap, HashSet},
    iter::FromIterator,
    sync::Arc,
    time::Duration,
};
use tokio::runtime::Handle;

/// `TwinId` is used by the NetworkPlayground to uniquely identify
/// nodes, even if they have the same `AccountAddress` (e.g. for Twins)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TwinId {
    /// Node's ID
    pub id: usize,
    /// Author (AccountAddress)
    pub author: Author,
}

/// `NetworkPlayground` mocks the network implementation and provides convenience
/// methods for testing. Test clients can use `wait_for_messages` or
/// `deliver_messages` to inspect the direct-send messages sent between peers.
/// They can also configure network messages to be dropped between specific peers.
///
/// Currently, RPC messages are delivered immediately and are not controlled by
/// `wait_for_messages` or `deliver_messages` for delivery. They are also not
/// currently dropped according to the `NetworkPlayground`'s drop config.
pub struct NetworkPlayground {
    /// Maps each Author to a Sender of their inbound network notifications.
    /// These events will usually be handled by the event loop spawned in
    /// `ConsensusNetworkImpl`.
    ///
    node_consensus_txs:
        Arc<Mutex<HashMap<TwinId, aptos_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>>>>,
    /// Nodes' outbound handlers forward their outbound non-rpc messages to this
    /// queue.
    outbound_msgs_tx: mpsc::Sender<(TwinId, PeerManagerRequest)>,
    /// NetworkPlayground reads all nodes' outbound messages through this queue.
    outbound_msgs_rx: mpsc::Receiver<(TwinId, PeerManagerRequest)>,
    /// Allow test code to timeout RPC messages between peers.
    timeout_config: Arc<RwLock<TimeoutConfig>>,
    /// Allow test code to drop direct-send messages between peers.
    drop_config: Arc<RwLock<DropConfig>>,
    /// Allow test code to drop direct-send messages between peers per round.
    drop_config_round: DropConfigRound,
    /// An executor for spawning node outbound network event handlers
    executor: Handle,
    /// Maps authors to twins IDs
    /// An author may have multiple twin IDs for Twins
    author_to_twin_ids: Arc<RwLock<AuthorToTwinIds>>,
    /// Information about connections
    peers_and_metadata: Arc<PeersAndMetadata>,
}

impl NetworkPlayground {
    pub fn new(executor: Handle) -> Self {
        let (outbound_msgs_tx, outbound_msgs_rx) = mpsc::channel(1_024);

        NetworkPlayground {
            node_consensus_txs: Arc::new(Mutex::new(HashMap::new())),
            outbound_msgs_tx,
            outbound_msgs_rx,
            timeout_config: Arc::new(RwLock::new(TimeoutConfig::default())),
            drop_config: Arc::new(RwLock::new(DropConfig::default())),
            drop_config_round: DropConfigRound::default(),
            executor,
            author_to_twin_ids: Arc::new(RwLock::new(AuthorToTwinIds::default())),
            peers_and_metadata: PeersAndMetadata::new(&[NetworkId::Validator]),
        }
    }

    pub fn handle(&self) -> Handle {
        self.executor.clone()
    }

    /// HashMap of supported protocols to initialize ConsensusNetworkClient.
    pub fn peer_protocols(&self) -> Arc<PeersAndMetadata> {
        self.peers_and_metadata.clone()
    }

    /// Create a new async task that handles outbound messages sent by a node.
    ///
    /// All non-rpc messages are forwarded to the NetworkPlayground's
    /// `outbound_msgs_rx` queue, which controls delivery through the
    /// `deliver_messages` and `wait_for_messages` API's.
    ///
    /// Rpc messages are immediately sent to the destination for handling, so
    /// they don't block.
    async fn start_node_outbound_handler(
        timeout_config: Arc<RwLock<TimeoutConfig>>,
        drop_config: Arc<RwLock<DropConfig>>,
        src_twin_id: TwinId,
        mut network_reqs_rx: aptos_channel::Receiver<(PeerId, ProtocolId), PeerManagerRequest>,
        mut outbound_msgs_tx: mpsc::Sender<(TwinId, PeerManagerRequest)>,
        node_consensus_txs: Arc<
            Mutex<HashMap<TwinId, aptos_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>>>,
        >,
        author_to_twin_ids: Arc<RwLock<AuthorToTwinIds>>,
    ) {
        while let Some(net_req) = network_reqs_rx.next().await {
            match net_req {
                // Immediately forward rpc requests for handling. Unfortunately,
                // we can't handle rpc requests in `deliver_messages` due to
                // blocking issues, e.g., I want to write:
                // ```
                // let block = sender.request_block(peer_id, block_id).await.unwrap();
                // playground.wait_for_messages(1).await;
                // ```
                // but because the rpc call blocks and depends on the message
                // delivery, we'd have to spawn the sending behaviour on a
                // separate task, which is inconvenient.
                PeerManagerRequest::SendRpc(dst, outbound_req) => {
                    let dst_twin_ids = author_to_twin_ids.read().get_twin_ids(dst);

                    let dst_twin_id = match dst_twin_ids.iter().find(|dst_twin_id| {
                        !drop_config
                            .read()
                            .is_message_dropped(&src_twin_id, dst_twin_id)
                    }) {
                        Some(id) => id,
                        None => continue, // drop rpc
                    };

                    let (protocol_id, data, res_tx, _) = outbound_req.into_parts();
                    if timeout_config
                        .read()
                        .is_message_timedout(&src_twin_id, dst_twin_id)
                    {
                        res_tx.send(Err(RpcError::TimedOut)).unwrap();
                        continue;
                    }

                    let node_consensus_tx =
                        node_consensus_txs.lock().get(dst_twin_id).unwrap().clone();

                    node_consensus_tx
                        .push(
                            (src_twin_id.author, ProtocolId::ConsensusRpcBcs),
                            ReceivedMessage {
                                message: NetworkMessage::RpcRequest(RpcRequest {
                                    protocol_id,
                                    request_id: 123,
                                    priority: 0,
                                    raw_request: data.into(),
                                }),
                                sender: PeerNetworkId::new(
                                    NetworkId::Validator,
                                    src_twin_id.author,
                                ),
                                receive_timestamp_micros: 0,
                                rpc_replier: Some(Arc::new(res_tx)),
                            },
                        )
                        .unwrap();
                },
                // Other PeerManagerRequest get buffered for `deliver_messages` to
                // synchronously drain.
                net_req => {
                    let _ = outbound_msgs_tx.send((src_twin_id, net_req)).await;
                },
            }
        }
    }

    /// Add a new node to the NetworkPlayground.
    pub fn add_node(
        &mut self,
        twin_id: TwinId,
        consensus_tx: aptos_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>,
        network_reqs_rx: aptos_channel::Receiver<(PeerId, ProtocolId), PeerManagerRequest>,
        conn_mgr_reqs_rx: aptos_channels::Receiver<aptos_network::ConnectivityRequest>,
    ) {
        self.node_consensus_txs.lock().insert(twin_id, consensus_tx);
        self.drop_config.write().add_node(twin_id);
        self.timeout_config.write().add_node(twin_id);

        self.extend_author_to_twin_ids(twin_id.author, twin_id);

        let fut1 = NetworkPlayground::start_node_outbound_handler(
            Arc::clone(&self.timeout_config),
            Arc::clone(&self.drop_config),
            twin_id,
            network_reqs_rx,
            self.outbound_msgs_tx.clone(),
            self.node_consensus_txs.clone(),
            self.author_to_twin_ids.clone(),
        );
        let fut2 = conn_mgr_reqs_rx.map(Ok).forward(::futures::sink::drain());
        self.executor.spawn(futures::future::join(fut1, fut2));
    }

    /// Deliver a `PeerManagerRequest` from peer `src` to the destination peer.
    /// Returns a copy of the delivered message and the sending peer id, and
    /// whether the message was successfully delivered
    async fn deliver_message(
        &mut self,
        src_twin_id: TwinId,
        dst_twin_id: TwinId,
        rmsg: ReceivedMessage,
    ) -> (Author, ConsensusMsg) {
        let node_consensus_tx = self
            .node_consensus_txs
            .lock()
            .get(&dst_twin_id)
            .unwrap()
            .clone();

        // copy message data
        let source_address = rmsg.sender.peer_id();
        let consensus_msg = match &rmsg.message {
            NetworkMessage::DirectSendMsg(dmsg) => dmsg
                .protocol_id
                .from_bytes(dmsg.raw_msg.as_slice())
                .unwrap(),
            wrong_message => {
                panic!(
                    "[network playground] Unexpected ReceivedMessage: {:?}",
                    wrong_message
                );
            },
        };
        let _ = node_consensus_tx.push(
            (src_twin_id.author, ProtocolId::ConsensusDirectSendBcs),
            rmsg,
        );
        (source_address, consensus_msg)
    }

    /// Wait for exactly `num_messages` to be enqueued and delivered. Return a
    /// copy of all messages for verification.
    /// While all the sent messages are delivered (except those configured to be dropped),
    /// only the messages that satisfy the given msg inspector are counted.
    pub async fn wait_for_messages<F>(
        &mut self,
        num_messages: usize,
        msg_inspector: F,
    ) -> Vec<(Author, ConsensusMsg)>
    where
        F: Fn(&(Author, ConsensusMsg)) -> bool,
    {
        let mut msg_copies = vec![];
        while msg_copies.len() < num_messages {
            // Take the next queued message
            let (src_twin_id, net_req) = self.outbound_msgs_rx.next().await
                .expect("[network playground] waiting for messages, but message queue has shutdown unexpectedly");

            // Convert PeerManagerRequest to corresponding ReceivedMessage,
            // and extract destination peer
            let (dst, msg) = match &net_req {
                PeerManagerRequest::SendDirectSend(dst_inner, msg_inner) => {
                    (*dst_inner, msg_inner.clone())
                },
                msg_inner => panic!(
                    "[network playground] Unexpected PeerManagerRequest: {:?}",
                    msg_inner
                ),
            };

            let dst_twin_ids = self.get_twin_ids(dst);
            for (idx, dst_twin_id) in dst_twin_ids.iter().enumerate() {
                let consensus_msg = msg.to_message().unwrap();

                // Deliver and copy message if it's not dropped
                if !self.is_message_dropped(&src_twin_id, dst_twin_id, consensus_msg) {
                    let rmsg = ReceivedMessage {
                        message: NetworkMessage::DirectSendMsg(DirectSendMsg {
                            protocol_id: msg.protocol_id(),
                            priority: 0,
                            raw_msg: msg.data().clone().into(),
                        }),
                        sender: PeerNetworkId::new(NetworkId::Validator, src_twin_id.author),
                        receive_timestamp_micros: 0,
                        rpc_replier: None,
                    };
                    let msg_copy = self.deliver_message(src_twin_id, *dst_twin_id, rmsg).await;

                    // Only insert msg_copy once for twins (if delivered)
                    if idx == 0 && msg_inspector(&msg_copy) {
                        msg_copies.push(msg_copy);
                    }
                }
            }
        }
        assert_eq!(msg_copies.len(), num_messages);
        msg_copies
    }

    /// Return the round of a given message
    fn get_message_round(msg: ConsensusMsg) -> Option<u64> {
        match msg {
            ConsensusMsg::ProposalMsg(proposal_msg) => Some(proposal_msg.proposal().round()),
            ConsensusMsg::VoteMsg(vote_msg) => Some(vote_msg.vote().vote_data().proposed().round()),
            ConsensusMsg::SyncInfo(sync_info) => Some(sync_info.highest_certified_round()),
            ConsensusMsg::CommitVoteMsg(commit_vote) => Some(commit_vote.commit_info().round()),
            _ => None,
        }
    }

    /// Returns true for any message
    pub fn take_all(_msg_copy: &(Author, ConsensusMsg)) -> bool {
        true
    }

    /// Returns true for proposal messages only.
    pub fn proposals_only(msg: &(Author, ConsensusMsg)) -> bool {
        matches!(&msg.1, ConsensusMsg::ProposalMsg(_))
    }

    /// Returns true for vote messages only.
    pub fn votes_only(msg: &(Author, ConsensusMsg)) -> bool {
        matches!(&msg.1, ConsensusMsg::VoteMsg(_))
    }

    pub fn extend_author_to_twin_ids(&mut self, author: Author, twin_id: TwinId) {
        self.author_to_twin_ids
            .write()
            .extend_author_to_twin_ids(author, twin_id);
    }

    pub fn get_twin_ids(&self, author: Author) -> Vec<TwinId> {
        self.author_to_twin_ids.read().get_twin_ids(author)
    }

    fn is_message_dropped(&self, src: &TwinId, dst: &TwinId, msg: ConsensusMsg) -> bool {
        self.drop_config.read().is_message_dropped(src, dst)
            || Self::get_message_round(msg).map_or(false, |r| {
                self.drop_config_round.is_message_dropped(src, dst, r)
            })
    }

    pub fn split_network(
        &self,
        partition_first: Vec<TwinId>,
        partition_second: Vec<TwinId>,
    ) -> bool {
        self.drop_config
            .write()
            .split_network(&partition_first, &partition_second)
    }

    /// Check if the message from 'src_twin_id' to 'dst_twin_id' should be dropped in the given round
    pub fn is_message_dropped_round(&self, src: &TwinId, dst: &TwinId, round: u64) -> bool {
        self.drop_config_round.is_message_dropped(src, dst, round)
    }

    /// Creates the given per round network partitions
    pub fn split_network_round(
        &mut self,
        round_partitions: &HashMap<u64, Vec<Vec<TwinId>>>,
    ) -> bool {
        let mut ret = true;

        for (round, partitions) in round_partitions.iter() {
            partitions.iter().enumerate().for_each(|(i, p1)| {
                partitions.iter().skip(i + 1).for_each(|p2| {
                    ret &= self
                        .drop_config_round
                        .drop_message_for_round(*round, p1, p2)
                })
            })
        }
        ret
    }

    pub fn timeout_config(&self) -> Arc<RwLock<TimeoutConfig>> {
        self.timeout_config.clone()
    }

    pub async fn start(mut self) {
        // Take the next queued message
        while let Some((src_twin_id, net_req)) = self.outbound_msgs_rx.next().await {
            // Convert PeerManagerRequest to corresponding ReceivedMessage,
            // and extract destination peer
            let (dst, msg) = match &net_req {
                PeerManagerRequest::SendDirectSend(dst_inner, msg_inner) => {
                    (*dst_inner, msg_inner.clone())
                },
                msg_inner => panic!(
                    "[network playground] Unexpected PeerManagerRequest: {:?}",
                    msg_inner
                ),
            };

            let dst_twin_ids = self.get_twin_ids(dst);

            for dst_twin_id in dst_twin_ids.iter() {
                let rmsg = ReceivedMessage {
                    message: NetworkMessage::DirectSendMsg(DirectSendMsg {
                        protocol_id: msg.protocol_id(),
                        priority: 0,
                        raw_msg: msg.data().clone().into(),
                    }),
                    sender: PeerNetworkId::new(NetworkId::Validator, src_twin_id.author),
                    receive_timestamp_micros: 0,
                    rpc_replier: None,
                };
                let consensus_msg = msg.to_message().unwrap();

                // Deliver and copy message it if it's not dropped
                if !self.is_message_dropped(&src_twin_id, dst_twin_id, consensus_msg) {
                    self.deliver_message(src_twin_id, *dst_twin_id, rmsg).await;
                }
            }
        }
    }
}

#[derive(Default)]
struct AuthorToTwinIds(HashMap<Author, Vec<TwinId>>);

impl AuthorToTwinIds {
    pub fn extend_author_to_twin_ids(&mut self, author: Author, twin_id: TwinId) {
        self.0.entry(author).or_default();

        self.0.get_mut(&author).unwrap().push(twin_id)
    }

    pub fn get_twin_ids(&self, author: Author) -> Vec<TwinId> {
        self.0.get(&author).unwrap().clone()
    }
}

#[derive(Default)]
struct DropConfig(HashMap<TwinId, HashSet<TwinId>>);

impl DropConfig {
    pub fn is_message_dropped(&self, src: &TwinId, dst: &TwinId) -> bool {
        self.0.get(src).map_or(false, |set| set.contains(dst))
    }

    pub fn drop_message_for(&mut self, src: &TwinId, dst: &TwinId) -> bool {
        self.0.entry(*src).or_default().insert(*dst)
    }

    pub fn split_network(
        &mut self,
        partition_first: &[TwinId],
        partition_second: &[TwinId],
    ) -> bool {
        partition_first
            .iter()
            .flat_map(move |n1| partition_second.iter().map(move |n2| (n1, n2)))
            .fold(true, |mut done, (n1, n2)| {
                done &= self.drop_message_for(n1, n2);
                done &= self.drop_message_for(n2, n1);
                done
            })
    }

    fn add_node(&mut self, src: TwinId) {
        self.0.insert(src, HashSet::new());
    }
}

#[derive(Default)]
pub(crate) struct TimeoutConfig(HashMap<TwinId, HashSet<TwinId>>);

impl TimeoutConfig {
    pub fn is_message_timedout(&self, src: &TwinId, dst: &TwinId) -> bool {
        self.0.get(src).map_or(false, |set| set.contains(dst))
    }

    pub fn timeout_message_for(&mut self, src: &TwinId, dst: &TwinId) -> bool {
        self.0.entry(*src).or_default().insert(*dst)
    }

    fn add_node(&mut self, src: TwinId) {
        self.0.insert(src, HashSet::new());
    }
}

/// Table of per round message dropping rules
#[derive(Default)]
struct DropConfigRound(HashMap<u64, DropConfig>);

impl DropConfigRound {
    /// Check if the message from 'src' to 'dst' should be dropped in the given round
    fn is_message_dropped(&self, src: &TwinId, dst: &TwinId, round: u64) -> bool {
        self.0
            .get(&round)
            .map_or(false, |config| config.is_message_dropped(src, dst))
    }

    /// Create partition for the round
    fn drop_message_for_round(
        &mut self,
        round: u64,
        partition_first: &[TwinId],
        partition_second: &[TwinId],
    ) -> bool {
        let config = self.0.entry(round).or_default();
        config.split_network(partition_first, partition_second)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        network::{IncomingRpcRequest, NetworkTask},
        network_interface::{DIRECT_SEND, RPC},
    };
    use aptos_config::network_id::{NetworkId, PeerNetworkId};
    use aptos_consensus_types::{
        block_retrieval::{BlockRetrievalRequest, BlockRetrievalResponse, BlockRetrievalStatus},
        common::Payload,
    };
    use aptos_crypto::HashValue;
    use aptos_network::{
        application::{
            interface::{NetworkClient, NetworkServiceEvents},
            storage::PeersAndMetadata,
        },
        protocols::{
            network,
            network::{NetworkEvents, NewNetworkSender},
        },
        transport::ConnectionMetadata,
    };
    use aptos_types::validator_verifier::random_validator_verifier;
    use bytes::Bytes;
    use futures::{channel::oneshot, future};
    use maplit::hashmap;

    #[test]
    fn test_split_network_round() {
        let runtime = consensus_runtime();
        let mut playground = NetworkPlayground::new(runtime.handle().clone());

        let num_nodes = 5;
        let (signers, _validator_verifier) = random_validator_verifier(num_nodes, None, false);

        let mut nodes = Vec::new();
        for (i, signer) in signers.iter().enumerate() {
            nodes.push(TwinId {
                id: i,
                author: signer.author(),
            });
        }

        // Create per round partitions
        let mut round_partitions: HashMap<u64, Vec<Vec<TwinId>>> = HashMap::new();
        // Round 1 partitions: [0], [1,2]
        round_partitions.insert(1, vec![vec![nodes[0]], vec![nodes[1], nodes[2]]]);
        // Round 2 partitions: [1], [2], [3,4]
        round_partitions.insert(2, vec![vec![nodes[1]], vec![nodes[2]], vec![
            nodes[3], nodes[4],
        ]]);
        assert!(playground.split_network_round(&round_partitions));

        // Round 1 checks (partitions: [0], [1,2])
        // Messages from 0 to 1 should be dropped
        assert!(playground.is_message_dropped_round(&nodes[0], &nodes[1], 1));
        // Messages from 1 to 0 should also be dropped
        assert!(playground.is_message_dropped_round(&nodes[1], &nodes[0], 1));
        // Messages from 1 to 2 should not be dropped
        assert!(!playground.is_message_dropped_round(&nodes[1], &nodes[2], 1));
        // Messages from 3 to 1 should not be dropped
        assert!(!playground.is_message_dropped_round(&nodes[3], &nodes[0], 1));

        // Round 2 checks (partitions: [1], [2], [3,4])
        // Messages from 2 to 4 should be dropped
        assert!(playground.is_message_dropped_round(&nodes[2], &nodes[4], 2));
        // Messages from 1 to 2 should be dropped
        assert!(playground.is_message_dropped_round(&nodes[1], &nodes[2], 2));
        // Messages from 3 to 4 should not be dropped
        assert!(!playground.is_message_dropped_round(&nodes[3], &nodes[4], 2));
        // Messages from 0 to 3 should not be dropped
        assert!(!playground.is_message_dropped_round(&nodes[0], &nodes[3], 2));
    }

    fn add_peer_to_storage(
        peers_and_metadata: &PeersAndMetadata,
        peer: &PeerId,
        protocols: &[ProtocolId],
    ) {
        let peer_network_id = PeerNetworkId::new(NetworkId::Validator, *peer);
        let mut conn_meta = ConnectionMetadata::mock(*peer);
        conn_meta.application_protocols = ProtocolIdSet::from_iter(protocols);
        peers_and_metadata
            .insert_connection_metadata(peer_network_id, conn_meta)
            .unwrap();
    }

    #[test]
    fn test_network_api() {
        let runtime = consensus_runtime();
        let _entered_runtime = runtime.enter();

        let num_nodes = 5;
        let mut receivers: Vec<NetworkReceivers> = Vec::new();
        let mut playground = NetworkPlayground::new(runtime.handle().clone());
        let mut nodes = Vec::new();
        let (signers, validator_verifier) = random_validator_verifier(num_nodes, None, false);
        let validator_verifier = Arc::new(validator_verifier);
        let peers: Vec<_> = signers.iter().map(|signer| signer.author()).collect();
        let peers_and_metadata = PeersAndMetadata::new(&[NetworkId::Validator]);

        for (peer_id, peer) in peers.iter().enumerate() {
            let (network_reqs_tx, network_reqs_rx) = aptos_channel::new(QueueStyle::FIFO, 8, None);
            let (connection_reqs_tx, _) = aptos_channel::new(QueueStyle::FIFO, 8, None);
            let (consensus_tx, consensus_rx) = aptos_channel::new(QueueStyle::FIFO, 8, None);
            let (_conn_mgr_reqs_tx, conn_mgr_reqs_rx) = aptos_channels::new_test(1024);

            add_peer_to_storage(&peers_and_metadata, peer, &[
                ProtocolId::ConsensusDirectSendJson,
                ProtocolId::ConsensusDirectSendBcs,
                ProtocolId::ConsensusRpcBcs,
            ]);

            let network_sender = network::NetworkSender::new(
                PeerManagerRequestSender::new(network_reqs_tx),
                ConnectionRequestSender::new(connection_reqs_tx),
            );
            let network_client = NetworkClient::new(
                DIRECT_SEND.into(),
                RPC.into(),
                hashmap! {NetworkId::Validator => network_sender},
                peers_and_metadata.clone(),
            );
            let consensus_network_client = ConsensusNetworkClient::new(network_client);

            let twin_id = TwinId {
                id: peer_id,
                author: *peer,
            };
            playground.add_node(twin_id, consensus_tx, network_reqs_rx, conn_mgr_reqs_rx);

            let (self_sender, self_receiver) = aptos_channels::new_unbounded_test();
            let node = NetworkSender::new(
                *peer,
                consensus_network_client,
                self_sender,
                validator_verifier.clone(),
            );

            let network_events = NetworkEvents::new(consensus_rx, None, true);
            let network_service_events =
                NetworkServiceEvents::new(hashmap! {NetworkId::Validator => network_events});
            let (task, receiver) = NetworkTask::new(network_service_events, self_receiver);

            receivers.push(receiver);
            runtime.handle().spawn(task.start());
            nodes.push(node);
        }
        let vote_msg = VoteMsg::new(
            Vote::new(
                VoteData::new(BlockInfo::random(1), BlockInfo::random(0)),
                peers[0],
                placeholder_ledger_info(),
                &signers[0],
            )
            .unwrap(),
            test_utils::placeholder_sync_info(),
        );
        let previous_qc = certificate_for_genesis();
        let proposal = ProposalMsg::new(
            Block::new_proposal(
                Payload::empty(false, true),
                1,
                1,
                previous_qc.clone(),
                &signers[0],
                Vec::new(),
            )
            .unwrap(),
            SyncInfo::new(
                previous_qc.clone(),
                previous_qc.into_wrapped_ledger_info(),
                None,
            ),
        );
        timed_block_on(&runtime, async {
            nodes[0]
                .send_vote(vote_msg.clone(), peers[2..5].to_vec())
                .await;
            playground
                .wait_for_messages(3, NetworkPlayground::take_all)
                .await;
            for r in receivers.iter_mut().take(5).skip(2) {
                let (_, msg) = r.consensus_messages.next().await.unwrap();
                match msg {
                    ConsensusMsg::VoteMsg(v) => assert_eq!(*v, vote_msg),
                    _ => panic!("unexpected messages"),
                }
            }
            nodes[0].broadcast_proposal(proposal.clone()).await;
            playground
                .wait_for_messages(4, NetworkPlayground::take_all)
                .await;
            for r in receivers.iter_mut().take(num_nodes - 1) {
                let (_, msg) = r.consensus_messages.next().await.unwrap();
                match msg {
                    ConsensusMsg::ProposalMsg(p) => assert_eq!(*p, proposal),
                    _ => panic!("unexpected messages"),
                }
            }
        });
    }

    #[test]
    fn test_rpc() {
        let runtime = consensus_runtime();
        let _entered_runtime = runtime.enter();

        let num_nodes = 2;
        let mut senders = Vec::new();
        let mut receivers: Vec<NetworkReceivers> = Vec::new();
        let mut playground = NetworkPlayground::new(runtime.handle().clone());
        let mut nodes = Vec::new();
        let (signers, validator_verifier) = random_validator_verifier(num_nodes, None, false);
        let validator_verifier = Arc::new(validator_verifier);
        let peers: Vec<_> = signers.iter().map(|signer| signer.author()).collect();
        let peers_and_metadata = PeersAndMetadata::new(&[NetworkId::Validator]);

        for (peer_id, peer) in peers.iter().enumerate() {
            let (network_reqs_tx, network_reqs_rx) = aptos_channel::new(QueueStyle::FIFO, 8, None);
            let (connection_reqs_tx, _) = aptos_channel::new(QueueStyle::FIFO, 8, None);
            let (consensus_tx, consensus_rx) = aptos_channel::new(QueueStyle::FIFO, 8, None);
            let (_conn_mgr_reqs_tx, conn_mgr_reqs_rx) = aptos_channels::new_test(1024);
            let network_sender = network::NetworkSender::new(
                PeerManagerRequestSender::new(network_reqs_tx),
                ConnectionRequestSender::new(connection_reqs_tx),
            );

            let network_client = NetworkClient::new(
                DIRECT_SEND.into(),
                RPC.into(),
                hashmap! {NetworkId::Validator => network_sender},
                peers_and_metadata.clone(),
            );
            let consensus_network_client = ConsensusNetworkClient::new(network_client);

            add_peer_to_storage(&peers_and_metadata, peer, &[
                ProtocolId::ConsensusDirectSendJson,
                ProtocolId::ConsensusDirectSendBcs,
                ProtocolId::ConsensusRpcJson,
            ]);

            let twin_id = TwinId {
                id: peer_id,
                author: *peer,
            };
            playground.add_node(twin_id, consensus_tx, network_reqs_rx, conn_mgr_reqs_rx);

            let (self_sender, self_receiver) = aptos_channels::new_unbounded_test();
            let node = NetworkSender::new(
                *peer,
                consensus_network_client.clone(),
                self_sender,
                validator_verifier.clone(),
            );

            let network_events = NetworkEvents::new(consensus_rx, None, true);
            let network_service_events =
                NetworkServiceEvents::new(hashmap! {NetworkId::Validator => network_events});
            let (task, receiver) = NetworkTask::new(network_service_events, self_receiver);

            senders.push(consensus_network_client);
            receivers.push(receiver);
            runtime.handle().spawn(task.start());
            nodes.push(node);
        }
        let receiver_1 = receivers.remove(1);
        let node0 = nodes[0].clone();
        let peer1 = peers[1];
        let vote_msg = VoteMsg::new(
            Vote::new(
                VoteData::new(BlockInfo::random(1), BlockInfo::random(0)),
                peers[0],
                placeholder_ledger_info(),
                &signers[0],
            )
            .unwrap(),
            test_utils::placeholder_sync_info(),
        );

        // verify request block rpc
        let mut rpc_rx = receiver_1.rpc_rx;
        let on_request_block = async move {
            while let Some((_, request)) = rpc_rx.next().await {
                // make sure the network task is not blocked during RPC
                // we limit the network notification queue size to 1 so if it's blocked,
                // we can not process 2 votes and the test will timeout
                node0.send_vote(vote_msg.clone(), vec![peer1]).await;
                node0.send_vote(vote_msg.clone(), vec![peer1]).await;
                playground
                    .wait_for_messages(2, NetworkPlayground::votes_only)
                    .await;
                let response =
                    BlockRetrievalResponse::new(BlockRetrievalStatus::IdNotFound, vec![]);
                let response = ConsensusMsg::BlockRetrievalResponse(Box::new(response));
                let bytes = Bytes::from(serde_json::to_vec(&response).unwrap());
                match request {
                    IncomingRpcRequest::BlockRetrieval(request) => {
                        request.response_sender.send(Ok(bytes)).unwrap()
                    },
                    _ => panic!("unexpected message"),
                }
            }
        };
        runtime.handle().spawn(on_request_block);
        let peer = peers[1];
        timed_block_on(&runtime, async {
            let response = nodes[0]
                .request_block(
                    BlockRetrievalRequest::new(HashValue::zero(), 1),
                    peer,
                    Duration::from_secs(5),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), BlockRetrievalStatus::IdNotFound);
        });
    }

    #[test]
    fn test_bad_message() {
        let runtime = consensus_runtime();
        let _entered_runtime = runtime.enter();

        let (peer_mgr_notifs_tx, peer_mgr_notifs_rx) =
            aptos_channel::new(QueueStyle::FIFO, 8, None);
        let network_events = NetworkEvents::new(peer_mgr_notifs_rx, None, true);
        let network_service_events =
            NetworkServiceEvents::new(hashmap! {NetworkId::Validator => network_events});
        let (self_sender, self_receiver) = aptos_channels::new_unbounded_test();

        let (network_task, mut network_receivers) =
            NetworkTask::new(network_service_events, self_receiver);

        let peer_id = PeerId::random();
        let protocol_id = ProtocolId::ConsensusDirectSendBcs;
        let bad_msg = ReceivedMessage {
            message: NetworkMessage::DirectSendMsg(DirectSendMsg {
                protocol_id,
                priority: 0,
                raw_msg: Bytes::from_static(b"\xde\xad\xbe\xef").into(),
            }),
            sender: PeerNetworkId::new(NetworkId::Validator, peer_id),
            receive_timestamp_micros: 0,
            rpc_replier: None,
        };

        peer_mgr_notifs_tx
            .push((peer_id, protocol_id), bad_msg)
            .unwrap();

        let liveness_check_msg = ConsensusMsg::BlockRetrievalRequest(Box::new(
            BlockRetrievalRequest::new(HashValue::random(), 1),
        ));

        let protocol_id = ProtocolId::ConsensusRpcJson;
        let (res_tx, _res_rx) = oneshot::channel();
        let liveness_check_msg = ReceivedMessage {
            message: NetworkMessage::RpcRequest(RpcRequest {
                protocol_id,
                request_id: 0, // TODO: seq?
                priority: 0,
                raw_request: Bytes::from(serde_json::to_vec(&liveness_check_msg).unwrap()).into(),
            }),
            sender: PeerNetworkId::new(NetworkId::Validator, peer_id),
            receive_timestamp_micros: 0,
            rpc_replier: Some(Arc::new(res_tx)),
        };

        peer_mgr_notifs_tx
            .push((peer_id, protocol_id), liveness_check_msg)
            .unwrap();

        let f_check = async move {
            assert!(network_receivers.rpc_rx.next().await.is_some());

            drop(peer_mgr_notifs_tx);
            drop(self_sender);

            assert!(network_receivers.rpc_rx.next().await.is_none());
            assert!(network_receivers.consensus_messages.next().await.is_none());
        };
        let f_network_task = network_task.start();

        let runtime = consensus_runtime();
        timed_block_on(&runtime, future::join(f_network_task, f_check));
    }
}
