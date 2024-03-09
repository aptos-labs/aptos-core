// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Convenience Network API for Aptos

pub use crate::protocols::RpcError;
use crate::{application::interface::{OutboundRpcMatcher, protocol_is_high_priority}, error::NetworkError, ProtocolId, counters};
use aptos_logger::prelude::*;
use aptos_types::{network_address::NetworkAddress, PeerId};
use bytes::Bytes;
use futures::{
    channel::oneshot,
    stream::{FusedStream, Stream, StreamExt},
    task::{Context, Poll},
};
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt::Debug, marker::PhantomData, pin::Pin, time::Duration};
use std::collections::{BTreeMap, HashMap};
use std::ops::Add;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::runtime::Handle;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use aptos_config::network_id::{NetworkContext, NetworkId, PeerNetworkId};
use crate::protocols::wire::messaging::v1::{DirectSendMsg, NetworkMessage, RequestId, RpcRequest, RpcResponse};
use hex::ToHex;
use aptos_config::config::RoleType;
use rand;

pub trait Message: DeserializeOwned + Serialize {}
impl<T: DeserializeOwned + Serialize> Message for T {}

/// Event is a templated typed wrapper around a Direct Message or an RPC Request.
///
/// This is the application side of incoming messages where they are converted from
/// network blobs to the local TMessage type (consensus, mempool, etc).
///
/// Historically there were also client connect/disconnect messages,
/// but those have moved to a subscription on the PeersAndMetadata object.
#[derive(Debug)]
pub enum Event<TMessage> {
    /// New inbound direct-send message from peer.
    Message(PeerNetworkId, TMessage),
    /// New inbound rpc request. The request is fulfilled by sending the
    /// serialized response `Bytes` over the `oneshot::Sender`, where the network
    /// layer will handle sending the response over-the-wire.
    RpcRequest(
        PeerNetworkId,
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

const DEFAULT_QUEUE_SIZE : usize = 1000;

#[derive(Clone)]
pub struct ApplicationProtocolConfig{
    pub protocol_id : ProtocolId,
    pub queue_size : usize,
}

/// Configuration needed for the service side of AptosNet applications
#[derive(Clone)]
pub struct NetworkApplicationConfig {
    /// Direct send protocols for the application (sorted by preference, highest to lowest)
    pub direct_send_protocols_and_preferences: Vec<ApplicationProtocolConfig>,
    /// RPC protocols for the application (sorted by preference, highest to lowest)
    pub rpc_protocols_and_preferences: Vec<ApplicationProtocolConfig>,
    /// Which networks do we want traffic from? [] for all.
    pub networks: Vec<NetworkId>,
}

fn default_protocol_configs(protocol_ids: Vec<ProtocolId>) -> Vec<ApplicationProtocolConfig> {
    protocol_ids.iter().map(|protocol_id| ApplicationProtocolConfig{protocol_id: *protocol_id, queue_size: DEFAULT_QUEUE_SIZE}).collect()
}

fn protocol_configs_for_queue_size(protocol_ids: Vec<ProtocolId>, queue_size: usize) -> Vec<ApplicationProtocolConfig> {
    protocol_ids.iter().map(|protocol_id| ApplicationProtocolConfig{protocol_id: *protocol_id, queue_size}).collect()
}

impl NetworkApplicationConfig {
    /// New NetworkApplicationConfig which talks to all peers on all networks
    pub fn new(
        direct_send_protocols_and_preferences: Vec<ProtocolId>,
        rpc_protocols_and_preferences: Vec<ProtocolId>,
    ) -> Self {
        Self {
            direct_send_protocols_and_preferences: default_protocol_configs(direct_send_protocols_and_preferences),
            rpc_protocols_and_preferences: default_protocol_configs(rpc_protocols_and_preferences),
            networks: vec![],
        }
    }
    /// New NetworkApplicationConfig which talks to peers only on selected networks
    pub fn new_for_networks(
        direct_send_protocols_and_preferences: Vec<ProtocolId>,
        rpc_protocols_and_preferences: Vec<ProtocolId>,
        networks: Vec<NetworkId>,
        queue_size: usize,
    ) -> Self {
        Self {
            direct_send_protocols_and_preferences: protocol_configs_for_queue_size(direct_send_protocols_and_preferences, queue_size),
            rpc_protocols_and_preferences: protocol_configs_for_queue_size(rpc_protocols_and_preferences, queue_size),
            networks,
        }
    }

    pub fn wants_network(&self, network: NetworkId) -> bool {
        self.networks.is_empty() || self.networks.contains(&network)
    }
}

#[derive(Debug,Clone,PartialEq)]
pub struct ReceivedMessage {
    pub message: NetworkMessage,
    pub sender: PeerNetworkId,

    // unix microseconds
    pub rx_at: u64,
}

impl ReceivedMessage {
    pub fn new(message: NetworkMessage, sender: PeerNetworkId) -> Self {
        let now = std::time::SystemTime::now();
        let rx_at = now.duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as u64;
        Self {
            message,
            sender,
            rx_at,
        }
    }
    pub fn protocol_id(&self) -> Option<ProtocolId> {
        match &self.message {
            NetworkMessage::Error(_e) => {
                None
            }
            NetworkMessage::RpcRequest(req) => {
                Some(req.protocol_id)
            }
            NetworkMessage::RpcResponse(_response) => {
                // design of RpcResponse lacking ProtocolId requires global rpc counter (or at least per-peer) and requires reply matching globally or per-peer
                None
            }
            NetworkMessage::DirectSendMsg(msg) => {
                Some(msg.protocol_id)
            }
        }
    }
    pub fn protocol_id_as_str(&self) -> &'static str {
        match &self.message {
            NetworkMessage::Error(_) => {"error"}
            NetworkMessage::RpcRequest(rr) => {rr.protocol_id.as_str()}
            NetworkMessage::RpcResponse(_) => {"rpc response"}
            NetworkMessage::DirectSendMsg(dm) => {dm.protocol_id.as_str()}
        }
    }
}

/// Trait specifying the signature for `new()` `NetworkEvents`
pub trait NewNetworkEvents {
    fn new(
        network_source: NetworkSource,
        peer_senders: Arc<OutboundPeerConnections>,
        label: &str,
        contexts: Arc<BTreeMap<NetworkId, NetworkContext>>,
    ) -> Self;
}

pub struct NetworkEvents<TMessage> {
    network_source: NetworkSource, //::sync::mpsc::Receiver<ReceivedMessage>,
    done: bool,

    peer_senders: Arc<OutboundPeerConnections>,
    peers: HashMap<PeerNetworkId, PeerStub>,
    peers_generation: u32,

    // TMessage is the type we will deserialize to
    phantom: PhantomData<TMessage>,

