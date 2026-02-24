// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Interface between Consensus and Network layers.

use crate::{
    dag::DAGNetworkMessage,
    pipeline,
    quorum_store::types::{Batch, BatchMsg, BatchRequest, BatchResponse},
    rand::{
        rand_gen::network_messages::RandGenMessage,
        secret_sharing::network_messages::SecretShareNetworkMessage,
    },
};
use aptos_prefix_consensus::{PrefixConsensusMsg, SlotConsensusMsg, StrongPrefixConsensusMsg};
use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_consensus_types::{
    block_retrieval::{BlockRetrievalRequest, BlockRetrievalRequestV1, BlockRetrievalResponse},
    epoch_retrieval::EpochRetrievalRequest,
    opt_proposal_msg::OptProposalMsg,
    order_vote_msg::OrderVoteMsg,
    pipeline::{commit_decision::CommitDecision, commit_vote::CommitVote},
    proof_of_store::{BatchInfo, BatchInfoExt, ProofOfStoreMsg, SignedBatchInfoMsg},
    proposal_msg::ProposalMsg,
    round_timeout::RoundTimeoutMsg,
    sync_info::SyncInfo,
    vote_msg::VoteMsg,
};
use aptos_network::{
    application::{error::Error, interface::NetworkClientInterface, storage::PeersAndMetadata},
    peer::DisconnectReason,
    ProtocolId,
};
use aptos_types::network_address::NetworkAddress;
use aptos_types::{epoch_change::EpochChangeProof, PeerId};
use bytes::Bytes;
pub use pipeline::commit_reliable_broadcast::CommitMessage;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};

