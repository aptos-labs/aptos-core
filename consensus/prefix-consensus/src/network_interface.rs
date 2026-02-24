// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Network interface for Prefix Consensus
//!
//! This module provides the abstraction layer for sending Prefix Consensus messages
//! over the Aptos network layer. It handles broadcasting votes to validators and
//! self-message delivery through channels.

use crate::{
    network_messages::{PrefixConsensusMsg, StrongPrefixConsensusMsg},
    slot_types::SlotConsensusMsg,
    types::{Vote1, Vote2, Vote3},
};
use aptos_channels::UnboundedSender;
use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_consensus_types::common::Author;
use aptos_logger::prelude::*;
use aptos_network::application::{error::Error, interface::NetworkClientInterface};
use aptos_types::{validator_verifier::ValidatorVerifier, PeerId};
use futures::SinkExt;
use serde::{Serialize, de::DeserializeOwned};
use std::{marker::PhantomData, sync::Arc};

// =============================================================================
// Sender traits
// =============================================================================

/// Trait for sending Prefix Consensus messages over the network.
///
/// Kept separate from `SubprotocolNetworkSender` because the basic PC protocol
/// has three specific vote methods rather than a generic send/broadcast.
#[async_trait::async_trait]
pub trait PrefixConsensusNetworkSender: Send + Sync + Clone {
    /// Broadcast a Vote1 message to all validators
    async fn broadcast_vote1(&self, vote: Vote1);

    /// Broadcast a Vote2 message to all validators
    async fn broadcast_vote2(&self, vote: Vote2);

    /// Broadcast a Vote3 message to all validators
    async fn broadcast_vote3(&self, vote: Vote3);
}

/// Generic trait for sending sub-protocol messages (Strong PC, Slot Consensus).
///
/// Both Strong PC and Slot Consensus have the same two operations: broadcast to
/// all validators and send to a specific peer. This trait unifies them.
#[async_trait::async_trait]
pub trait SubprotocolNetworkSender<M: Send + Sync + Clone>: Send + Sync + Clone {
    /// Broadcast a message to all validators
    async fn broadcast(&self, msg: M);

    /// Send a message to a specific peer
    async fn send_to(&self, peer: Author, msg: M);
}

// =============================================================================
// Generic network client
// =============================================================================

/// Generic network client wrapper for prefix consensus sub-protocols.
///
/// Wraps the NetworkClient to provide sub-protocol-specific sending methods.
/// Parameterized over the message type `M` and the underlying network client.
pub struct SubprotocolNetworkClient<M, NetworkClient> {
    network_client: NetworkClient,
    _phantom: PhantomData<fn() -> M>,
}