    // for metrics
    label: String,
    contexts: Arc<BTreeMap<NetworkId, NetworkContext>>,
}

impl<TMessage: Message + Unpin> NewNetworkEvents for NetworkEvents<TMessage> {
    fn new(
        network_source: NetworkSource,
        peer_senders: Arc<OutboundPeerConnections>,
        label: &str,
        contexts: Arc<BTreeMap<NetworkId, NetworkContext>>,
    ) -> Self {
        Self {
            network_source,
            done: false,
            peer_senders,
            peers: HashMap::new(),
            peers_generation: 0,
            phantom: Default::default(),
            label: label.to_string(),
            contexts,
        }
    }
}

/// Result from processing a ReceivedMessage
struct OutboundEvent<TMessage> {
    msg: Option<Event<TMessage>>,
    index: u32,
}

/// It's like a TCP window allowing for out of order packet arrival, sending when the 'next' packet is ready.
struct OutboundEventRingBuffer<TMessage> {
    event_tx: tokio::sync::mpsc::Sender<Event<TMessage>>,
    nthreads: u32,
    held_outputs: Vec<OutboundEvent<TMessage>>,
    next: u32,
}

impl<TMessage> OutboundEventRingBuffer<TMessage> {
    fn new(
        event_tx: tokio::sync::mpsc::Sender<Event<TMessage>>,
        nthreads: u32,
    ) -> Self {
        Self {
            event_tx,
            nthreads,
            held_outputs: Vec::with_capacity(nthreads as usize),
            next: 0,
        }
    }

    /// Put an event to send in sequence.
    /// Should put None for a message that was dropped.
    /// Returns number of messages sent to sink.
    async fn put(&mut self, msg: Option<Event<TMessage>>, index: u32) -> u32 {
        // Put into ring buffer with open slots for the next {nthreads} results.
        // Window moves forwards as front end at {next} is ready.
        // | next | next + 1 | next + 2 | ... | next + nthreads -1 |
        //
        // Send on the outbound queue only blocks if it is full, in which case doing more
        // work is pointless, so this is not a problem, just do blocking send().
        let mut sent : u32 = 0;
        let oi = (index % self.nthreads) as usize;
        if index == self.next {
            self.held_outputs[oi].index = self.next;
            // send it immediately
            if let Some(msg) = msg {
                self.event_tx.send(msg).await; // TODO: error handling
                sent += 1;
            }
            loop {
                self.next += 1;
                let oi = (self.next % self.nthreads) as usize;
                let oe = &mut self.held_outputs[oi];
                if oe.index == self.next {
                    // another ready element to send
                    if let Some(msg) = oe.msg.take() {
                        self.event_tx.send(msg).await; // TODO: error handling
                        sent += 1;
                    }
                } else {
                    return sent;
                }
            }
        }

        // this math should handle u32 overflow
        assert!(index - self.next < self.nthreads, "overflow, index {} > {} + {}", index, self.next, self.nthreads);

        self.held_outputs[oi] = OutboundEvent {
            msg,
            index,
        };
        return 0;
    }
}

fn process_received_message<TMessage: Message + Unpin + Send>(
    // out: Arc<Mutex<OutboundEventRingBuffer<TMessage>>>,
    wmsg: ReceivedMessage,
    // label: &'static str,
    label: String,
    sender_source: Arc<std::sync::Mutex<PeerSenderCache>>,
    // sender_source: &mut PeerSenderCache,
    // nc_source: fn(network_id: &NetworkId) -> NetworkContext,
    contexts: Arc<BTreeMap<NetworkId, NetworkContext>>,
) -> Option<Event<TMessage>> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as u64;
    let queue_micros = now - wmsg.rx_at;
    counters::inbound_queue_delay(wmsg.sender.network_id().as_str(), wmsg.protocol_id_as_str(), queue_micros);
    match wmsg.message {
        NetworkMessage::Error(err) => {
            // We just drop error responses! TODO: never send them
            // fall through, maybe get another
            info!("app_int err msg discarded: {:?}", err);
            //out.lock().await.put(None, index);
            None
        }
        NetworkMessage::RpcRequest(request) => {
            // let role_type = mself.contexts.get(&msg.sender.network_id()).unwrap().role();
            let role_type = "unk";
            // info!("app_int rpc_req {} id={}, {}b", request.protocol_id, request.request_id, request.raw_request.len());
            let request_received_start = tokio::time::Instant::now();
            let data_len = request.raw_request.len();
            // Multi-core decoding protocol_id.from_bytes() is left as an exercise to the application code.
            // The easiest approach is pipelining and introducing a short queue of decoded items inside the app, a thread continuously reads from this Stream and stores decoded objects in the app queue, this disconnects decoding from handling, allowing the app to do request handling work while a decode is happening in another thread.
            // See network_event_prefetch below.
            let app_msg = match request.protocol_id.from_bytes(request.raw_request.as_slice()) {
                Ok(x) => { x },
                Err(err) => {
                    // mself.done = true;
                    warn!("app_int rpc_req {} id={}; err {}, {} {} -> {}", request.protocol_id, request.request_id, err, label, request.raw_request.encode_hex_upper::<String>(), std::any::type_name::<TMessage>());
                    counters::rpc_message_bytes(wmsg.sender.network_id(), request.protocol_id.as_str(), role_type, counters::REQUEST_LABEL, counters::INBOUND_LABEL, "err", data_len as u64);
                    // TODO: BSC failed. log error, count error, close peer connection
                    // out.lock().await.put(None, index);
                    return None;
                }
            };
            let rpc_id = request.request_id;
            // setup responder oneshot channel
            let (responder, response_reader) = oneshot::channel();
            // let raw_sender = match sender_source.sender_for_peer(&wmsg.sender) {
            let raw_sender = match sender_source.lock().unwrap().sender_for_peer(&wmsg.sender) {
                None => {
                    // peer disconnected in the moment between receiving message and here?
                    warn!("app_int rpc_req no peer {} id={}", request.protocol_id, request.request_id);
                    counters::rpc_message_bytes(wmsg.sender.network_id(), request.protocol_id.as_str(), role_type, counters::REQUEST_LABEL, counters::INBOUND_LABEL, "err", data_len as u64);
                    // out.lock().await.put(None, index);
                    return None;
                }
                Some(x) => x,
            };
            // when this spawned task reads from the response oneshot channel it will send it to the network peer
            // let network_context = mself.contexts.get(&msg.sender.network_id()).unwrap();
            let network_context = contexts.get(&wmsg.sender.network_id()).unwrap();
            counters::pending_rpc_requests(request.protocol_id.as_str()).inc();
            Handle::current().spawn(rpc_response_sender(response_reader, rpc_id, raw_sender, request_received_start, *network_context, request.protocol_id));
            counters::rpc_message_bytes(wmsg.sender.network_id(), request.protocol_id.as_str(), role_type, counters::REQUEST_LABEL, counters::INBOUND_LABEL, "delivered", data_len as u64);
            let event = Event::RpcRequest(
                wmsg.sender, app_msg, request.protocol_id, responder);
            // out.lock().await.put(Some(event), index);
            Some(event)
        }
        NetworkMessage::RpcResponse(_) => {
            unreachable!("NetworkMessage::RpcResponse should not arrive in NetworkEvents because it is handled by Peer and reconnected with oneshot there");
        }
        NetworkMessage::DirectSendMsg(message) => {
            // Multi-core decoding protocol_id.from_bytes() is left as an exercise to the application code.
            // The easiest approach is pipelining and introducing a short queue of decoded items inside the app, a thread continuously reads from this Stream and stores decoded objects in the app queue, this disconnects decoding from handling, allowing the app to do request handling work while a decode is happening in another thread.
            // See network_event_prefetch below.
            match message.protocol_id.from_bytes(message.raw_msg.as_slice()) {
                Ok(app_msg) => {
                    // out.lock().await.put(Some(Event::Message(wmsg.sender, app_msg)), index);
                    // return Poll::Ready(Some(Event::Message(msg.sender, app_msg)));
                    Some(Event::Message(wmsg.sender, app_msg))
                },
                Err(_) => {
                    // TODO: BSC failed. log error, count error, close peer connection
                    // out.lock().await.put(None, index);
                    None
                }
            }
        }
    }
}

