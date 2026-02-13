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
    types::{Vote1, Vote2, Vote3},
};
use aptos_channels::UnboundedSender;
use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_consensus_types::common::Author;
use aptos_logger::prelude::*;
use aptos_network::application::{error::Error, interface::NetworkClientInterface};
use aptos_types::{validator_verifier::ValidatorVerifier, PeerId};
use futures::SinkExt;
use std::sync::Arc;

/// Trait for sending Prefix Consensus messages over the network
///
/// This trait abstracts the network layer, allowing the protocol to send votes
/// without depending on specific network implementation details.
#[async_trait::async_trait]
pub trait PrefixConsensusNetworkSender: Send + Sync + Clone {
    /// Broadcast a Vote1 message to all validators
    async fn broadcast_vote1(&self, vote: Vote1);

    /// Broadcast a Vote2 message to all validators
    async fn broadcast_vote2(&self, vote: Vote2);

    /// Broadcast a Vote3 message to all validators
    async fn broadcast_vote3(&self, vote: Vote3);
}

/// Trait for sending Strong Prefix Consensus messages over the network
#[async_trait::async_trait]
pub trait StrongPrefixConsensusNetworkSender: Send + Sync + Clone {
    /// Broadcast a Strong PC message to all validators
    async fn broadcast_strong_msg(&self, msg: StrongPrefixConsensusMsg);

    /// Send a Strong PC message to a specific peer
    async fn send_strong_msg(&self, peer: Author, msg: StrongPrefixConsensusMsg);
}

/// Network client wrapper for Prefix Consensus
///
/// Wraps the generic NetworkClient to provide Prefix Consensus-specific
/// sending methods. Mirrors the pattern used by ConsensusNetworkClient.
#[derive(Clone)]
pub struct PrefixConsensusNetworkClient<NetworkClient> {
    network_client: NetworkClient,
}

impl<NetworkClient: NetworkClientInterface<PrefixConsensusMsg>>
    PrefixConsensusNetworkClient<NetworkClient>
{
    /// Returns a new prefix consensus network client
    pub fn new(network_client: NetworkClient) -> Self {
        Self { network_client }
    }

    /// Send a single message to the destination peer
    pub fn send_to(&self, peer: PeerId, message: PrefixConsensusMsg) -> Result<(), Error> {
        let peer_network_id = self.get_peer_network_id_for_peer(peer);
        self.network_client.send_to_peer(message, peer_network_id)
    }

    /// Send a single message to the destination peers
    pub fn send_to_many(
        &self,
        peers: Vec<PeerId>,
        message: PrefixConsensusMsg,
    ) -> Result<(), Error> {
        let peer_network_ids: Vec<PeerNetworkId> = peers
            .into_iter()
            .map(|peer| self.get_peer_network_id_for_peer(peer))
            .collect();
        self.network_client.send_to_peers(message, peer_network_ids)
    }

    fn get_peer_network_id_for_peer(&self, peer: PeerId) -> PeerNetworkId {
        PeerNetworkId::new(NetworkId::Validator, peer)
    }
}

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
// Strong Prefix Consensus network types
// =============================================================================

/// Network client wrapper for Strong Prefix Consensus
///
/// Same pattern as `PrefixConsensusNetworkClient` but for `StrongPrefixConsensusMsg`.
#[derive(Clone)]
pub struct StrongPrefixConsensusNetworkClient<NetworkClient> {
    network_client: NetworkClient,
}

impl<NetworkClient: NetworkClientInterface<StrongPrefixConsensusMsg>>
    StrongPrefixConsensusNetworkClient<NetworkClient>
{
    pub fn new(network_client: NetworkClient) -> Self {
        Self { network_client }
    }

    pub fn send_to(
        &self,
        peer: PeerId,
        message: StrongPrefixConsensusMsg,
    ) -> Result<(), Error> {
        let peer_network_id = PeerNetworkId::new(NetworkId::Validator, peer);
        self.network_client.send_to_peer(message, peer_network_id)
    }

    pub fn send_to_many(
        &self,
        peers: Vec<PeerId>,
        message: StrongPrefixConsensusMsg,
    ) -> Result<(), Error> {
        let peer_network_ids: Vec<PeerNetworkId> = peers
            .into_iter()
            .map(|peer| PeerNetworkId::new(NetworkId::Validator, peer))
            .collect();
        self.network_client
            .send_to_peers(message, peer_network_ids)
    }
}

/// Network sender adapter for Strong Prefix Consensus
///
/// Implements `StrongPrefixConsensusNetworkSender` using the Aptos network layer.
/// Self-send goes through a channel; all other sends go through the network client.
#[derive(Clone)]
pub struct StrongNetworkSenderAdapter<NetworkClient> {
    author: Author,
    network_client: StrongPrefixConsensusNetworkClient<NetworkClient>,
    self_sender: UnboundedSender<(Author, StrongPrefixConsensusMsg)>,
    validators: Arc<ValidatorVerifier>,
}

impl<NetworkClient> StrongNetworkSenderAdapter<NetworkClient> {
    pub fn new(
        author: Author,
        network_client: StrongPrefixConsensusNetworkClient<NetworkClient>,
        self_sender: UnboundedSender<(Author, StrongPrefixConsensusMsg)>,
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
impl<NetworkClient> StrongPrefixConsensusNetworkSender
    for StrongNetworkSenderAdapter<NetworkClient>
where
    NetworkClient:
        NetworkClientInterface<StrongPrefixConsensusMsg> + Send + Sync + Clone,
{
    async fn broadcast_strong_msg(&self, msg: StrongPrefixConsensusMsg) {
        // Send to self via channel
        let mut self_sender = self.self_sender.clone();
        if let Err(err) = self_sender.send((self.author, msg.clone())).await {
            error!(
                error = ?err,
                "Failed to send strong PC msg to self via channel"
            );
        }

        // Send to all other validators
        let others = self.other_validators();
        if !others.is_empty() {
            if let Err(err) = self.network_client.send_to_many(others, msg) {
                warn!(
                    error = ?err,
                    "Failed to broadcast strong PC msg to other validators"
                );
            }
        }
    }

    async fn send_strong_msg(&self, peer: Author, msg: StrongPrefixConsensusMsg) {
        if peer == self.author {
            // Self-send via channel
            let mut self_sender = self.self_sender.clone();
            if let Err(err) = self_sender.send((self.author, msg)).await {
                error!(
                    error = ?err,
                    "Failed to send strong PC msg to self via channel"
                );
            }
        } else {
            // Send to remote peer via network
            if let Err(err) = self.network_client.send_to(peer, msg) {
                warn!(
                    error = ?err,
                    peer = %peer,
                    "Failed to send strong PC msg to peer"
                );
            }
        }
    }
}

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
