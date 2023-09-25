// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Convenience Network API for Aptos

pub use crate::protocols::rpc::error::RpcError;
use crate::{
    error::NetworkError,
    peer_manager::{
        ConnectionNotification, ConnectionRequestSender, PeerManagerNotification,
        PeerManagerRequestSender,
    },
    transport::ConnectionMetadata,
    ProtocolId,
};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_logger::prelude::*;
use aptos_short_hex_str::AsShortHexStr;
use aptos_types::{network_address::NetworkAddress, PeerId};
use bytes::Bytes;
use futures::{
    channel::oneshot,
    stream::{FusedStream, Map, Select, Stream, StreamExt},
    task::{Context, Poll},
};
use futures_util::FutureExt;
use pin_project::pin_project;
use serde::{de::DeserializeOwned, Serialize};
use std::{cmp::min, fmt::Debug, marker::PhantomData, pin::Pin, time::Duration};

pub trait Message: DeserializeOwned + Serialize {}
impl<T: DeserializeOwned + Serialize> Message for T {}

// TODO: do we want to make this configurable?
const MAX_DESERIALIZATION_QUEUE_SIZE_PER_PEER: usize = 50;

/// Events received by network clients in a validator
///
/// An enumeration of the various types of messages that the network will be sending
/// to its clients. This differs from [`PeerNotification`] since the contents are deserialized
/// into the type `TMessage` over which `Event` is generic. Note that we assume here that for every
/// consumer of this API there's a singleton message type, `TMessage`,  which encapsulates all the
/// messages and RPCs that are received by that consumer.
///
/// [`PeerNotification`]: crate::peer::PeerNotification
#[derive(Debug)]
pub enum Event<TMessage> {
    /// New inbound direct-send message from peer.
    Message(PeerId, TMessage),
    /// New inbound rpc request. The request is fulfilled by sending the
    /// serialized response `Bytes` over the `oneshot::Sender`, where the network
    /// layer will handle sending the response over-the-wire.
    RpcRequest(
        PeerId,
        TMessage,
        ProtocolId,
        oneshot::Sender<Result<Bytes, RpcError>>,
    ),
    /// Peer which we have a newly established connection with.
    NewPeer(ConnectionMetadata),
    /// Peer with which we've lost our connection.
    LostPeer(ConnectionMetadata),
}

/// impl PartialEq for simpler testing
impl<TMessage: PartialEq> PartialEq for Event<TMessage> {
    fn eq(&self, other: &Event<TMessage>) -> bool {
        use Event::*;
        match (self, other) {
            (Message(pid1, msg1), Message(pid2, msg2)) => pid1 == pid2 && msg1 == msg2,
            // ignore oneshot::Sender in comparison
            (RpcRequest(pid1, msg1, proto1, _), RpcRequest(pid2, msg2, proto2, _)) => {
                pid1 == pid2 && msg1 == msg2 && proto1 == proto2
            },
            (NewPeer(metadata1), NewPeer(metadata2)) => metadata1 == metadata2,
            (LostPeer(metadata1), LostPeer(metadata2)) => metadata1 == metadata2,
            _ => false,
        }
    }
}

/// Configuration needed for the client side of AptosNet applications
#[derive(Clone)]
pub struct NetworkClientConfig {
    /// Direct send protocols for the application (sorted by preference, highest to lowest)
    pub direct_send_protocols_and_preferences: Vec<ProtocolId>,
    /// RPC protocols for the application (sorted by preference, highest to lowest)
    pub rpc_protocols_and_preferences: Vec<ProtocolId>,
}

impl NetworkClientConfig {
    pub fn new(
        direct_send_protocols_and_preferences: Vec<ProtocolId>,
        rpc_protocols_and_preferences: Vec<ProtocolId>,
    ) -> Self {
        Self {
            direct_send_protocols_and_preferences,
            rpc_protocols_and_preferences,
        }
    }
}