pub struct PeerSenderCache {
    peer_senders: Arc<OutboundPeerConnections>,
    pub peers: HashMap<PeerNetworkId, PeerStub>,
    peers_generation: u32,
}

impl PeerSenderCache {
    pub fn new(peer_senders: Arc<OutboundPeerConnections>) -> Self {
        Self {
            peer_senders,
            peers: HashMap::new(),
            peers_generation: 0,
        }
    }

    pub fn update_peers(&mut self) {
        if let Some((new_peers, new_generation)) = self.peer_senders.get_generational(self.peers_generation) {
            self.peers_generation = new_generation;
            self.peers.clear();
            self.peers.extend(new_peers);
        }
    }

    //sender_source: fn(peer_network_id: &PeerNetworkId) -> Option<tokio::sync::mpsc::Sender<(NetworkMessage,u64)>>,
    pub fn sender_for_peer(&mut self, peer_network_id: &PeerNetworkId) -> Option<tokio::sync::mpsc::Sender<(NetworkMessage,u64)>> {
        self.update_peers();
        self.peers.get(peer_network_id).map(|stub| stub.sender.clone())
    }
}

#[cfg(disabled)]
async fn parallel_deserialization_worker<TMessage: Message + Unpin + Send>(
    source: async_channel::Receiver<(ReceivedMessage,u32)>,
    sink: tokio::sync::mpsc::Sender<(Option<Event<TMessage>>,u32)>,
    label: String,
    // sender_source: Arc<std::sync::Mutex<PeerSenderCache>>,
    peer_senders: Arc<OutboundPeerConnections>,
    contexts: Arc<BTreeMap<NetworkId, NetworkContext>>,
) {
    let mut sender_source = PeerSenderCache::new(peer_senders);
    loop {
        match source.recv().await {
            Ok((msg, index)) => {
                let xm = process_received_message(msg, label.clone(), &mut sender_source, contexts.clone());
                if let Err(se) = sink.send((xm,index)).await {
                    error!("parallel_deserialization_worker send err: {:?}", se);
                    return;
                }
            }
            Err(err) => {
                error!("parallel_deserialization_worker exit: {:?}", err);
                return;
            }
        }
    }
}

async fn parallel_deserialization_collector<TMessage: Message + Unpin + Send>(
    mut source: tokio::sync::mpsc::Receiver<(Option<Event<TMessage>>,u32)>,
    releases: tokio::sync::mpsc::Sender<()>,
    sink: tokio::sync::mpsc::Sender<Event<TMessage>>,
    nthreads: u32,
) {
    let mut out = OutboundEventRingBuffer::new(sink, nthreads);
    loop {
        let (em,index) = match source.recv().await {
            None => {
                error!("parallel_deserialization_collector source None; exiting");
                return;
            }
            Some((em,index)) => (em,index)
        };
        let sent = out.put(em, index).await;
        if sent != 0 {
            for _i in 0..sent {
                releases.send(()).await;
            }
        }
    }
}

#[cfg(unused)]
/// network_event_prefetch when spawned as a new tokio task decouples BCS deserialization an object handling allowing for parallelization.
/// NetworkEvents Stream .next() does the work of BCS decode in this thread, another thread listening on the Receiver<Event<TMessage>> does the application work.
/// This is the solution I think I want, but Rust thinks I need to make "TMessage: 'static" and I think I don't want that -- bolson 2024-03-07 09:02:20 EST
pub async fn network_event_prefetch_parallel<TMessage: Message + Unpin + Send>(
    mut network_events: NetworkEvents<TMessage>,
    event_tx: tokio::sync::mpsc::Sender<Event<TMessage>>,
    nthreads: u32,
    handle: Handle,
) {
// ) -> tokio::sync::mpsc::Receiver<Event<TMessage>> {
    let (rm_sender, rm_receiver) = async_channel::bounded((nthreads*2) as usize);
    let (collect_sender, collect_receiver) = tokio::sync::mpsc::channel((nthreads*2) as usize);
    let (backpressure_sender, backpressure_receiver) = tokio::sync::mpsc::channel((nthreads*2) as usize);
    // let (event_tx, event_rx) = tokio::sync::mpsc::channel((nthreads*2) as usize);
    let peer_senders = network_events.peer_senders.clone();
    let label = network_events.label.clone();
    let contexts = network_events.contexts.clone();
    handle.spawn(network_events.parallel_deserialization_input_stage(rm_sender, backpressure_receiver));
    for _ in 0..nthreads {
        handle.spawn(parallel_deserialization_worker(rm_receiver.clone(), collect_sender.clone(), label.clone(), peer_senders.clone(), contexts.clone()));
    }
    handle.spawn(parallel_deserialization_collector(collect_receiver, backpressure_sender, event_tx, nthreads));
    // event_rx
}

#[cfg(disabled)]
async fn process_received_message_op<TMessage: Message + Unpin + Send>(
    // out: Arc<Mutex<OutboundEventRingBuffer<TMessage>>>,
    wmsg: ReceivedMessage,
    index: u32,
    // label: &'static str,
    label: String,
    // peers: fn () -> HashMap<PeerNetworkId, PeerStub>,
    // sender_source: fn(peer_network_id: &PeerNetworkId) -> Option<tokio::sync::mpsc::Sender<(NetworkMessage,u64)>>,
    sender_source: Arc<std::sync::Mutex<PeerSenderCache>>,
    // nc_source: fn(network_id: &NetworkId) -> NetworkContext,
    contexts: Arc<BTreeMap<NetworkId, NetworkContext>>,
    out : Arc<tokio::sync::Mutex<OutboundEventRingBuffer<TMessage>>>,
) {
    let xm = process_received_message(wmsg, label, sender_source, contexts);
    out.lock().await.put(xm, index).await
}

impl<TMessage: Message + Unpin + Send> NetworkEvents<TMessage> {
    fn update_peers(&mut self) {
        if let Some((new_peers, new_generation)) = self.peer_senders.get_generational(self.peers_generation) {
            self.peers_generation = new_generation;
            self.peers.clear();
            self.peers.extend(new_peers);
        }
    }

