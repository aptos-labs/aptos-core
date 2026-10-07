// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Convenience Network API for Aptos

pub use crate::protocols::rpc::error::RpcError;
use crate::{
    error::NetworkError,
    peer::DisconnectReason,
    peer_manager::{ConnectionRequestSender, PeerManagerRequestSender},
    protocols::wire::messaging::v1::{IncomingRequest, NetworkMessage},
    ProtocolId,
};
use aptos_channels::aptos_channel;
use aptos_config::network_id::PeerNetworkId;
use aptos_logger::prelude::*;
use aptos_short_hex_str::AsShortHexStr;
use aptos_types::{network_address::NetworkAddress, PeerId};
use bytes::Bytes;
use futures::{
    channel::oneshot,
    stream::{FusedStream, FuturesUnordered, Stream, StreamExt},
    task::{Context, Poll},
};
use futures_util::ready;
use pin_project::pin_project;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    cmp::min, collections::HashSet, fmt::Debug, future, marker::PhantomData, pin::Pin, sync::Arc,
    time::Duration,
};

pub trait Message: DeserializeOwned + Serialize {}
impl<T: DeserializeOwned + Serialize> Message for T {}

/// Events received by network clients in a validator
///
/// We assume here that for every consumer of this API there's a singleton message type, `TMessage`,
/// which encapsulates all the messages and RPCs that are received by that consumer.
/// This struct is received by application code after a message of bytes has been deserialized
/// into the application code's TMessage struct type.
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

#[derive(Debug, Clone)]
pub struct ReceivedMessage {
    pub message: NetworkMessage,
    pub sender: PeerNetworkId,

    // unix microseconds
    pub receive_timestamp_micros: u64,

    pub rpc_replier: Option<Arc<oneshot::Sender<Result<Bytes, RpcError>>>>,
}

impl ReceivedMessage {
    pub fn new(message: NetworkMessage, sender: PeerNetworkId) -> Self {
        let rx_at = unix_micros();
        Self {
            message,
            sender,
            receive_timestamp_micros: rx_at,
            rpc_replier: None,
        }
    }

    pub fn protocol_id(&self) -> Option<ProtocolId> {
        match &self.message {
            NetworkMessage::Error(_e) => None,
            NetworkMessage::RpcRequest(req) => Some(req.protocol_id),
            NetworkMessage::RpcResponse(_response) => {
                // design of RpcResponse lacking ProtocolId requires global rpc counter (or at least per-peer) and requires reply matching globally or per-peer
                None
            },
            NetworkMessage::DirectSendMsg(msg) => Some(msg.protocol_id),
        }
    }

    pub fn protocol_id_as_str(&self) -> &'static str {
        match &self.message {
            NetworkMessage::Error(_) => "error",
            NetworkMessage::RpcRequest(rr) => rr.protocol_id.as_str(),
            NetworkMessage::RpcResponse(_) => "rpc response",
            NetworkMessage::DirectSendMsg(dm) => dm.protocol_id.as_str(),
        }
    }
}

impl PartialEq for ReceivedMessage {
    fn eq(&self, other: &Self) -> bool {
        (self.message == other.message)
            && (self.receive_timestamp_micros == other.receive_timestamp_micros)
            && (self.sender == other.sender)
    }
}

/// A `Stream` of `Event<TMessage>` from the lower network layer to an upper
/// network application that deserializes inbound network direct-send and rpc
/// messages into `TMessage`. Inbound messages that fail to deserialize are logged
/// and dropped.
#[pin_project]
pub struct NetworkEvents<TMessage> {
    #[pin]
    event_stream: Pin<Box<dyn Stream<Item = Event<TMessage>> + Send + Sync + 'static>>,
    done: bool,
    _marker: PhantomData<TMessage>,
}

/// Trait specifying the signature for `new()` `NetworkEvents`
pub trait NewNetworkEvents {
    fn new(
        peer_mgr_notifs_rx: aptos_channel::Receiver<(PeerId, ProtocolId), ReceivedMessage>,
        max_parallel_deserialization_tasks: Option<usize>,
        allow_out_of_order_delivery: bool,
    ) -> Self;
}