/// Configuration needed for the service side of AptosNet applications
#[derive(Clone)]
pub struct NetworkServiceConfig {
    /// Direct send protocols for the application (sorted by preference, highest to lowest)
    pub direct_send_protocols_and_preferences: Vec<ProtocolId>,
    /// RPC protocols for the application (sorted by preference, highest to lowest)
    pub rpc_protocols_and_preferences: Vec<ProtocolId>,
    /// The inbound queue config (from the network to the application)
    pub inbound_queue_config: aptos_channel::Config,
}

impl NetworkServiceConfig {
    pub fn new(
        direct_send_protocols_and_preferences: Vec<ProtocolId>,
        rpc_protocols_and_preferences: Vec<ProtocolId>,
        inbound_queue_config: aptos_channel::Config,
    ) -> Self {
        Self {
            direct_send_protocols_and_preferences,
            rpc_protocols_and_preferences,
            inbound_queue_config,
        }
    }
}

/// Configuration needed for AptosNet applications to register with the network
/// builder. Supports client and service side.
#[derive(Clone)]
pub struct NetworkApplicationConfig {
    pub network_client_config: NetworkClientConfig,
    pub network_service_config: NetworkServiceConfig,
}

impl NetworkApplicationConfig {
    pub fn new(
        network_client_config: NetworkClientConfig,
        network_service_config: NetworkServiceConfig,
    ) -> Self {
        Self {
            network_client_config,
            network_service_config,
        }
    }
}

/// A `Stream` of `Event<TMessage>` from the lower network layer to an upper
/// network application that deserializes inbound network direct-send and rpc
/// messages into `TMessage`. Inbound messages that fail to deserialize are logged
/// and dropped.
///
/// `NetworkEvents` is really just a thin wrapper around a
/// `channel::Receiver<PeerNotification>` that deserializes inbound messages.
#[pin_project]
pub struct NetworkEvents<TMessage> {
    #[pin]
    event_stream: Select<
        aptos_channel::Receiver<PeerId, Event<TMessage>>,
        Map<
            aptos_channel::Receiver<PeerId, ConnectionNotification>,
            fn(ConnectionNotification) -> Event<TMessage>,
        >,
    >,
    _marker: PhantomData<TMessage>,
}

/// Trait specifying the signature for `new()` `NetworkEvents`
pub trait NewNetworkEvents {
    fn new(
        peer_mgr_notifs_rx: aptos_channel::Receiver<(PeerId, ProtocolId), PeerManagerNotification>,
        connection_notifs_rx: aptos_channel::Receiver<PeerId, ConnectionNotification>,
        max_parallel_deserialization_tasks: Option<usize>,
    ) -> Self;
}

impl<TMessage: Message + Send + 'static> NewNetworkEvents for NetworkEvents<TMessage> {
    fn new(
        peer_mgr_notifs_rx: aptos_channel::Receiver<(PeerId, ProtocolId), PeerManagerNotification>,
        connection_notifs_rx: aptos_channel::Receiver<PeerId, ConnectionNotification>,
        max_parallel_deserialization_tasks: Option<usize>,
    ) -> Self {
        // Create a channel for deserialized messages
        let (deserialized_message_sender, deserialized_message_receiver) = aptos_channel::new(
            QueueStyle::FIFO,
            MAX_DESERIALIZATION_QUEUE_SIZE_PER_PEER,
            None,
        );

        // Deserialize the peer manager notifications in parallel (for each
        // network application) and send them to the receiver. Note: this
        // may cause out of order message delivery, but applications
        // should already be handling this.
        tokio::spawn(async move {
            peer_mgr_notifs_rx
                .for_each_concurrent(
                    max_parallel_deserialization_tasks,
                    move |peer_manager_notification| {
                        // Get the peer ID for the notification
                        let deserialized_message_sender = deserialized_message_sender.clone();
                        let peer_id_for_notification = peer_manager_notification.get_peer_id();

                        // Spawn a new blocking task to deserialize the message
                        tokio::task::spawn_blocking(move || {
                            if let Some(deserialized_message) =
                                peer_mgr_notif_to_event(peer_manager_notification)
                            {
                                if let Err(error) = deserialized_message_sender
                                    .push(peer_id_for_notification, deserialized_message)
                                {
                                    warn!(
                                        "Failed to send deserialized message to receiver: {:?}",
                                        error
                                    );
                                }
                            }
                        })
                        .map(|_| ())
                    },
                )
                .await
        });

        // Process the control messages
        let control_event_stream = connection_notifs_rx
            .map(control_msg_to_event as fn(ConnectionNotification) -> Event<TMessage>);

        Self {
            event_stream: ::futures::stream::select(
                deserialized_message_receiver,
                control_event_stream,
            ),
            _marker: PhantomData,
        }
    }
}