    fn sender_for_peer(&mut self, peer_network_id: &PeerNetworkId) -> Option<tokio::sync::mpsc::Sender<(NetworkMessage,u64)>> {
        self.update_peers();
        self.peers.get(peer_network_id).map(|stub| stub.sender.clone())
    }

    /// spawn() this at head of parallel deserialization tasks
    async fn parallel_deserialization_input_stage(
        mut self,
        sink: async_channel::Sender<(ReceivedMessage,u32)>,
        mut releases: tokio::sync::mpsc::Receiver<()>,
    ) {
        loop {
            let mut index : u32 = 0;
            // wait for backpressure release (which will be primed with N entries)
            if releases.recv().await.is_none() {
                error!("parallel_deserialization_input_stage releases.next is None, exiting.");
            }
            match self.network_source.next().await {
                Some(msg) => {
                    if let Err(err) = sink.send((msg,index)).await {
                        error!("parallel_deserialization_input_stage sink err (exiting): {:?}", err);
                        return;
                    }
                    index = index + 1;
                }
                None => {
                    error!("parallel_deserialization_input_stage network_source.next is None, exiting.");
                    return;
                }
            }
        }
    }

    /// spawn() this, takes ownerships of NetworkEvents
    async fn parallel_deserialization(
        mut self,
        sink: tokio::sync::mpsc::Sender<Event<TMessage>>,
        nthreads: usize,
    ) {
        let label = self.label.clone();
        let contexts = self.contexts.clone();
        let sender_source = Arc::new(std::sync::Mutex::new(PeerSenderCache::new(self.peer_senders)));
        self.network_source.for_each_concurrent(
            nthreads,
            |msg| async {
                let sink = sink.clone();
                let xm = process_received_message(msg, label.clone(), sender_source.clone(), contexts.clone());
                if let Some(em) = xm {
                    sink.send(em).await; // TODO: error checking
                }
            }
        ).await;
    }

    #[cfg(disabled)]
    async fn wat(
        &mut self,
        // nthreads: u32,
        // event_tx: tokio::sync::mpsc::Sender<Event<TMessage>>,
        out : Arc<tokio::sync::Mutex<OutboundEventRingBuffer<TMessage>>>,
        peer_senders: Arc<OutboundPeerConnections>
    ) {
        let mut next_receive_msg_index : u32 = 0;
        let handle = Handle::current();
        // let out = Arc::new(Mutex::new(OutboundEventRingBuffer::new(event_tx, nthreads)));
        let peers = Arc::new(std::sync::Mutex::new(PeerSenderCache::new(peer_senders)));

        loop {
            let msg = match self.network_source.next().await {
                None => {
                    // Done!
                    error!(
                        app = self.label,
                        "stream to app closed"
                    );
                    return
                },
                Some(x) => x,
            };
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as u64;
            let queue_micros = now - msg.rx_at;
            counters::inbound_queue_delay(msg.sender.network_id().as_str(), msg.protocol_id_as_str(), queue_micros);
            let index = next_receive_msg_index;
            handle.spawn(process_received_message_op(msg, index, self.label.clone(), peers.clone(), self.contexts.clone(), out.clone()));
            // handle.spawn(|| async {
            //     let (xm, ui) = process_received_message(msg, index, self.label.clone(), peers.clone(), self.contexts.clone());
            //     out.lock().unwrap().put(xm, ui)
            // });
            next_receive_msg_index = next_receive_msg_index + 1;
        }
    }

    #[cfg(disabled)]
    async fn w2(
        &mut self,
        event_tx: tokio::sync::mpsc::Sender<Event<TMessage>>,
        peer_senders: Arc<OutboundPeerConnections>,
    ) {
        let peers = Arc::new(Mutex::new(PeerSenderCache::new(peer_senders)));
        // ERROR: for_each_concurrent is not order preserving
        let x = self.network_source.for_each_concurrent(
            4,
            |msg| {
                let (xm,_u) = process_received_message(msg, 0, self.label.clone(), peers.clone(), self.contexts.clone());
                if let(Some(xm)) = xm {
                    event_tx.clone().send(xm).await;
                }
            }).await;
    }
}

async fn rpc_response_sender(
    receiver: oneshot::Receiver<Result<Bytes, RpcError>>,
    rpc_id: RequestId,
    peer_sender: tokio::sync::mpsc::Sender<(NetworkMessage,u64)>,
    request_received_start: tokio::time::Instant, // TODO: use TimeService
    network_context: NetworkContext, // for metrics
    protocol_id: ProtocolId, // for metrics
) {
    let bytes = match receiver.await {
        Ok(iresult) => match iresult{
            Ok(bytes) => {bytes}
            Err(_) => {
                counters::rpc_messages(network_context.network_id(),protocol_id.as_str(),network_context.role(),counters::RESPONSE_LABEL,counters::OUTBOUND_LABEL,"err").inc();
                counters::pending_rpc_requests(protocol_id.as_str()).dec();
                return;
            }
        }
        Err(_) => {
            counters::rpc_messages(network_context.network_id(),protocol_id.as_str(),network_context.role(),counters::RESPONSE_LABEL,counters::OUTBOUND_LABEL,counters::CANCELED_LABEL).inc();
            counters::pending_rpc_requests(protocol_id.as_str()).dec();
            return;
        }
    };
    let now = tokio::time::Instant::now();
    let data_len = bytes.len();
    let msg = NetworkMessage::RpcResponse(RpcResponse{
        request_id: rpc_id,
        priority: 0,
        raw_response: bytes.into(),
    });
    let dt = now.duration_since(request_received_start);
    let enqueue_micros = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as u64;
    match peer_sender.send((msg,enqueue_micros)).await {
        Ok(_) => {
            counters::inbound_rpc_handler_latency(&network_context, protocol_id).observe(dt.as_secs_f64());
            counters::rpc_message_bytes(network_context.network_id(),protocol_id.as_str(),network_context.role().as_str(),counters::RESPONSE_LABEL,counters::OUTBOUND_LABEL,counters::SENT_LABEL, data_len as u64);
            counters::pending_rpc_requests(protocol_id.as_str()).dec();
        }
        Err(_) => {
            counters::rpc_message_bytes(network_context.network_id(),protocol_id.as_str(),network_context.role().as_str(),counters::RESPONSE_LABEL,counters::OUTBOUND_LABEL,"peererr", data_len as u64);
            counters::pending_rpc_requests(protocol_id.as_str()).dec();
        }
    }
}