impl<TMessage: Message + Send + Sync + 'static> NewNetworkEvents for NetworkEvents<TMessage> {
    fn new(
        peer_mgr_notifs_rx: aptos_channel::Receiver<(PeerId, ProtocolId), ReceivedMessage>,
        max_parallel_deserialization_tasks: Option<usize>,
        allow_out_of_order_delivery: bool,
    ) -> Self {
        // Determine the number of parallel deserialization tasks to use
        let max_parallel_deserialization_tasks = max_parallel_deserialization_tasks.unwrap_or(1);

        let data_event_stream: Pin<
            Box<dyn Stream<Item = Event<TMessage>> + Send + Sync + 'static>,
        > = if allow_out_of_order_delivery {
            Box::pin(PeerFairDeserializationStream::new(
                peer_mgr_notifs_rx,
                max_parallel_deserialization_tasks,
            ))
        } else {
            let data_event_stream = peer_mgr_notifs_rx.map(|notification| {
                tokio::task::spawn_blocking(move || received_message_to_event(notification))
            });
            Box::pin(
                data_event_stream
                    .buffered(max_parallel_deserialization_tasks)
                    .filter_map(|res| future::ready(res.expect("JoinError from spawn blocking"))),
            )
        };

        Self {
            event_stream: data_event_stream,
            done: false,
            _marker: PhantomData,
        }
    }
}

/// Schedules at most one blocking deserialization per peer while retaining the
/// configured global parallelism across different peers. Messages from active
/// peers stay in the lower network queue with its configured capacity and
/// eviction policy, so normal bursts are not dropped by the scheduler.
struct PeerFairDeserializationStream<TMessage> {
    input: aptos_channel::Receiver<(PeerId, ProtocolId), ReceivedMessage>,
    in_flight: FuturesUnordered<tokio::task::JoinHandle<(PeerId, Option<Event<TMessage>>)>>,
    active_peers: HashSet<PeerId>,
    max_in_flight: usize,
    input_done: bool,
}

impl<TMessage: Message + Send + 'static> PeerFairDeserializationStream<TMessage> {
    fn new(
        input: aptos_channel::Receiver<(PeerId, ProtocolId), ReceivedMessage>,
        max_in_flight: usize,
    ) -> Self {
        Self {
            input,
            in_flight: FuturesUnordered::new(),
            active_peers: HashSet::new(),
            max_in_flight: max_in_flight.max(1),
            input_done: false,
        }
    }
}

