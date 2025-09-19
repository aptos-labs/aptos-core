// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use aptos_consensus_types::{
    block_retrieval::{BlockRetrievalRequest, BlockRetrievalResponse},
    opt_proposal_msg::OptProposalMsg,
    order_vote_msg::OrderVoteMsg,
    proposal_msg::ProposalMsg,
    round_timeout::RoundTimeoutMsg,
    sync_info::SyncInfo,
    vote_msg::VoteMsg,
};
use aptos_crypto::HashValue;
use crate::network_interface::ConsensusMsg;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProxyConsensusMsg {
    pub proxy_consensus_message: ProxyConsensusMessage,
    pub consensus_id: ConsensusId,
}

impl ProxyConsensusMsg {
    pub fn into_network_message(self) -> ConsensusMsg {
        ConsensusMsg::ProxyConsensusMsg(Box::new(self))
    }

    pub fn name(&self) -> &str {
        match self.proxy_consensus_message {
            ProxyConsensusMessage::BlockRetrievalResponse(_) => "BlockRetrievalResponse",
            ProxyConsensusMessage::ProposalMsg(_) => "ProposalMsg",
            ProxyConsensusMessage::SyncInfo(_) => "SyncInfo",
            ProxyConsensusMessage::VoteMsg(_) => "VoteMsg",
            ProxyConsensusMessage::OrderVoteMsg(_) => "OrderVoteMsg",
            ProxyConsensusMessage::RoundTimeoutMsg(_) => "RoundTimeoutMsg",
            ProxyConsensusMessage::BlockRetrievalRequest(_) => "BlockRetrievalRequest",
            ProxyConsensusMessage::OptProposalMsg(_) => "OptProposalMsg",
        }
    }

    pub fn consensus_id(&self) -> ConsensusId {
        self.consensus_id.clone()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum ConsensusId {
    Primary,
    Proxy(HashValue),
}

/// Network type for proxy consensus message
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ProxyConsensusMessage {
    /// RPC to get a chain of block of the given length starting from the given block id, using epoch and round.
    BlockRetrievalRequest(BlockRetrievalRequest),
    /// Carries the returned blocks and the retrieval status.
    BlockRetrievalResponse(BlockRetrievalResponse),
    /// ProposalMsg contains the required information for the proposer election protocol to make
    /// its choice (typically depends on round and proposer info).
    ProposalMsg(ProposalMsg),
    /// OptProposalMsg contains the optimistic proposal and sync info.
    OptProposalMsg(OptProposalMsg),
    /// VoteMsg is the struct that is ultimately sent by the voter in response for receiving a
    /// proposal.
    VoteMsg(VoteMsg),
    /// OrderVoteMsg is the struct that is broadcasted by a validator on receiving quorum certificate
    /// on a block.
    OrderVoteMsg(OrderVoteMsg),
    /// RoundTimeoutMsg is broadcasted by a validator once it decides to timeout the current round.
    RoundTimeoutMsg(RoundTimeoutMsg),
    /// This struct describes basic synchronization metadata.
    SyncInfo(SyncInfo),
}
