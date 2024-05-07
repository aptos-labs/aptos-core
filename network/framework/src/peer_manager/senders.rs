// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    peer_manager::{
        types::PeerManagerRequest, ConnectionRequest, MessageAndMetadata, PeerManagerError,
    },
    protocols::rpc::{error::RpcError, OutboundRpcRequest},
    ProtocolId,
};
use aptos_channels::{self, aptos_channel};
use aptos_types::{network_address::NetworkAddress, PeerId};
use bytes::Bytes;
use futures::channel::oneshot;
use std::time::Duration;

/// Convenience wrapper which makes it easy to issue communication requests and await the responses
/// from PeerManager.
#[derive(Clone, Debug)]
pub struct PeerManagerRequestSender {
    inner: aptos_channel::Sender<(PeerId, ProtocolId), PeerManagerRequest>,
}

/// Convenience wrapper which makes it easy to issue connection requests and await the responses
/// from PeerManager.
#[derive(Clone, Debug)]
pub struct ConnectionRequestSender {
    inner: aptos_channel::Sender<PeerId, ConnectionRequest>,
}

impl PeerManagerRequestSender {
    /// Construct a new PeerManagerRequestSender with a raw channel::Sender
    pub fn new(inner: aptos_channel::Sender<(PeerId, ProtocolId), PeerManagerRequest>) -> Self {
        Self { inner }
    }

    /// Send a fire-and-forget direct-send message to remote peer.
    ///
    /// The function returns when the message has been enqueued on the network actor's event queue.
    /// It therefore makes no reliable delivery guarantees. An error is returned if the event queue
    /// is unexpectedly shutdown.
    pub fn send_to(
        &self,
        peer_id: PeerId,
        protocol_id: ProtocolId,
        message_and_metadata: MessageAndMetadata,
    ) -> Result<(), PeerManagerError> {
        // Create the peer manager direct send request
        let request = PeerManagerRequest::new_direct_send(peer_id, message_and_metadata);

        // Send the request to the peer manager
        self.inner.push((peer_id, protocol_id), request)?;
        Ok(())
    }

    /// Send the _same_ message to many recipients using the direct-send protocol.
    ///
    /// This method is an optimization so that we can avoid serializing and
    /// copying the same message many times when we want to sent a single message
    /// to many peers. Note that the `Bytes` the messages is serialized into is a
    /// ref-counted byte buffer, so we can avoid excess copies as all direct-sends
    /// will share the same underlying byte buffer.
    ///
    /// The function returns when all send requests have been enqueued on the network
    /// actor's event queue. It therefore makes no reliable delivery guarantees.
    /// An error is returned if the event queue is unexpectedly shutdown.
    pub fn send_to_many(
        &self,
        recipients: impl Iterator<Item = PeerId>,
        protocol_id: ProtocolId,
        message_and_metadata: MessageAndMetadata,
    ) -> Result<(), PeerManagerError> {
        for recipient in recipients {
            // Create the peer manager direct send request
            let request =
                PeerManagerRequest::new_direct_send(recipient, message_and_metadata.clone());

            // Send the request to the peer manager.
            // We return `Err` early here if the send fails. Since sending will
            // only fail if the queue is unexpectedly shutdown (i.e., receiver
            // dropped early), we know that we can't make further progress if
            // this send fails.
            self.inner.push((recipient, protocol_id), request)?;
        }

        Ok(())
    }

    /// Sends a unary RPC to a remote peer and waits to either receive a response or times out.
    pub async fn send_rpc(
        &self,
        peer_id: PeerId,
        protocol_id: ProtocolId,
        message_and_metadata: MessageAndMetadata,
        timeout: Duration,
    ) -> Result<Bytes, RpcError> {
        // Create the peer manager RPC request
        let (res_tx, res_rx) = oneshot::channel();
        let outbound_rpc_request = OutboundRpcRequest {
            protocol_id,
            message_and_metadata,
            res_tx,
            timeout,
        };
        let request = PeerManagerRequest::SendRpc(peer_id, outbound_rpc_request);

        // Send the request to the peer manager
        self.inner.push((peer_id, protocol_id), request)?;

        // Wait for the response
        res_rx.await?
    }
}

impl ConnectionRequestSender {
    /// Construct a new ConnectionRequestSender with a raw aptos_channel::Sender
    pub fn new(inner: aptos_channel::Sender<PeerId, ConnectionRequest>) -> Self {
        Self { inner }
    }

    pub async fn dial_peer(
        &self,
        peer: PeerId,
        addr: NetworkAddress,
    ) -> Result<(), PeerManagerError> {
        let (oneshot_tx, oneshot_rx) = oneshot::channel();
        self.inner
            .push(peer, ConnectionRequest::DialPeer(peer, addr, oneshot_tx))?;
        oneshot_rx.await?
    }

    pub async fn disconnect_peer(&self, peer: PeerId) -> Result<(), PeerManagerError> {
        let (oneshot_tx, oneshot_rx) = oneshot::channel();
        self.inner
            .push(peer, ConnectionRequest::DisconnectPeer(peer, oneshot_tx))?;
        oneshot_rx.await?
    }
}