impl<TMessage: Message + Unpin + Send> Stream for NetworkEvents<TMessage> {
    type Item = Event<TMessage>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // this is the stream of events from all peers to the application.
        // It should never return Poll::Ready(None) which signals that the stream is closed.
        // It should find out-of-band non-Stream ways to push back that a message from a peer was bad.
        // This Stream.poll_next() should only ever return Poll::Ready(Some(...)) or Poll::Pending)
        if self.done {
            // see excption 10 lines below...
            info!("app_int poll still DONE");
            return Poll::Ready(None);
        }
        let mself = self.get_mut();
        // throw away up to 10 messages while looking for one to return
        'retries: for _ in 1..10 {
            let msg = match Pin::new(&mut mself.network_source).poll_next(cx) {
                Poll::Ready(x) => match x {
                    Some(msg) => {
                        msg
                    }
                    None => {
                        // this is the exception to the preamble comment, but it is also an error that the underlying stream closed, that shouldn't happen.
                        error!("app_int poll DONE");
                        mself.done = true;
                        return Poll::Ready(None);
                    }
                }
                Poll::Pending => {
                    return Poll::Pending
                }
            };
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as u64;
            let queue_micros = now - msg.rx_at;
            counters::inbound_queue_delay(msg.sender.network_id().as_str(), msg.protocol_id_as_str(), queue_micros);
            match msg.message {
                NetworkMessage::Error(err) => {
                    // We just drop error responses! TODO: never send them
                    // fall through, maybe get another
                    info!("app_int err msg discarded: {:?}", err);
                }
                NetworkMessage::RpcRequest(request) => {
                    let role_type = mself.contexts.get(&msg.sender.network_id()).unwrap().role();
                    // info!("app_int rpc_req {} id={}, {}b", request.protocol_id, request.request_id, request.raw_request.len());
                    let request_received_start = tokio::time::Instant::now();
                    let data_len = request.raw_request.len();
                    // Multi-core decoding protocol_id.from_bytes() is left as an exercise to the application code.
                    // The easiest approach is pipelining and introducing a short queue of decoded items inside the app, a thread continuously reads from this Stream and stores decoded objects in the app queue, this disconnects decoding from handling, allowing the app to do request handling work while a decode is happening in another thread.
                    // See network_event_prefetch below.
                    let app_msg = match request.protocol_id.from_bytes(request.raw_request.as_slice()) {
                        Ok(x) => { x },
                        Err(err) => {
                            mself.done = true;
                            warn!("app_int rpc_req {} id={}; err {}, {} {} -> {}; from {:?}", request.protocol_id, request.request_id, err, mself.label, request.raw_request.encode_hex_upper::<String>(), std::any::type_name::<TMessage>(), &mself.network_source);
                            counters::rpc_message_bytes(msg.sender.network_id(), request.protocol_id.as_str(), role_type.as_str(), counters::REQUEST_LABEL, counters::INBOUND_LABEL, "err", data_len as u64);
                            // TODO: BSC failed. log error, count error, close peer connection
                            continue 'retries;
                        }
                    };
                    let rpc_id = request.request_id;
                    // setup responder oneshot channel
                    let (responder, response_reader) = oneshot::channel();
                    mself.update_peers();
                    let raw_sender = match mself.peers.get(&msg.sender) {
                        None => {
                            // peer disconnected in the moment between receiving message and here?
                            warn!("app_int rpc_req no peer {} id={}", request.protocol_id, request.request_id);
                            counters::rpc_message_bytes(msg.sender.network_id(), request.protocol_id.as_str(), role_type.as_str(), counters::REQUEST_LABEL, counters::INBOUND_LABEL, "err", data_len as u64);
                            continue 'retries;
                        }
                        Some(peer_stub) => {
                            peer_stub.sender.clone()
                        }
                    };
                    // when this spawned task reads from the response oneshot channel it will send it to the network peer
                    let network_context = mself.contexts.get(&msg.sender.network_id()).unwrap();
                    counters::pending_rpc_requests(request.protocol_id.as_str()).inc();
                    Handle::current().spawn(rpc_response_sender(response_reader, rpc_id, raw_sender, request_received_start, *network_context, request.protocol_id));
                    counters::rpc_message_bytes(msg.sender.network_id(), request.protocol_id.as_str(), role_type.as_str(), counters::REQUEST_LABEL, counters::INBOUND_LABEL, "delivered", data_len as u64);
                    return Poll::Ready(Some(Event::RpcRequest(
                        msg.sender, app_msg, request.protocol_id, responder)))
                }
                NetworkMessage::RpcResponse(_) => {
                    unreachable!("NetworkMessage::RpcResponse should not arrive in NetworkEvents because it is handled by Peer and reconnected with oneshot there");
                }
                NetworkMessage::DirectSendMsg(message) => {
                    // Multi-core decoding protocol_id.from_bytes() is left as an exercise to the application code.
                    // The easiest approach is pipelining and introducing a short queue of decoded items inside the app, a thread continuously reads from this Stream and stores decoded objects in the app queue, this disconnects decoding from handling, allowing the app to do request handling work while a decode is happening in another thread.
                    // See network_event_prefetch below.
                    let app_msg = match message.protocol_id.from_bytes(message.raw_msg.as_slice()) {
                        Ok(app_msg) => {
                            return Poll::Ready(Some(Event::Message(msg.sender, app_msg)));
                        },
                        Err(_) => {
                            // TODO: BSC failed. log error, count error, close peer connection
                            // fall out to fetching another message from the underlying stream
                        }
                    };
                }
            }
        }
        Poll::Pending
    }
}

impl<TMessage: Message + Unpin + Send> FusedStream for NetworkEvents<TMessage> {
    fn is_terminated(&self) -> bool {
        self.done
    }
}

/// network_event_prefetch when spawned as a new tokio task decouples BCS deserialization an object handling allowing for parallelization.
/// NetworkEvents Stream .next() does the work of BCS decode in this thread, another thread listening on the Receiver<Event<TMessage>> does the application work.
pub async fn network_event_prefetch<TMessage: Message + Unpin + Send>(
    mut network_events: NetworkEvents<TMessage>,
    event_tx: tokio::sync::mpsc::Sender<Event<TMessage>>,
) {
    loop {
        let mut network_events = Pin::new(&mut network_events);
        match network_events.next().await {
            Some(msg) => {
                let result = event_tx.send(msg).await;
                if result.is_err() {
                    return;
                }
            },
            None => {return;},
        };
    }
}

pub async fn network_event_prefetch_parallel<TMessage: Message + Unpin + Send>(
    mut network_events: NetworkEvents<TMessage>,
    event_tx: tokio::sync::mpsc::Sender<Event<TMessage>>,
    nthreads: usize,
) {
    network_events.parallel_deserialization(event_tx, nthreads).await;
}

/// `NetworkSender` is the generic interface from upper network applications to
/// the lower network layer. It provides the full API for network applications,
/// including sending direct-send messages, sending rpc requests, as well as
/// dialing or disconnecting from peers and updating the list of accepted public
/// keys.
#[derive(Debug)]
pub struct NetworkSender<TMessage> {
    network_id: NetworkId,
    peer_senders: Arc<OutboundPeerConnections>,

    peers: RwLock<HashMap<PeerNetworkId, PeerStub>>,
    peers_generation: AtomicU32,
    role_type: RoleType, // for metrics
    _marker: PhantomData<TMessage>,
}

/// Trait specifying the signature for `new()` `NetworkSender`s
pub trait NewNetworkSender {
    fn new(
        network_id: NetworkId,
        peer_senders: Arc<OutboundPeerConnections>,
        role_type: RoleType,
    ) -> Self;
}