impl<TMessage> Stream for NetworkEvents<TMessage> {
    type Item = Event<TMessage>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Self::Item>> {
        self.project().event_stream.poll_next(context)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.event_stream.size_hint()
    }
}

/// Deserialize inbound direct send and rpc messages into the application `TMessage`
/// type, logging and dropping messages that fail to deserialize.
fn peer_mgr_notif_to_event<TMessage: Message>(
    notification: PeerManagerNotification,
) -> Option<Event<TMessage>> {
    match notification {
        PeerManagerNotification::RecvRpc(peer_id, rpc_req) => {
            request_to_network_event(peer_id, &rpc_req)
                .map(|msg| Event::RpcRequest(peer_id, msg, rpc_req.protocol_id, rpc_req.res_tx))
        },
        PeerManagerNotification::RecvMessage(peer_id, request) => {
            request_to_network_event(peer_id, &request).map(|msg| Event::Message(peer_id, msg))
        },
    }
}

/// Converts a `SerializedRequest` into a network `Event` for sending to other nodes
fn request_to_network_event<TMessage: Message, Request: SerializedRequest>(
    peer_id: PeerId,
    request: &Request,
) -> Option<TMessage> {
    match request.to_message() {
        Ok(msg) => Some(msg),
        Err(err) => {
            let data = &request.data();
            warn!(
                SecurityEvent::InvalidNetworkEvent,
                error = ?err,
                remote_peer_id = peer_id.short_str(),
                protocol_id = request.protocol_id(),
                data_prefix = hex::encode(&data[..min(16, data.len())]),
            );
            None
        },
    }
}

fn control_msg_to_event<TMessage>(notif: ConnectionNotification) -> Event<TMessage> {
    match notif {
        ConnectionNotification::NewPeer(metadata, _context) => Event::NewPeer(metadata),
        ConnectionNotification::LostPeer(metadata, _context, _reason) => Event::LostPeer(metadata),
    }
}

impl<TMessage> FusedStream for NetworkEvents<TMessage> {
    fn is_terminated(&self) -> bool {
        self.event_stream.is_terminated()
    }
}

/// `NetworkSender` is the generic interface from upper network applications to
/// the lower network layer. It provides the full API for network applications,
/// including sending direct-send messages, sending rpc requests, as well as
/// dialing or disconnecting from peers and updating the list of accepted public
/// keys.
///
/// `NetworkSender` is in fact a thin wrapper around a `PeerManagerRequestSender`, which in turn is
/// a thin wrapper on `aptos_channel::Sender<(PeerId, ProtocolId), PeerManagerRequest>`,
/// mostly focused on providing a more ergonomic API. However, network applications will usually
/// provide their own thin wrapper around `NetworkSender` that narrows the API to the specific
/// interface they need.
///
/// Provide Protobuf wrapper over `[peer_manager::PeerManagerRequestSender]`
#[derive(Clone, Debug)]
pub struct NetworkSender<TMessage> {
    peer_mgr_reqs_tx: PeerManagerRequestSender,
    connection_reqs_tx: ConnectionRequestSender,
    _marker: PhantomData<TMessage>,
}