/// Network type for consensus
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ConsensusMsg {
    /// DEPRECATED: Once this is introduced in the next release, please use
    /// [`ConsensusMsg::BlockRetrievalRequest`](ConsensusMsg::BlockRetrievalRequest) going forward
    /// This variant was renamed from `BlockRetrievalRequest` to `DeprecatedBlockRetrievalRequest`
    /// RPC to get a chain of block of the given length starting from the given block id.
    DeprecatedBlockRetrievalRequest(Box<BlockRetrievalRequestV1>),
    /// Carries the returned blocks and the retrieval status.
    BlockRetrievalResponse(Box<BlockRetrievalResponse>),
    /// Request to get a EpochChangeProof from current_epoch to target_epoch
    EpochRetrievalRequest(Box<EpochRetrievalRequest>),
    /// ProposalMsg contains the required information for the proposer election protocol to make
    /// its choice (typically depends on round and proposer info).
    ProposalMsg(Box<ProposalMsg>),
    /// This struct describes basic synchronization metadata.
    SyncInfo(Box<SyncInfo>),
    /// A vector of LedgerInfo with contiguous increasing epoch numbers to prove a sequence of
    /// epoch changes from the first LedgerInfo's epoch.
    EpochChangeProof(Box<EpochChangeProof>),
    /// VoteMsg is the struct that is ultimately sent by the voter in response for receiving a
    /// proposal.
    VoteMsg(Box<VoteMsg>),
    /// CommitProposal is the struct that is sent by the validator after execution to propose
    /// on the committed state hash root.
    CommitVoteMsg(Box<CommitVote>),
    /// CommitDecision is the struct that is sent by the validator after collecting no fewer
    /// than 2f + 1 signatures on the commit proposal. This part is not on the critical path, but
    /// it can save slow machines to quickly confirm the execution result.
    CommitDecisionMsg(Box<CommitDecision>),
    /// Quorum Store: Send a Batch of transactions.
    BatchMsg(Box<BatchMsg<BatchInfo>>),
    /// Quorum Store: Request the payloads of a completed batch.
    BatchRequestMsg(Box<BatchRequest>),
    /// Quorum Store: Response to the batch request.
    BatchResponse(Box<Batch<BatchInfo>>),
    /// Quorum Store: Send a signed batch digest. This is a vote for the batch and a promise that
    /// the batch of transactions was received and will be persisted until batch expiration.
    SignedBatchInfo(Box<SignedBatchInfoMsg<BatchInfo>>),
    /// Quorum Store: Broadcast a certified proof of store (a digest that received 2f+1 votes).
    ProofOfStoreMsg(Box<ProofOfStoreMsg<BatchInfo>>),
    /// DAG protocol message
    DAGMessage(DAGNetworkMessage),
    /// Commit message
    CommitMessage(Box<CommitMessage>),
    /// Randomness generation message
    RandGenMessage(RandGenMessage),
    /// Quorum Store: Response to the batch request.
    BatchResponseV2(Box<BatchResponse>),
    /// OrderVoteMsg is the struct that is broadcasted by a validator on receiving quorum certificate
    /// on a block.
    OrderVoteMsg(Box<OrderVoteMsg>),
    /// RoundTimeoutMsg is broadcasted by a validator once it decides to timeout the current round.
    RoundTimeoutMsg(Box<RoundTimeoutMsg>),
    /// RPC to get a chain of block of the given length starting from the given block id, using epoch and round.
    BlockRetrievalRequest(Box<BlockRetrievalRequest>),
    /// OptProposalMsg contains the optimistic proposal and sync info.
    OptProposalMsg(Box<OptProposalMsg>),
    /// Quorum Store: Send a Batch of transactions.
    BatchMsgV2(Box<BatchMsg<BatchInfoExt>>),
    /// Quorum Store: Send a signed batch digest with BatchInfoExt. This is a vote for the batch and a promise that
    /// the batch of transactions was received and will be persisted until batch expiration.
    SignedBatchInfoMsgV2(Box<SignedBatchInfoMsg<BatchInfoExt>>),
    /// Quorum Store: Broadcast a certified proof of store (a digest that received 2f+1 votes) with BatchInfoExt.
    ProofOfStoreMsgV2(Box<ProofOfStoreMsg<BatchInfoExt>>),
    /// Secret share message: Used to share secrets per consensus round
    SecretShareMsg(SecretShareNetworkMessage),
    /// Prefix Consensus message: Vote1, Vote2, or Vote3 for prefix consensus protocol
    PrefixConsensusMsg(Box<PrefixConsensusMsg>),
    /// Strong Prefix Consensus message: multi-view protocol messages
    StrongPrefixConsensusMsg(Box<StrongPrefixConsensusMsg>),
    /// Slot Consensus message: slot proposals and per-slot SPC messages
    SlotConsensusMsg(Box<SlotConsensusMsg>),
}