impl<TMessage> NewNetworkSender for NetworkSender<TMessage> {
    fn new(
        network_id: NetworkId,
        peer_senders: Arc<OutboundPeerConnections>,
        role_type: RoleType,
    ) -> Self {
        Self {
            network_id,
            peer_senders,
            peers: RwLock::new(HashMap::new()),
            peers_generation: AtomicU32::new(0),
            role_type,
            _marker: PhantomData,
        }
    }
}

impl<TMessage> Clone for NetworkSender<TMessage> {
    fn clone(&self) -> Self {
        let reader = self.peers.read().unwrap();
        let peers_generation = self.peers_generation.load(Ordering::SeqCst);
        let peers = reader.clone();
        Self{
            network_id: self.network_id,
            peer_senders: self.peer_senders.clone(),
            peers: RwLock::new(peers),
            peers_generation: AtomicU32::new(peers_generation),
            role_type: self.role_type,
            _marker: PhantomData,
        }
    }
}

impl<TMessage> NetworkSender<TMessage> {
    /// Request that a given Peer be dialed at the provided `NetworkAddress` and
    /// synchronously wait for the request to be performed.
    pub async fn dial_peer(&self, _peer: PeerId, _addr: NetworkAddress) -> Result<(), NetworkError> {
        // dead network-1 code we might want to bring back...
        unreachable!("NetworkSender.dial_peer unimplemented (and unused)");
        // Ok(())
    }

    /// Request that a given Peer be disconnected and synchronously wait for the request to be
    /// performed.
    pub async fn disconnect_peer(&self, _peer: PeerId) -> Result<(), NetworkError> {
        // dead network-1 code we might want to bring back...
        unreachable!("NetworkSender.disconnect_peer unimplemented (and unused)");
        // Ok(())
    }

    fn update_peers(&self) {
        let cur_generation = self.peers_generation.load(Ordering::SeqCst);
        if let Some((new_peers, new_generation)) = self.peer_senders.get_generational(cur_generation) {
            let mut writer = self.peers.write().unwrap();
            self.peers_generation.store(new_generation, Ordering::SeqCst);
            writer.clear();
            writer.extend(new_peers);
            // writer.deref_mut().from_iter(new_peers);
        }
    }
}