/// Trait specifying the signature for `new()` `NetworkSender`s
pub trait NewNetworkSender {
    fn new(
        peer_mgr_reqs_tx: PeerManagerRequestSender,
        connection_reqs_tx: ConnectionRequestSender,
    ) -> Self;
}

impl<TMessage> NewNetworkSender for NetworkSender<TMessage> {
    fn new(
        peer_mgr_reqs_tx: PeerManagerRequestSender,
        connection_reqs_tx: ConnectionRequestSender,
    ) -> Self {
        Self {
            peer_mgr_reqs_tx,
            connection_reqs_tx,
            _marker: PhantomData,
        }
    }
}

impl<TMessage> NetworkSender<TMessage> {
    /// Request that a given Peer be dialed at the provided `NetworkAddress` and
    /// synchronously wait for the request to be performed.
    pub async fn dial_peer(&self, peer: PeerId, addr: NetworkAddress) -> Result<(), NetworkError> {
        self.connection_reqs_tx.dial_peer(peer, addr).await?;
        Ok(())
    }

    /// Request that a given Peer be disconnected and synchronously wait for the request to be
    /// performed.
    pub async fn disconnect_peer(&self, peer: PeerId) -> Result<(), NetworkError> {
        self.connection_reqs_tx.disconnect_peer(peer).await?;
        Ok(())
    }
}

impl<TMessage: Message> NetworkSender<TMessage> {
    /// Send a protobuf message to a single recipient. Provides a wrapper over
    /// `[peer_manager::PeerManagerRequestSender::send_to]`.
    pub fn send_to(
        &self,
        recipient: PeerId,
        protocol: ProtocolId,
        message: TMessage,
    ) -> Result<(), NetworkError> {
        let mdata = protocol.to_bytes(&message)?.into();
        self.peer_mgr_reqs_tx.send_to(recipient, protocol, mdata)?;
        Ok(())
    }

    /// Send a protobuf message to a many recipients. Provides a wrapper over
    /// `[peer_manager::PeerManagerRequestSender::send_to_many]`.
    pub fn send_to_many(
        &self,
        recipients: impl Iterator<Item = PeerId>,
        protocol: ProtocolId,
        message: TMessage,
    ) -> Result<(), NetworkError> {
        // Serialize message.
        let mdata = protocol.to_bytes(&message)?.into();
        self.peer_mgr_reqs_tx
            .send_to_many(recipients, protocol, mdata)?;
        Ok(())
    }

    /// Send a protobuf rpc request to a single recipient while handling
    /// serialization and deserialization of the request and response respectively.
    /// Assumes that the request and response both have the same message type.
    pub async fn send_rpc(
        &self,
        recipient: PeerId,
        protocol: ProtocolId,
        req_msg: TMessage,
        timeout: Duration,
    ) -> Result<TMessage, RpcError> {
        // serialize request
        let req_data = protocol.to_bytes(&req_msg)?.into();
        let res_data = self
            .peer_mgr_reqs_tx
            .send_rpc(recipient, protocol, req_data, timeout)
            .await?;
        let res_msg: TMessage = protocol.from_bytes(&res_data)?;
        Ok(res_msg)
    }
}

/// Generalized functionality for any request across `DirectSend` and `Rpc`.
pub trait SerializedRequest {
    fn protocol_id(&self) -> ProtocolId;
    fn data(&self) -> &Bytes;

    /// Converts the `SerializedMessage` into its deserialized version of `TMessage` based on the
    /// `ProtocolId`.  See: [`ProtocolId::from_bytes`]
    fn to_message<TMessage: DeserializeOwned>(&self) -> anyhow::Result<TMessage> {
        self.protocol_id().from_bytes(self.data())
    }
}