impl<TMessage: Message + Send + 'static> Stream for PeerFairDeserializationStream<TMessage> {
    type Item = Event<TMessage>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            // Only dequeue when a worker is available, skipping active peers.
            // The channel's round-robin selection serves other peers without
            // draining or discarding an active peer's backlog.
            while !this.input_done && this.in_flight.len() < this.max_in_flight {
                match this
                    .input
                    .poll_next_filtered(context, |(peer, _)| !this.active_peers.contains(peer))
                {
                    Poll::Ready(Some(message)) => {
                        let peer = message.sender.peer_id();
                        assert!(this.active_peers.insert(peer));
                        this.in_flight.push(tokio::task::spawn_blocking(move || {
                            (peer, received_message_to_event(message))
                        }));
                    },
                    Poll::Ready(None) => this.input_done = true,
                    Poll::Pending => break,
                }
            }

            // Poll newly-created tasks to register their completion wakers.
            match this.in_flight.poll_next_unpin(context) {
                Poll::Ready(Some(result)) => {
                    let (peer, event) = result.expect("JoinError from spawn blocking");
                    assert!(this.active_peers.remove(&peer));
                    if let Some(event) = event {
                        return Poll::Ready(Some(event));
                    }
                },
                Poll::Ready(None) if this.input_done => return Poll::Ready(None),
                Poll::Ready(None) | Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl<TMessage> Stream for NetworkEvents<TMessage> {
    type Item = Event<TMessage>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Self::Item>> {
        let this = self.project();
        if *this.done {
            return Poll::Ready(None);
        }
        let item = ready!(this.event_stream.poll_next(context));
        if item.is_none() {
            *this.done = true;
        }
        Poll::Ready(item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.event_stream.size_hint()
    }
}

fn unix_micros() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64
}

/// Deserialize inbound direct send and rpc messages into the application `TMessage`
/// type, logging and dropping messages that fail to deserialize.
fn received_message_to_event<TMessage: Message>(
    message: ReceivedMessage,
) -> Option<Event<TMessage>> {
    let peer_id = message.sender.peer_id();
    let ReceivedMessage {
        message,
        sender: _sender,
        receive_timestamp_micros: rx_at,
        rpc_replier,
    } = message;
    let dequeue_at = unix_micros();
    let dt_micros = dequeue_at - rx_at;
    let dt_seconds = (dt_micros as f64) / 1000000.0;
    match message {
        NetworkMessage::RpcRequest(rpc_req) => {
            crate::counters::inbound_queue_delay_observe(rpc_req.protocol_id, dt_seconds);
            let rpc_replier = Arc::into_inner(rpc_replier.unwrap()).unwrap();
            request_to_network_event(peer_id, &rpc_req)
                .map(|msg| Event::RpcRequest(peer_id, msg, rpc_req.protocol_id, rpc_replier))
        },
        NetworkMessage::DirectSendMsg(request) => {
            crate::counters::inbound_queue_delay_observe(request.protocol_id, dt_seconds);
            request_to_network_event(peer_id, &request).map(|msg| Event::Message(peer_id, msg))
        },
        _ => None,
    }
}

/// Converts a `SerializedRequest` into a network `Event` for sending to other nodes
fn request_to_network_event<TMessage: Message, Request: IncomingRequest>(
    peer_id: PeerId,
    request: &Request,
) -> Option<TMessage> {
    match request.to_message() {
        Ok(msg) => Some(msg),
        Err(err) => {
            let data = request.data();
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

impl<TMessage> FusedStream for NetworkEvents<TMessage> {
    fn is_terminated(&self) -> bool {
        self.done
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
    pub async fn disconnect_peer(
        &self,
        peer: PeerId,
        disconnect_reason: DisconnectReason,
    ) -> Result<(), NetworkError> {
        self.connection_reqs_tx
            .disconnect_peer(peer, disconnect_reason)
            .await?;
        Ok(())
    }
}

impl<TMessage: Message + Send + 'static> NetworkSender<TMessage> {
    /// Send a protobuf message to a single recipient. Provides a wrapper over
    /// `[peer_manager::PeerManagerRequestSender::send_to]`.
    pub fn send_to(
        &self,
        recipient: PeerId,
        protocol: ProtocolId,
        message: TMessage,
    ) -> Result<(), NetworkError> {
        let mdata = protocol.to_bytes(&message)?.into();
        self.send_to_raw(recipient, protocol, mdata)
    }

    /// Sends a raw message to a single recipient
    pub fn send_to_raw(
        &self,
        recipient: PeerId,
        protocol: ProtocolId,
        message: Bytes,
    ) -> Result<(), NetworkError> {
        self.peer_mgr_reqs_tx
            .send_to(recipient, protocol, message)?;
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
        // Serialize the request using a blocking task
        let req_data = tokio::task::spawn_blocking(move || protocol.to_bytes(&req_msg))
            .await??
            .into();

        // Send the request and wait for the response
        self.send_rpc_raw(recipient, protocol, req_data, timeout)
            .await
    }

    /// Send a protobuf rpc request to a single recipient while handling
    /// serialization and deserialization of the request and response respectively.
    /// Assumes that the request and response both have the same message type.
    pub async fn send_rpc_raw(
        &self,
        recipient: PeerId,
        protocol: ProtocolId,
        req_msg: Bytes,
        timeout: Duration,
    ) -> Result<TMessage, RpcError> {
        // Send the request and wait for the response
        let res_data = self
            .peer_mgr_reqs_tx
            .send_rpc(recipient, protocol, req_msg, timeout)
            .await?;

        // Deserialize the response using a blocking task
        let res_msg = tokio::task::spawn_blocking(move || protocol.from_bytes(&res_data)).await??;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::wire::messaging::v1::DirectSendMsg;
    use aptos_channels::message_queues::QueueStyle;
    use aptos_config::network_id::NetworkId;
    use serde::{Deserialize, Deserializer};

    #[derive(Debug, Eq, PartialEq, Serialize)]
    struct DelayedMessage {
        delay_ms: u64,
        id: u8,
    }

    impl<'de> Deserialize<'de> for DelayedMessage {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            #[derive(Deserialize)]
            struct WireMessage {
                delay_ms: u64,
                id: u8,
            }

            let message = WireMessage::deserialize(deserializer)?;
            std::thread::sleep(Duration::from_millis(message.delay_ms));
            Ok(Self {
                delay_ms: message.delay_ms,
                id: message.id,
            })
        }
    }

    fn received_message_with_protocol(
        peer: PeerId,
        message: DelayedMessage,
        protocol: ProtocolId,
    ) -> ReceivedMessage {
        let raw_msg = protocol.to_bytes(&message).unwrap();
        ReceivedMessage::new(
            NetworkMessage::DirectSendMsg(DirectSendMsg {
                protocol_id: protocol,
                priority: 0,
                raw_msg,
            }),
            PeerNetworkId::new(NetworkId::Validator, peer),
        )
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn busy_peer_cannot_fill_all_deserialization_slots() {
        assert_peer_isolation(ProtocolId::ConsensusDirectSendBcs).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn consensus_observer_peers_are_isolated() {
        assert_peer_isolation(ProtocolId::ConsensusObserver).await;
    }

    async fn assert_peer_isolation(protocol: ProtocolId) {
        let attacker = PeerId::random();
        let honest_peer = PeerId::random();
        let (sender, receiver) = aptos_channel::new(QueueStyle::FIFO, 10, None);
        let mut events = NetworkEvents::<DelayedMessage>::new(receiver, Some(2), true);

        sender
            .push(
                (attacker, protocol),
                received_message_with_protocol(
                    attacker,
                    DelayedMessage {
                        delay_ms: 250,
                        id: 1,
                    },
                    protocol,
                ),
            )
            .unwrap();
        sender
            .push(
                (attacker, protocol),
                received_message_with_protocol(
                    attacker,
                    DelayedMessage {
                        delay_ms: 250,
                        id: 2,
                    },
                    protocol,
                ),
            )
            .unwrap();

        // Poll once so the first attacker message starts deserializing before
        // the honest message arrives. The second must remain queued.
        let next_event = events.next();
        tokio::pin!(next_event);
        tokio::select! {
            event = &mut next_event => panic!("slow attacker unexpectedly completed: {event:?}"),
            _ = tokio::time::sleep(Duration::from_millis(25)) => {},
        }

        sender
            .push(
                (honest_peer, protocol),
                received_message_with_protocol(
                    honest_peer,
                    DelayedMessage { delay_ms: 0, id: 3 },
                    protocol,
                ),
            )
            .unwrap();

        let event = tokio::time::timeout(Duration::from_millis(100), &mut next_event)
            .await
            .expect("honest peer should use the other deserialization slot")
            .expect("network event stream ended unexpectedly");
        assert_eq!(
            event,
            Event::Message(honest_peer, DelayedMessage { delay_ms: 0, id: 3 },)
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn peer_burst_is_not_dropped_during_deserialization() {
        for parallelism in [1, 2] {
            let peer = PeerId::random();
            let protocol = ProtocolId::ConsensusDirectSendBcs;
            let (sender, receiver) = aptos_channel::new(QueueStyle::FIFO, 50, None);
            let mut events =
                NetworkEvents::<DelayedMessage>::new(receiver, Some(parallelism), true);
            for id in 0..50 {
                sender
                    .push(
                        (peer, protocol),
                        received_message_with_protocol(
                            peer,
                            DelayedMessage { delay_ms: 5, id },
                            protocol,
                        ),
                    )
                    .unwrap();
            }
            drop(sender);
            let mut ids = Vec::new();
            tokio::time::timeout(Duration::from_secs(5), async {
                while let Some(event) = events.next().await {
                    let Event::Message(sender, message) = event else {
                        panic!("expected a direct-send message");
                    };
                    assert_eq!(sender, peer);
                    ids.push(message.id);
                }
            })
            .await
            .expect("queued messages should drain after the sender closes");
            assert_eq!(ids, (0..50).collect::<Vec<_>>());
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn consensus_observer_preserves_publisher_order() {
        let peer = PeerId::random();
        let protocol = ProtocolId::ConsensusObserver;
        let (sender, receiver) = aptos_channel::new(QueueStyle::FIFO, 10, None);
        let mut events = NetworkEvents::<DelayedMessage>::new(receiver, Some(2), true);
        for message in [
            DelayedMessage {
                delay_ms: 50,
                id: 1,
            },
            DelayedMessage { delay_ms: 0, id: 2 },
        ] {
            sender
                .push(
                    (peer, protocol),
                    received_message_with_protocol(peer, message, protocol),
                )
                .unwrap();
        }
        drop(sender);
        let messages = tokio::time::timeout(Duration::from_secs(2), async {
            let mut messages = Vec::new();
            while let Some(event) = events.next().await {
                messages.push(event);
            }
            messages
        })
        .await
        .expect("the stream should drain after the sender closes");
        assert_eq!(messages, vec![
            Event::Message(peer, DelayedMessage {
                delay_ms: 50,
                id: 1
            }),
            Event::Message(peer, DelayedMessage { delay_ms: 0, id: 2 }),
        ]);
    }
}