impl<TMessage: Message + Send + 'static> NetworkSender<TMessage> {
    /// Send a message to a single recipient.
    fn peer_try_send<F>(&self, peer_network_id: &PeerNetworkId, msg_src: F, high_prio: bool) -> Result<(), NetworkError>
    where F: Fn() -> Result<NetworkMessage,NetworkError>
    {
        match self.peers.read() {
            Ok(peers) => {
                match peers.get(peer_network_id) {
                    None => {
                        // we _could_ know the protocol_id here, but it would be _work_ we'd just throw away.
                        counters::direct_send_message_bytes(self.network_id, "unk", self.role_type, "peergone", 0);
                        Err(NetworkError::NotConnected)
                    }
                    Some(peer) => {
                        let msg = msg_src()?;
                        let protocol_id_str = msg.protocol_id_as_str();
                        let data_len = msg.data_len() as u64;
                        let enqueue_micros = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as u64;
                        let msg = (msg,enqueue_micros);
                        let send_result = if high_prio {
                            peer.sender_high_prio.try_send(msg)
                        } else {
                            peer.sender.try_send(msg)
                        };
                        match send_result {
                            Ok(_) => {
                                counters::direct_send_message_bytes(self.network_id, protocol_id_str, self.role_type, counters::SENT_LABEL, data_len);
                                Ok(())
                            }
                            Err(tse) => match &tse {
                                TrySendError::Full(_) => {
                                    counters::direct_send_message_bytes(self.network_id, protocol_id_str, self.role_type, "peerfull", data_len);
                                    Err(NetworkError::PeerFullCondition)
                                }
                                TrySendError::Closed(_) => {
                                    counters::direct_send_message_bytes(self.network_id, protocol_id_str, self.role_type, "peergone", data_len);
                                    Err(NetworkError::NotConnected)
                                }
                            }
                        }
                    }
                }
            }
            Err(lf_err) => {
                // lock fail, wtf
                Err(NetworkError::Error(lf_err.to_string()))
            }
        }
    }

    /// blocking async version of peer_try_send
    async fn peer_send<F>(&self, peer_network_id: &PeerNetworkId, msg_src: F, high_prio: bool) -> Result<(), NetworkError>
        where F: Fn() -> Result<NetworkMessage,NetworkError>
    {
        let sender = match self.peers.read() {
            Ok(peers) => {
                match peers.get(peer_network_id) {
                    None => {
                        // we _could_ know the protocol_id here, but it would just be work thrown away.
                        counters::direct_send_message_bytes(self.network_id, "unk", self.role_type, "peergone", 0);
                        return Err(NetworkError::NotConnected);
                    }
                    Some(peer) => {
                        if high_prio {
                            peer.sender_high_prio.clone()
                        } else {
                            peer.sender.clone()
                        }
                    }
                }
            }
            Err(lf_err) => {
                // lock fail, wtf
                return Err(NetworkError::Error(lf_err.to_string()));
            }
        };
        let msg = msg_src()?;
        let protocol_id_str = msg.protocol_id_as_str();
        let data_len = msg.data_len() as u64;
        // let handle = Handle::current();
        let enqueue_micros = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as u64;
        match sender.send((msg,enqueue_micros)).await {
            Ok(_) => {
                counters::direct_send_message_bytes(self.network_id, protocol_id_str, self.role_type, counters::SENT_LABEL, data_len);
                Ok(())
            }
            Err(_send_err) => {
                // at this time the only SendError is a not-connected error (the other end of the queue is closed)
                Err(NetworkError::NotConnected)
            }
        }
    }

    /// Send a protobuf message to a single recipient. Provides a wrapper over
    /// `[peer_manager::PeerManagerRequestSender::send_to]`.
    pub fn send_to(
        &self,
        recipient: PeerId,
        protocol: ProtocolId,
        message: TMessage,
    ) -> Result<(), NetworkError> {
        let peer_network_id = PeerNetworkId::new(self.network_id, recipient);
        let msg_src = || -> Result<NetworkMessage,NetworkError> {
            // TODO: detach .send_to() into its own thread in app code that cares to continue on instead of waiting for protocol.to_bytes()
            // let start = Instant::now();
            let mdata : Vec<u8> = protocol.to_bytes(&message)?;
            // let dt = Instant::now().duration_since(start);
            // counters::bcs_encode_count(mdata.len(), dt);
            Ok(NetworkMessage::DirectSendMsg(DirectSendMsg {
                protocol_id: protocol,
                priority: 0,
                raw_msg: mdata,
            }))
        };
        self.update_peers();
        self.peer_try_send(&peer_network_id, msg_src, protocol_is_high_priority(protocol))
    }


    /// Send a message to a single recipient.
    /// Use async framework, block local thread until sent or err.
    pub async fn send_async(
        &self,
        recipient: PeerId,
        protocol: ProtocolId,
        message: TMessage,
    ) -> Result<(), NetworkError> {
        let peer_network_id = PeerNetworkId::new(self.network_id, recipient);
        let msg_src = || -> Result<NetworkMessage,NetworkError> {
            // TODO: detach .send_async() into its own thread in app code that cares to continue on instead of waiting for protocol.to_bytes()
            let mdata = protocol.to_bytes(&message)?;
            Ok(NetworkMessage::DirectSendMsg(DirectSendMsg {
                protocol_id: protocol,
                priority: 0,
                raw_msg: mdata,
            }))
        };
        self.update_peers();
        self.peer_send(&peer_network_id, msg_src, protocol_is_high_priority(protocol)).await
    }

    /// Send a message to a many recipients.
    pub fn send_to_many(
        &self,
        recipients: impl Iterator<Item = PeerId>,
        protocol: ProtocolId,
        message: TMessage,
    ) -> Result<(), NetworkError> {
        // TODO: detach .send_to_many() into its own thread in app code that cares to continue on instead of waiting for protocol.to_bytes()
        let mdata : Vec<u8> = protocol.to_bytes(&message)?;
        let msg_src = || -> Result<NetworkMessage,NetworkError> {
            Ok(NetworkMessage::DirectSendMsg(DirectSendMsg {
                protocol_id: protocol,
                priority: 0,
                raw_msg: mdata.clone(),
            }))
        };
        self.update_peers();
        let mut errs = vec![];
        for recipient in recipients {
            let peer_network_id = PeerNetworkId::new(self.network_id, recipient);
            match self.peer_try_send(&peer_network_id, msg_src, protocol_is_high_priority(protocol)) {
                Ok(_) => {}
                Err(xe) => {errs.push(xe)}
            }
        }
        if errs.is_empty() {
            Ok(())
        } else {
            // return first error. TODO: return summary or concatenation of all errors
            Err(errs.into_iter().nth(0).unwrap())
        }
    }

    /// Send a rpc request to a single recipient while handling
    /// serialization and deserialization of the request and response respectively.
    /// Assumes that the request and response both have the same message type.
    pub async fn send_rpc(
        &self,
        recipient: PeerId,
        protocol: ProtocolId,
        req_msg: TMessage,
        timeout: Duration,
        high_prio: bool,
    ) -> Result<TMessage, RpcError> {
        // Don't use `?` error return so that we can put a counter on every error condition
        // TODO: plumb through a TimeService to here
        let now = tokio::time::Instant::now();
        let deadline = now.add(timeout);
        let peer_network_id = PeerNetworkId::new(self.network_id, recipient);
        self.update_peers();
        // This holds a read-lock on the application's local cache of the peer map for a little while.
        // The big part is probably serialization in protocol.to_bytes().
        // peer.sender.try_send() should either accept or fail quickly.
        let receiver = match self.peers.read() {
            Ok(peers) => {
                match peers.get(&peer_network_id) {
                    None => {
                        counters::rpc_messages(self.network_id, protocol.as_str(), self.role_type, counters::REQUEST_LABEL, counters::OUTBOUND_LABEL, "peergone").inc();
                        return Err(RpcError::NotConnected(recipient));
                    }
                    Some(peer) => {
                        // TODO: detach .send_rpc() into its own thread in app code that cares to continue on instead of waiting for protocol.to_bytes()
                        // Serialize the request using a blocking task
                        // let mdata: Vec<u8> = match tokio::task::spawn_blocking(move || protocol.to_bytes(&req_msg)).await {
                        let mdata: Vec<u8> = match protocol.to_bytes(&req_msg) {
                            Ok(x) => {x}
                            Err(err) => {
                                counters::rpc_messages(self.network_id, protocol.as_str(), self.role_type, counters::REQUEST_LABEL, counters::OUTBOUND_LABEL, "bcserr").inc();
                                return Err(err.into());
                            }
                        };
                        let data_len = mdata.len() as u64;
                        let request_id = peer.rpc_counter.fetch_add(1, Ordering::SeqCst);
                        let msg = NetworkMessage::RpcRequest(RpcRequest {
                            protocol_id: protocol,
                            request_id,
                            priority: 0,
                            raw_request: mdata,
                        });

                        let (sender, receiver) = oneshot::channel();
                        peer.open_outbound_rpc.insert(request_id, sender, protocol, now, deadline, self.network_id, self.role_type);
                        let enqueue_micros = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as u64;
                        let msg = (msg,enqueue_micros);
                        let send_result = if high_prio {
                            peer.sender_high_prio.try_send(msg)
                        } else {
                            peer.sender.try_send(msg)
                        };
                        match send_result {
                            Ok(_) => {
                                counters::rpc_message_bytes(self.network_id, protocol.as_str(), self.role_type.as_str(), counters::REQUEST_LABEL, counters::OUTBOUND_LABEL, counters::SENT_LABEL, data_len);
                                receiver
                                // now we wait for rpc reply
                            }
                            Err(tse) => match &tse {
                                TrySendError::Full(_) => {
                                    counters::rpc_message_bytes(self.network_id, protocol.as_str(), self.role_type.as_str(), counters::REQUEST_LABEL, counters::OUTBOUND_LABEL, "peerfull", data_len);
                                    return Err(RpcError::TooManyPending(1)); // TODO: look up the channel size and return that many pending?
                                }
                                TrySendError::Closed(_) => {
                                    counters::rpc_message_bytes(self.network_id, protocol.as_str(), self.role_type.as_str(), counters::REQUEST_LABEL, counters::OUTBOUND_LABEL, "peergone", data_len);
                                    return Err(RpcError::NotConnected(recipient));
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => {
                counters::rpc_messages(self.network_id, protocol.as_str(), self.role_type, counters::REQUEST_LABEL, counters::OUTBOUND_LABEL, "wat").inc();
                return Err(RpcError::TimedOut); // TODO: better error for lock fail?
            }
        };
        let sub_timeout = match deadline.checked_duration_since(tokio::time::Instant::now()) {
            None => {
                // we are already past deadline, just sending was too slow?
                counters::rpc_messages(self.network_id, protocol.as_str(), self.role_type, counters::RESPONSE_LABEL, counters::INBOUND_LABEL, "timeout0").inc();
                return Err(RpcError::TimedOut);
            }
            Some(sub) => {sub}
        };
        match tokio::time::timeout(sub_timeout, receiver).await {
            Ok(receiver_result) => match receiver_result {
                Ok(content_result) => match content_result {
                    Ok((bytes, rx_time)) => {
                        let data_len = bytes.len() as u64;
                        let endtime = tokio::time::Instant::now();
                        // time message spent waiting in queue
                        let delay_micros = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as u64) - rx_time;
                        // time since this send_rpc() started
                        let rpc_micros = endtime.duration_since(now).as_micros() as u64;
                        counters::inbound_queue_delay(self.network_id.as_str(), protocol.as_str(), delay_micros);
                        counters::rpc_message_bytes(self.network_id, protocol.as_str(), self.role_type.as_str(), counters::RESPONSE_LABEL, counters::INBOUND_LABEL, "delivered", data_len);
                        counters::outbound_rpc_request_api_latency(self.network_id, protocol).observe((rpc_micros as f64) / 1_000_000.0);

			            let wat = match tokio::task::spawn_blocking(move || protocol.from_bytes(bytes.as_ref())).await {
                        //let wat = match protocol.from_bytes(bytes.as_ref()) {
                            Ok(x) => match x {
                                Ok(y) => { y },
                                Err(yerr) => {
                                    counters::rpc_messages(self.network_id, protocol.as_str(), self.role_type, counters::RESPONSE_LABEL, counters::INBOUND_LABEL, "bcserr").inc();
                                    return Err(yerr.into());
                                },
                            },
                            Err(err) => {
                                counters::rpc_messages(self.network_id, protocol.as_str(), self.role_type, counters::RESPONSE_LABEL, counters::INBOUND_LABEL, "bcserr").inc();
                                return Err(err.into());
                            },
                        };
                        Ok(wat)
                    }
                    Err(err) => {
                        counters::rpc_messages(self.network_id, protocol.as_str(), self.role_type, counters::RESPONSE_LABEL, counters::INBOUND_LABEL, counters::FAILED_LABEL).inc();
                        Err(err)
                    }
                }
                Err(_) => {
                    counters::rpc_messages(self.network_id, protocol.as_str(), self.role_type, counters::RESPONSE_LABEL, counters::INBOUND_LABEL, counters::FAILED_LABEL).inc();
                    Err(RpcError::UnexpectedResponseChannelCancel)
                }
            }
            Err(_timeout) => {
                // timeout waiting for a response
                counters::rpc_messages(self.network_id, protocol.as_str(), self.role_type, counters::RESPONSE_LABEL, counters::INBOUND_LABEL, "timeout").inc();
                Err(RpcError::TimedOut)
            }
        }
        // end of function where there should be no ? return
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

// tokio::sync::mpsc::Receiver<ReceivedMessage>,

#[derive(Debug)]
enum NetworkSourceUnion {
    SingleSource(tokio_stream::wrappers::ReceiverStream<ReceivedMessage>),
    ManySource(futures::stream::SelectAll<ReceiverStream<ReceivedMessage>>),
}

#[derive(Debug)]
pub struct NetworkSource {
    source: NetworkSourceUnion,
}

/// NetworkSource implements Stream<ReceivedMessage> and is all messages routed to this application from all peers
impl NetworkSource {
    pub fn new_single_source(network_source: tokio::sync::mpsc::Receiver<ReceivedMessage>) -> Self {
        let network_source = ReceiverStream::new(network_source);
        Self {
            source: NetworkSourceUnion::SingleSource(network_source),
        }
    }

    pub fn new_multi_source(receivers: Vec<tokio::sync::mpsc::Receiver<ReceivedMessage>>) -> Self {
        Self {
            source: NetworkSourceUnion::ManySource(merge_receivers(receivers))
        }
    }
}


fn merge_receivers(receivers: Vec<tokio::sync::mpsc::Receiver<ReceivedMessage>>) -> futures::stream::SelectAll<ReceiverStream<ReceivedMessage>> {
    futures::stream::select_all(
        receivers.into_iter().map(tokio_stream::wrappers::ReceiverStream::new)
    )
}

impl Stream for NetworkSource {
    type Item = ReceivedMessage;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<Option<Self::Item>> {
        match &mut self.get_mut().source {
            NetworkSourceUnion::SingleSource(rs) => { Pin::new(rs).poll_next(cx) }
            NetworkSourceUnion::ManySource(sa) => { Pin::new(sa).poll_next(cx) }
        }
    }
}

/// Closer someone replicates Go Context.Done() or a Mutex+Condition variable
#[derive(Clone,Debug)]
pub struct Closer {
    pub wat: Arc<tokio::sync::Mutex<tokio::sync::watch::Sender<bool>>>,
    pub done: tokio::sync::watch::Receiver<bool>,
}

impl Closer {
    pub fn new() -> Self {
        let (sender, receiver) = tokio::sync::watch::channel(false);
        Self {
            wat: Arc::new(tokio::sync::Mutex::new(sender)),
            done: receiver,
        }
    }

    /// wait for exit. ignores errors.
    pub async fn wait(&mut self) {
        _ = self.done.wait_for(|x| *x).await;
    }

    pub async fn close(&self) {
        self.wat.lock().await.send_modify(|x| *x = true);
    }

    pub fn is_closed(&self) -> bool {
        *self.done.borrow()
    }
}

impl Default for Closer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct PeerStub {
    /// channel to Peer's write thread
    pub sender: tokio::sync::mpsc::Sender<(NetworkMessage,u64)>,
    pub sender_high_prio: tokio::sync::mpsc::Sender<(NetworkMessage,u64)>,
    pub rpc_counter: Arc<AtomicU32>,
    pub close: Closer,
    open_outbound_rpc: OutboundRpcMatcher,
}

impl PeerStub {
    pub fn new(
        sender: tokio::sync::mpsc::Sender<(NetworkMessage,u64)>,
        sender_high_prio: tokio::sync::mpsc::Sender<(NetworkMessage,u64)>,
        open_outbound_rpc: OutboundRpcMatcher,
        close: Closer,
    ) -> Self {
        Self {
            sender,
            sender_high_prio,
            rpc_counter: Arc::new(AtomicU32::new(rand::random())),
            close,
            open_outbound_rpc,
        }
    }
}

/// Container for map of PeerNetworkId and associated outbound channels.
/// Generational fetch for fast no-change path. Local map copy can then operate lock free.
#[derive(Debug)]
pub struct OutboundPeerConnections {
    peer_connections: RwLock<HashMap<PeerNetworkId, PeerStub>>,
    generation: AtomicU32,
}

impl OutboundPeerConnections {
    pub fn new() -> Self {
        Self{
            peer_connections: RwLock::new(HashMap::new()),
            generation: AtomicU32::new(0),
        }
    }

    /// pass in a generation number, if it is stale return new peer map and current generation, otherwise None
    pub fn get_generational(&self, generation: u32) -> Option<(HashMap<PeerNetworkId,PeerStub>, u32)> {
        let generation_test = self.generation.load(Ordering::SeqCst);
        if generation == generation_test {
            return None;
        }
        let read = self.peer_connections.read().unwrap();
        let generation_actual = self.generation.load(Ordering::SeqCst);
        let out = read.clone();
        Some((out, generation_actual))
    }

    /// set a (PeerNetworkId, PeerStub) pair
    /// return new generation counter
    pub fn insert(&self, peer_network_id: PeerNetworkId, peer: PeerStub) -> u32 {
        let mut write = self.peer_connections.write().unwrap();
        write.insert(peer_network_id, peer);
        self.generation.fetch_add(1 , Ordering::SeqCst)
    }

    /// remove a PeerNetworkId entry
    /// return new generation counter
    pub fn remove(&self, peer_network_id: &PeerNetworkId) -> u32 {
        let mut write = self.peer_connections.write().unwrap();
        write.remove(peer_network_id);
        self.generation.fetch_add(1 , Ordering::SeqCst)
    }
}

impl Default for OutboundPeerConnections {
    fn default() -> Self {
        Self::new()
    }
}