// Manual Clone impl to avoid unnecessary M: Clone bound from derive
impl<M, NetworkClient: Clone> Clone for SubprotocolNetworkClient<M, NetworkClient> {
    fn clone(&self) -> Self {
        Self {
            network_client: self.network_client.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<M, NetworkClient> SubprotocolNetworkClient<M, NetworkClient>
where
    M: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
    NetworkClient: NetworkClientInterface<M>,
{
    /// Returns a new sub-protocol network client
    pub fn new(network_client: NetworkClient) -> Self {
        Self {
            network_client,
            _phantom: PhantomData,
        }
    }

    /// Send a single message to the destination peer
    pub fn send_to(&self, peer: PeerId, message: M) -> Result<(), Error> {
        let peer_network_id = PeerNetworkId::new(NetworkId::Validator, peer);
        self.network_client.send_to_peer(message, peer_network_id)
    }

    /// Send a single message to the destination peers
    pub fn send_to_many(&self, peers: Vec<PeerId>, message: M) -> Result<(), Error> {
        let peer_network_ids: Vec<PeerNetworkId> = peers
            .into_iter()
            .map(|peer| PeerNetworkId::new(NetworkId::Validator, peer))
            .collect();
        self.network_client.send_to_peers(message, peer_network_ids)
    }
}

/// Type aliases for backward compatibility
pub type PrefixConsensusNetworkClient<NC> = SubprotocolNetworkClient<PrefixConsensusMsg, NC>;
pub type StrongPrefixConsensusNetworkClient<NC> =
    SubprotocolNetworkClient<StrongPrefixConsensusMsg, NC>;
pub type SlotConsensusNetworkClient<NC> = SubprotocolNetworkClient<SlotConsensusMsg, NC>;

// =============================================================================
// Prefix Consensus sender adapter (unique: 3 vote methods)
// =============================================================================

/// Network sender adapter for Prefix Consensus
///
/// Implements the PrefixConsensusNetworkSender trait using the Aptos network layer.
/// Handles self-send via a channel since the network layer doesn't support sending to self.
#[derive(Clone)]
pub struct NetworkSenderAdapter<NetworkClient> {
    /// This validator's author address
    author: Author,

    /// Network client for sending to other validators
    network_client: PrefixConsensusNetworkClient<NetworkClient>,

    /// Channel for sending messages to self (bypasses network)
    self_sender: UnboundedSender<(Author, PrefixConsensusMsg)>,

    /// Validator set for determining broadcast recipients
    validators: Arc<ValidatorVerifier>,
}

impl<NetworkClient> NetworkSenderAdapter<NetworkClient> {
    /// Create a new network sender adapter
    pub fn new(
        author: Author,
        network_client: PrefixConsensusNetworkClient<NetworkClient>,
        self_sender: UnboundedSender<(Author, PrefixConsensusMsg)>,
        validators: Arc<ValidatorVerifier>,
    ) -> Self {
        Self {
            author,
            network_client,
            self_sender,
            validators,
        }
    }

    /// Get this validator's author address
    pub fn author(&self) -> Author {
        self.author
    }

    /// Get the list of other validators (excluding self)
    fn other_validators(&self) -> Vec<Author> {
        self.validators
            .get_ordered_account_addresses_iter()
            .filter(|addr| addr != &self.author)
            .collect()
    }
}

impl<NetworkClient> NetworkSenderAdapter<NetworkClient>
where
    NetworkClient: NetworkClientInterface<PrefixConsensusMsg> + Send + Sync + Clone,
{
    /// Generic helper to broadcast any vote type
    ///
    /// This reduces code duplication across broadcast_vote1/2/3 methods.
    async fn broadcast_vote<V>(&self, vote: V, vote_name: &str)
    where
        V: Into<PrefixConsensusMsg>,
    {
        let msg = vote.into();

        // Send to self via channel
        let mut self_sender = self.self_sender.clone();
        if let Err(err) = self_sender.send((self.author, msg.clone())).await {
            error!(
                error = ?err,
                vote_type = vote_name,
                "Failed to send vote to self via channel"
            );
        }

        // Broadcast to other validators
        let other_validators = self.other_validators();
        if !other_validators.is_empty() {
            if let Err(err) = self.network_client.send_to_many(other_validators, msg) {
                warn!(
                    error = ?err,
                    vote_type = vote_name,
                    "Failed to broadcast vote to other validators"
                );
            }
        }
    }
}

#[async_trait::async_trait]
impl<NetworkClient> PrefixConsensusNetworkSender for NetworkSenderAdapter<NetworkClient>
where
    NetworkClient: NetworkClientInterface<PrefixConsensusMsg> + Send + Sync + Clone,
{
    async fn broadcast_vote1(&self, vote: Vote1) {
        self.broadcast_vote(vote, "Vote1").await;
    }

    async fn broadcast_vote2(&self, vote: Vote2) {
        self.broadcast_vote(vote, "Vote2").await;
    }

    async fn broadcast_vote3(&self, vote: Vote3) {
        self.broadcast_vote(vote, "Vote3").await;
    }
}

// =============================================================================
// Generic sender adapter (Strong PC + Slot Consensus)
// =============================================================================

/// Generic sender adapter for sub-protocol messages.
///
/// Implements `SubprotocolNetworkSender<M>` using the Aptos network layer.
/// Self-send goes through a channel; all other sends go through the network client.
pub struct SubprotocolSenderAdapter<M, NetworkClient> {
    author: Author,
    network_client: SubprotocolNetworkClient<M, NetworkClient>,
    self_sender: UnboundedSender<(Author, M)>,
    validators: Arc<ValidatorVerifier>,
}

// Manual Clone impl to avoid unnecessary M: Clone bound from derive
impl<M, NC: Clone> Clone for SubprotocolSenderAdapter<M, NC> {
    fn clone(&self) -> Self {
        Self {
            author: self.author,
            network_client: self.network_client.clone(),
            self_sender: self.self_sender.clone(),
            validators: self.validators.clone(),
        }
    }
}

impl<M, NetworkClient> SubprotocolSenderAdapter<M, NetworkClient> {
    pub fn new(
        author: Author,
        network_client: SubprotocolNetworkClient<M, NetworkClient>,
        self_sender: UnboundedSender<(Author, M)>,
        validators: Arc<ValidatorVerifier>,
    ) -> Self {
        Self {
            author,
            network_client,
            self_sender,
            validators,
        }
    }

    fn other_validators(&self) -> Vec<Author> {
        self.validators
            .get_ordered_account_addresses_iter()
            .filter(|addr| addr != &self.author)
            .collect()
    }
}

#[async_trait::async_trait]
impl<M, NetworkClient> SubprotocolNetworkSender<M>
    for SubprotocolSenderAdapter<M, NetworkClient>
where
    M: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
    NetworkClient: NetworkClientInterface<M> + Send + Sync + Clone,
{
    async fn broadcast(&self, msg: M) {
        // Send to self via channel
        let mut self_sender = self.self_sender.clone();
        if let Err(err) = self_sender.send((self.author, msg.clone())).await {
            error!(
                error = ?err,
                "Failed to send sub-protocol msg to self via channel"
            );
        }

        // Send to all other validators
        let others = self.other_validators();
        if !others.is_empty() {
            if let Err(err) = self.network_client.send_to_many(others, msg) {
                warn!(
                    error = ?err,
                    "Failed to broadcast sub-protocol msg to other validators"
                );
            }
        }
    }

    async fn send_to(&self, peer: Author, msg: M) {
        if peer == self.author {
            // Self-send via channel
            let mut self_sender = self.self_sender.clone();
            if let Err(err) = self_sender.send((self.author, msg)).await {
                error!(
                    error = ?err,
                    "Failed to send sub-protocol msg to self via channel"
                );
            }
        } else {
            // Send to remote peer via network
            if let Err(err) = self.network_client.send_to(peer, msg) {
                warn!(
                    error = ?err,
                    peer = %peer,
                    "Failed to send sub-protocol msg to peer"
                );
            }
        }
    }
}

/// Type aliases for backward compatibility
pub type StrongNetworkSenderAdapter<NC> =
    SubprotocolSenderAdapter<StrongPrefixConsensusMsg, NC>;
pub type SlotNetworkSenderAdapter<NC> = SubprotocolSenderAdapter<SlotConsensusMsg, NC>;

#[cfg(test)]
mod tests {
    use aptos_types::{
        validator_signer::ValidatorSigner,
        validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
    };
    use std::sync::Arc;

    fn create_test_validators(count: usize) -> (Vec<ValidatorSigner>, Arc<ValidatorVerifier>) {
        let signers: Vec<_> = (0..count)
            .map(|_| ValidatorSigner::random(None))
            .collect();

        let validator_infos: Vec<_> = signers
            .iter()
            .map(|signer| {
                ValidatorConsensusInfo::new(
                    signer.author(),
                    signer.public_key(),
                    1, // voting power
                )
            })
            .collect();

        let verifier = Arc::new(ValidatorVerifier::new(validator_infos));
        (signers, verifier)
    }

    // Note: Full integration tests for network sending are deferred to smoke tests in Phase 7-9
    // The network interface compiles correctly and will be tested end-to-end with real validators

    #[test]
    fn test_types_compile() {
        // Basic compile-time test that the types work together
        let (_signers, verifier) = create_test_validators(4);
        assert_eq!(verifier.len(), 4);
    }
}