/// Network type for consensus
impl ConsensusMsg {
    /// ConsensusMsg type in string
    /// TODO @bchocho @hariria can remove after all nodes upgrade to release with enum BlockRetrievalRequest (not struct)
    pub fn name(&self) -> &str {
        match self {
            ConsensusMsg::DeprecatedBlockRetrievalRequest(_) => "DeprecatedBlockRetrievalRequest",
            ConsensusMsg::BlockRetrievalResponse(_) => "BlockRetrievalResponse",
            ConsensusMsg::EpochRetrievalRequest(_) => "EpochRetrievalRequest",
            ConsensusMsg::ProposalMsg(_) => "ProposalMsg",
            ConsensusMsg::OptProposalMsg(_) => "OptProposalMsg",
            ConsensusMsg::SyncInfo(_) => "SyncInfo",
            ConsensusMsg::EpochChangeProof(_) => "EpochChangeProof",
            ConsensusMsg::VoteMsg(_) => "VoteMsg",
            ConsensusMsg::OrderVoteMsg(_) => "OrderVoteMsg",
            ConsensusMsg::CommitVoteMsg(_) => "CommitVoteMsg",
            ConsensusMsg::CommitDecisionMsg(_) => "CommitDecisionMsg",
            ConsensusMsg::BatchMsg(_) => "BatchMsg",
            ConsensusMsg::BatchRequestMsg(_) => "BatchRequestMsg",
            ConsensusMsg::BatchResponse(_) => "BatchResponse",
            ConsensusMsg::SignedBatchInfo(_) => "SignedBatchInfo",
            ConsensusMsg::ProofOfStoreMsg(_) => "ProofOfStoreMsg",
            ConsensusMsg::DAGMessage(_) => "DAGMessage",
            ConsensusMsg::CommitMessage(_) => "CommitMessage",
            ConsensusMsg::RandGenMessage(_) => "RandGenMessage",
            ConsensusMsg::BatchResponseV2(_) => "BatchResponseV2",
            ConsensusMsg::RoundTimeoutMsg(_) => "RoundTimeoutV2",
            ConsensusMsg::BlockRetrievalRequest(_) => "BlockRetrievalRequest",
            ConsensusMsg::BatchMsgV2(_) => "BatchMsgV2",
            ConsensusMsg::SignedBatchInfoMsgV2(_) => "SignedBatchInfoMsgV2",
            ConsensusMsg::ProofOfStoreMsgV2(_) => "ProofOfStoreMsgV2",
            ConsensusMsg::SecretShareMsg(_) => "SecretShareMsg",
            ConsensusMsg::PrefixConsensusMsg(_) => "PrefixConsensusMsg",
            ConsensusMsg::StrongPrefixConsensusMsg(_) => "StrongPrefixConsensusMsg",
            ConsensusMsg::SlotConsensusMsg(_) => "SlotConsensusMsg",
        }
    }
}

/// The interface from Consensus to Networking layer.
///
/// This is a thin wrapper around a `NetworkClient<ConsensusMsg>`, so it is easy
/// to clone and send off to a separate task. For example, the rpc requests
/// return Futures that encapsulate the whole flow, from sending the request to
/// remote, to finally receiving the response and deserializing. It therefore
/// makes the most sense to make the rpc call on a separate async task, which
/// requires the `ConsensusNetworkClient` to be `Clone` and `Send`.
#[derive(Clone)]
pub struct ConsensusNetworkClient<NetworkClient> {
    network_client: NetworkClient,
}

/// Supported protocols in preferred order (from highest priority to lowest).
pub const RPC: &[ProtocolId] = &[
    ProtocolId::ConsensusRpcCompressed,
    ProtocolId::ConsensusRpcBcs,
    ProtocolId::ConsensusRpcJson,
];

/// Supported protocols in preferred order (from highest priority to lowest).
pub const DIRECT_SEND: &[ProtocolId] = &[
    ProtocolId::ConsensusDirectSendCompressed,
    ProtocolId::ConsensusDirectSendBcs,
    ProtocolId::ConsensusDirectSendJson,
];

impl<NetworkClient: NetworkClientInterface<ConsensusMsg>> ConsensusNetworkClient<NetworkClient> {
    /// Returns a new consensus network client
    pub fn new(network_client: NetworkClient) -> Self {
        Self { network_client }
    }

    /// Send a single message to the destination peer
    pub fn send_to(&self, peer: PeerId, message: ConsensusMsg) -> Result<(), Error> {
        let peer_network_id = self.get_peer_network_id_for_peer(peer);
        self.network_client.send_to_peer(message, peer_network_id)
    }

    /// Send a single message to the destination peers
    pub fn send_to_many(&self, peers: Vec<PeerId>, message: ConsensusMsg) -> Result<(), Error> {
        let peer_network_ids: Vec<PeerNetworkId> = peers
            .into_iter()
            .map(|peer| self.get_peer_network_id_for_peer(peer))
            .collect();
        self.network_client.send_to_peers(message, peer_network_ids)
    }

    /// Send a RPC to the destination peer
    pub async fn send_rpc(
        &self,
        peer: PeerId,
        message: ConsensusMsg,
        rpc_timeout: Duration,
    ) -> Result<ConsensusMsg, Error> {
        let peer_network_id = self.get_peer_network_id_for_peer(peer);
        self.network_client
            .send_to_peer_rpc(message, rpc_timeout, peer_network_id)
            .await
    }

    pub async fn send_rpc_raw(
        &self,
        peer: PeerId,
        message: Bytes,
        rpc_timeout: Duration,
    ) -> Result<ConsensusMsg, Error> {
        let peer_network_id = self.get_peer_network_id_for_peer(peer);
        self.network_client
            .send_to_peer_rpc_raw(message, rpc_timeout, peer_network_id)
            .await
    }

    pub fn to_bytes_by_protocol(
        &self,
        peers: Vec<PeerId>,
        message: ConsensusMsg,
    ) -> anyhow::Result<HashMap<PeerId, Bytes>> {
        let peer_network_ids: Vec<PeerNetworkId> = peers
            .into_iter()
            .map(|peer| self.get_peer_network_id_for_peer(peer))
            .collect();
        Ok(self
            .network_client
            .to_bytes_by_protocol(peer_network_ids, message)?
            .into_iter()
            .map(|(peer_network_id, bytes)| (peer_network_id.peer_id(), bytes))
            .collect())
    }

    // TODO: we shouldn't need to expose this. Migrate the code to handle
    // peer and network ids.
    fn get_peer_network_id_for_peer(&self, peer: PeerId) -> PeerNetworkId {
        PeerNetworkId::new(NetworkId::Validator, peer)
    }

    pub fn sort_peers_by_latency(&self, peers: &mut [PeerId]) {
        self.network_client
            .sort_peers_by_latency(NetworkId::Validator, peers);
    }

    /// Get reference to the underlying network client
    /// This is used by the prefix consensus bridge adapters
    pub fn network_client(&self) -> &NetworkClient {
        &self.network_client
    }
}

// =============================================================================
// Prefix Consensus network bridge
//
// Generic bridge wraps sub-protocol messages (PrefixConsensusMsg,
// StrongPrefixConsensusMsg, SlotConsensusMsg) inside ConsensusMsg variants so
// they travel over the single consensus network channel.
// =============================================================================

/// Trait for sub-protocol message types that can be wrapped in `ConsensusMsg`.
pub trait ConsensusSubprotocolMsg: Clone + Send + Sync + Sized + Serialize + serde::de::DeserializeOwned {
    fn into_consensus_msg(self) -> ConsensusMsg;
    fn from_consensus_msg(msg: ConsensusMsg) -> Option<Self>;
}

impl ConsensusSubprotocolMsg for PrefixConsensusMsg {
    fn into_consensus_msg(self) -> ConsensusMsg {
        ConsensusMsg::PrefixConsensusMsg(Box::new(self))
    }

    fn from_consensus_msg(msg: ConsensusMsg) -> Option<Self> {
        match msg {
            ConsensusMsg::PrefixConsensusMsg(m) => Some(*m),
            _ => None,
        }
    }
}

impl ConsensusSubprotocolMsg for StrongPrefixConsensusMsg {
    fn into_consensus_msg(self) -> ConsensusMsg {
        ConsensusMsg::StrongPrefixConsensusMsg(Box::new(self))
    }

    fn from_consensus_msg(msg: ConsensusMsg) -> Option<Self> {
        match msg {
            ConsensusMsg::StrongPrefixConsensusMsg(m) => Some(*m),
            _ => None,
        }
    }
}

impl ConsensusSubprotocolMsg for SlotConsensusMsg {
    fn into_consensus_msg(self) -> ConsensusMsg {
        ConsensusMsg::SlotConsensusMsg(Box::new(self))
    }

    fn from_consensus_msg(msg: ConsensusMsg) -> Option<Self> {
        match msg {
            ConsensusMsg::SlotConsensusMsg(m) => Some(*m),
            _ => None,
        }
    }
}

/// Generic bridge adapter: wraps a sub-protocol message type `M` inside its
/// `ConsensusMsg` variant so it can be sent over the consensus network.
#[derive(Clone)]
pub(crate) struct SubprotocolNetworkBridge<NetworkClient> {
    inner: ConsensusNetworkClient<NetworkClient>,
}

impl<NetworkClient> SubprotocolNetworkBridge<NetworkClient> {
    pub fn new(inner: ConsensusNetworkClient<NetworkClient>) -> Self {
        Self { inner }
    }
}

#[async_trait::async_trait]
impl<M, NetworkClient> NetworkClientInterface<M>
    for SubprotocolNetworkBridge<NetworkClient>
where
    M: ConsensusSubprotocolMsg + 'static,
    NetworkClient: NetworkClientInterface<ConsensusMsg> + Send + Sync,
{
    async fn add_peers_to_discovery(
        &self,
        peers: &[(PeerNetworkId, NetworkAddress)],
    ) -> Result<(), Error> {
        self.inner.network_client().add_peers_to_discovery(peers).await
    }

    async fn disconnect_from_peer(
        &self,
        peer: PeerNetworkId,
        disconnect_reason: DisconnectReason,
    ) -> Result<(), Error> {
        self.inner
            .network_client()
            .disconnect_from_peer(peer, disconnect_reason)
            .await
    }

    fn get_available_peers(&self) -> Result<Vec<PeerNetworkId>, Error> {
        self.inner.network_client().get_available_peers()
    }

    fn get_peers_and_metadata(&self) -> Arc<PeersAndMetadata> {
        self.inner.network_client().get_peers_and_metadata()
    }

    fn send_to_peer(&self, message: M, peer: PeerNetworkId) -> Result<(), Error> {
        let wrapped = message.into_consensus_msg();
        self.inner.network_client().send_to_peer(wrapped, peer)
    }

    fn send_to_peer_raw(&self, message: Bytes, peer: PeerNetworkId) -> Result<(), Error> {
        self.inner.network_client().send_to_peer_raw(message, peer)
    }

    fn send_to_peers(&self, message: M, peers: Vec<PeerNetworkId>) -> Result<(), Error> {
        let wrapped = message.into_consensus_msg();
        self.inner.network_client().send_to_peers(wrapped, peers)
    }

    async fn send_to_peer_rpc(
        &self,
        message: M,
        rpc_timeout: Duration,
        peer: PeerNetworkId,
    ) -> Result<M, Error> {
        let wrapped = message.into_consensus_msg();
        let response = self
            .inner
            .network_client()
            .send_to_peer_rpc(wrapped, rpc_timeout, peer)
            .await?;
        M::from_consensus_msg(response).ok_or_else(|| {
            Error::RpcError("Unexpected message type in sub-protocol RPC response".to_string())
        })
    }

    async fn send_to_peer_rpc_raw(
        &self,
        _message: Bytes,
        _rpc_timeout: Duration,
        _peer: PeerNetworkId,
    ) -> Result<M, Error> {
        Err(Error::RpcError(
            "Raw RPC not supported for prefix consensus sub-protocols".to_string(),
        ))
    }

    fn to_bytes_by_protocol(
        &self,
        peers: Vec<PeerNetworkId>,
        message: M,
    ) -> anyhow::Result<HashMap<PeerNetworkId, Bytes>> {
        let wrapped = message.into_consensus_msg();
        self.inner
            .network_client()
            .to_bytes_by_protocol(peers, wrapped)
    }

    fn sort_peers_by_latency(&self, network: NetworkId, peers: &mut [PeerId]) {
        self.inner
            .network_client()
            .sort_peers_by_latency(network, peers)
    }
}

/// Type aliases for backward compatibility with existing callers.
pub(crate) type ConsensusNetworkBridge<NC> = SubprotocolNetworkBridge<NC>;
pub(crate) type StrongConsensusNetworkBridge<NC> = SubprotocolNetworkBridge<NC>;
#[allow(dead_code)]
pub(crate) type SlotConsensusNetworkBridge<NC> = SubprotocolNetworkBridge<NC>;
