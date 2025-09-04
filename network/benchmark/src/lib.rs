// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_config::{
    config::NodeConfig,
    network_id::{NetworkId, PeerNetworkId},
};
use velor_logger::{
    debug, info,
    prelude::{sample, SampleRate},
    warn,
};
use velor_metrics_core::{register_int_counter_vec, IntCounter, IntCounterVec};
use velor_network::{
    application::interface::{NetworkClient, NetworkClientInterface, NetworkServiceEvents},
    peer_manager::ConnectionNotification,
    protocols::{network::Event, rpc::error::RpcError, wire::handshake::v1::ProtocolId},
};
use velor_time_service::{TimeService, TimeServiceTrait};
use velor_types::{account_address::AccountAddress, PeerId};
use bytes::Bytes;
use futures::{
    channel::oneshot::Sender,
    stream::{FuturesUnordered, StreamExt},
};
use once_cell::sync::Lazy;
use rand::{rngs::OsRng, Rng};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, ops::DerefMut, sync::Arc, time::Duration};
use tokio::{runtime::Handle, select, sync::RwLock};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[allow(clippy::large_enum_variant)]
pub enum NetbenchMessage {
    DataSend(NetbenchDataSend),
    DataReply(NetbenchDataReply),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NetbenchDataSend {
    pub request_counter: u64, // A monotonically increasing counter to verify responses
    pub send_micros: u64, // micro seconds since some epoch at a moment just before this message is sent
    pub data: Vec<u8>,    // A vector of bytes to send in the request; zero length in reply
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NetbenchDataReply {
    pub request_counter: u64, // A monotonically increasing counter to verify responses
    pub send_micros: u64, // micro seconds since some epoch at a moment just before this message is sent
    pub request_send_micros: u64, // the send_micros from the previous message
}

/// Counter for pending network events to the network benchmark service (server-side)
pub static PENDING_NETBENCH_NETWORK_EVENTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_netbench_pending_network_events",
        "Counters for pending network events for benchmarking",
        &["state"]
    )
    .unwrap()
});

// Get messages from the network and quickly shuffle them to N threads of workers.
async fn source_loop(
    network_requests: NetworkServiceEvents<NetbenchMessage>,
    work: async_channel::Sender<(NetworkId, Event<NetbenchMessage>)>,
) {
    let network_events: Vec<_> = network_requests
        .into_network_and_events()
        .into_iter()
        .map(|(network_id, events)| events.map(move |event| (network_id, event)))
        .collect();
    let mut network_events = futures::stream::select_all(network_events).fuse();

    loop {
        match network_events.next().await {
            None => {
                // fused stream will never return more
                work.close();
                return;
            },
            Some(x) => match work.send(x).await {
                Ok(_) => {},
                Err(send_error) => {
                    warn!("netbench source_loop work send: {}", send_error);
                },
            },
        };
    }
}

async fn handle_direct(
    network_client: &NetworkClient<NetbenchMessage>,
    network_id: NetworkId,
    peer_id: AccountAddress,
    msg_wrapper: NetbenchMessage,
    time_service: TimeService,
    shared: Arc<RwLock<NetbenchSharedState>>,
) {
    match msg_wrapper {
        NetbenchMessage::DataSend(send) => {
            let reply = NetbenchDataReply {
                request_counter: send.request_counter,
                send_micros: time_service.now_unix_time().as_micros() as u64,
                request_send_micros: send.send_micros,
            };
            let result = network_client.send_to_peer(
                NetbenchMessage::DataReply(reply),
                PeerNetworkId::new(network_id, peer_id),
            );
            if let Err(err) = result {
                direct_messages("reply_err");
                info!(
                    "netbench ds [{}] could not reply: {}",
                    send.request_counter, err
                );
            }
        },
        NetbenchMessage::DataReply(reply) => {
            let receive_time = time_service.now_unix_time().as_micros() as u64;
            let rec = {
                let reader = shared.read().await;
                reader.find(reply.request_counter)
            };
            if rec.request_counter == reply.request_counter {
                let micros = receive_time - rec.send_micros;
                direct_messages("ok");
                direct_micros("ok", micros);
                direct_bytes("ok", rec.bytes_sent as u64);
            } else {
                direct_messages("late");
                info!(
                    "netbench ds [{}] unknown bytes in > {} micros",
                    reply.request_counter,
                    receive_time - rec.send_micros
                )
            }
        },
    }
}

async fn handle_rpc(
    _peer_id: AccountAddress,
    msg_wrapper: NetbenchMessage,
    protocol_id: ProtocolId,
    time_service: TimeService,
    sender: Sender<Result<Bytes, RpcError>>,
) {
    match msg_wrapper {
        NetbenchMessage::DataSend(send) => {
            let reply = NetbenchDataReply {
                request_counter: send.request_counter,
                send_micros: time_service.now_unix_time().as_micros() as u64,
                request_send_micros: send.send_micros,
            };
            let reply = NetbenchMessage::DataReply(reply);
            let reply_bytes = match protocol_id.to_bytes(&reply) {
                Ok(rb) => rb,
                Err(_) => {
                    rpc_messages("err");
                    return;
                },
            };
            let reply_bytes: Bytes = reply_bytes.into();
            let result = sender.send(Ok(reply_bytes));
            if let Err(err) = result {
                match err {
                    Ok(_) => {}, // what? Ok inside Err?
                    Err(err) => {
                        rpc_messages("err");
                        info!("netbench rpc [{}] reply err: {}", send.request_counter, err);
                    },
                }
            }
        },
        NetbenchMessage::DataReply(_) => {
            rpc_messages("err");
        },
    }
}

/// handle work split out by source_loop()
async fn handler_task(
    network_client: NetworkClient<NetbenchMessage>,
    work_rx: async_channel::Receiver<(NetworkId, Event<NetbenchMessage>)>,
    time_service: TimeService,
    shared: Arc<RwLock<NetbenchSharedState>>,
) {
    loop {
        let (network_id, event) = match work_rx.recv().await {
            Ok(v) => v,
            Err(_) => {
                // RecvError means source was closed, we're done here.
                return;
            },
        };
        match event {
            Event::Message(peer_id, wat) => {
                let msg_wrapper: NetbenchMessage = wat;
                handle_direct(
                    &network_client,
                    network_id,
                    peer_id,
                    msg_wrapper,
                    time_service.clone(),
                    shared.clone(),
                )
                .await;
            },
            Event::RpcRequest(peer_id, msg_wrapper, protocol_id, sender) => {
                handle_rpc(
                    peer_id,
                    msg_wrapper,
                    protocol_id,
                    time_service.clone(),
                    sender,
                )
                .await;
            },
        }
    }
}

/// run_netbench_service() does not return, it should be called by .spawn()
pub async fn run_netbench_service(
    node_config: NodeConfig,
    network_client: NetworkClient<NetbenchMessage>,
    network_requests: NetworkServiceEvents<NetbenchMessage>,
    time_service: TimeService,
) {
    let shared = Arc::new(RwLock::new(NetbenchSharedState::new()));
    let config = node_config.netbench.unwrap();
    let benchmark_service_threads = config.netbench_service_threads;
    let num_threads = match benchmark_service_threads {
        Some(x) => x,
        None => match std::thread::available_parallelism() {
            Ok(val) => {
                let num_threads = val.get();
                debug!(
                    "netbench service running {:?} threads based on available parallelism",
                    num_threads
                );
                num_threads
            },
            Err(_) => {
                debug!("netbench service running 1 thread as fallback");
                1
            },
        },
    };
    let (work_sender, work_receiver) = async_channel::bounded(num_threads * 2);
    let runtime_handle = Handle::current();
    let listener_task = runtime_handle.spawn(connection_listener(
        node_config.clone(),
        network_client.clone(),
        time_service.clone(),
        shared.clone(),
        runtime_handle.clone(),
    ));
    let source_task = runtime_handle.spawn(source_loop(network_requests, work_sender));
    let mut handlers = vec![];
    for _ in 0..num_threads {
        handlers.push(runtime_handle.spawn(handler_task(
            network_client.clone(),
            work_receiver.clone(),
            time_service.clone(),
            shared.clone(),
        )));
    }
    let listener_task_result = listener_task.await;
    info!("netbench listener_task exited {:?}", listener_task_result);
    if let Err(err) = source_task.await {
        warn!("benchmark source_thread join: {}", err);
    }
    for hai in handlers {
        if let Err(err) = hai.await {
            warn!("benchmark handler_thread join: {}", err);
        }
    }
}

async fn connection_listener(
    node_config: NodeConfig,
    network_client: NetworkClient<NetbenchMessage>,
    time_service: TimeService,
    shared: Arc<RwLock<NetbenchSharedState>>,
    handle: Handle,
) {
    let config = node_config.netbench.unwrap();
    let peers_and_metadata = network_client.get_peers_and_metadata();
    let mut connected_peers = HashSet::new();
    let mut connection_notifications = peers_and_metadata.subscribe();
    loop {
        match connection_notifications.recv().await {
            None => {
                info!("netbench connection_listener exit");
                return;
            },
            Some(note) => match note {
                ConnectionNotification::NewPeer(meta, network_id) => {
                    let peer_network_id = PeerNetworkId::new(network_id, meta.remote_peer_id);
                    if connected_peers.contains(&peer_network_id) {
                        continue;
                    }
                    info!(
                        "netbench connection_listener new {:?} {:?}",
                        meta, network_id
                    );
                    if config.enable_direct_send_testing {
                        handle.spawn(direct_sender(
                            node_config.clone(),
                            network_client.clone(),
                            time_service.clone(),
                            network_id,
                            meta.remote_peer_id,
                            shared.clone(),
                        ));
                    }
                    if config.enable_rpc_testing {
                        handle.spawn(rpc_sender(
                            node_config.clone(),
                            network_client.clone(),
                            time_service.clone(),
                            network_id,
                            meta.remote_peer_id,
                            shared.clone(),
                        ));
                    }
                    connected_peers.insert(peer_network_id);
                },
                ConnectionNotification::LostPeer(meta, network_id) => {
                    let peer_network_id = PeerNetworkId::new(network_id, meta.remote_peer_id);
                    connected_peers.remove(&peer_network_id);
                },
            },
        }
    }
}

// Once every X milliseconds log a message
const BLAB_MILLIS: u64 = 1000; // 1 second

pub async fn direct_sender(
    node_config: NodeConfig,
    network_client: NetworkClient<NetbenchMessage>,
    time_service: TimeService,
    network_id: NetworkId,
    peer_id: PeerId,
    shared: Arc<RwLock<NetbenchSharedState>>,
) {
    let config = node_config.netbench.unwrap();
    let interval = Duration::from_nanos(1_000_000_000 / config.direct_send_per_second);
    let ticker = time_service.interval(interval);
    futures::pin_mut!(ticker);
    let data_size = config.direct_send_data_size;
    let mut rng = OsRng;
    let mut blob = Vec::<u8>::with_capacity(data_size);

    // random payload filler
    for _ in 0..data_size {
        blob.push(rng.gen());
    }

    let mut counter: u64 = rng.gen();

    loop {
        ticker.next().await;

        counter += 1;
        {
            // tweak the random payload a little on every send
            let counter_bytes: [u8; 8] = counter.to_le_bytes();
            let (dest, _) = blob.deref_mut().split_at_mut(8);
            dest.copy_from_slice(&counter_bytes);
        }

        let nowu = time_service.now_unix_time().as_micros() as u64;
        let msg = NetbenchDataSend {
            request_counter: counter,
            send_micros: nowu,
            data: blob.clone(),
        };
        {
            shared.write().await.set(SendRecord {
                request_counter: counter,
                send_micros: nowu,
                bytes_sent: blob.len(),
            })
        }
        let wrapper = NetbenchMessage::DataSend(msg);
        let result = network_client.send_to_peer(wrapper, PeerNetworkId::new(network_id, peer_id));
        if let Err(err) = result {
            direct_messages("serr");
            info!(
                "netbench [{},{}] direct send err: {}",
                network_id, peer_id, err
            );
            return;
        } else {
            direct_messages("sent");
        }

        sample!(
            SampleRate::Duration(Duration::from_millis(BLAB_MILLIS)),
            info!("netbench ds counter={}", counter)
        );
    }
}

pub async fn rpc_sender(
    node_config: NodeConfig,
    network_client: NetworkClient<NetbenchMessage>,
    time_service: TimeService,
    network_id: NetworkId,
    peer_id: PeerId,
    shared: Arc<RwLock<NetbenchSharedState>>,
) {
    let config = node_config.netbench.unwrap();
    let interval = Duration::from_nanos(1_000_000_000 / config.rpc_per_second);
    let ticker = time_service.interval(interval);
    futures::pin_mut!(ticker);
    // random payload filler
    let data_size = config.rpc_data_size;
    let mut blob = Vec::<u8>::with_capacity(data_size);
    let mut rng = OsRng;
    for _ in 0..data_size {
        blob.push(rng.gen());
    }

    let mut counter: u64 = rng.gen();

    let mut open_rpcs = FuturesUnordered::new();

    loop {
        select! {
            _ = ticker.next() => {
                if open_rpcs.len() >= config.rpc_in_flight {
                    continue;
                }
                // do rpc send
                counter += 1;
                {
                    // tweak the random payload a little on every send
                    let counter_bytes: [u8; 8] = counter.to_le_bytes();
                    let (dest, _) = blob.deref_mut().split_at_mut(8);
                    dest.copy_from_slice(&counter_bytes);
                }

                let nowu = time_service.now_unix_time().as_micros() as u64;
                let msg = NetbenchDataSend {
                    request_counter: counter,
                    send_micros: nowu,
                    data: blob.clone(),
                };
                {
                    shared.write().await.set(SendRecord{
                        request_counter: counter,
                        send_micros: nowu,
                        bytes_sent: blob.len(),
                    })
                }
                let wrapper = NetbenchMessage::DataSend(msg);
                let result = network_client.send_to_peer_rpc(wrapper, Duration::from_secs(10), PeerNetworkId::new(network_id, peer_id));
                rpc_messages("sent");
                open_rpcs.push(result);

                sample!(SampleRate::Duration(Duration::from_millis(BLAB_MILLIS)), info!("netbench rpc counter={}", counter));
            }
            result = open_rpcs.next() => {
                let result = match result {
                    Some(subr) => {subr}
                    None => {
                        continue
                    }
                };
                // handle rpc result
                match result {
                    Err(err) => {
                        info!("netbench [{},{}] rpc send err: {}", network_id, peer_id, err);
                        rpc_messages("err");
                        return;
                    }
                    Ok(msg_wrapper) => {
                        let nowu = time_service.now_unix_time().as_micros() as u64;
                        if let NetbenchMessage::DataReply(msg) = msg_wrapper {
                            let send_dt = nowu - msg.request_send_micros;
                            info!("netbench [{}] rpc at {} µs, took {} µs", msg.request_counter, nowu, send_dt);
                            rpc_messages("ok");
                            rpc_bytes("ok").inc_by(data_size as u64);
                            rpc_micros("ok").inc_by(send_dt);
                        } else {
                            rpc_messages("bad");
                            info!("netbench [{}] rpc garbage reply", counter);
                        }
                    }
                }
            }
        }
    }
}

pub struct NetbenchSharedState {
    // Circular buffer of sent records
    sent: Vec<SendRecord>,
    // sent[sent_pos] is the next index to write
    sent_pos: usize,
}

impl Default for NetbenchSharedState {
    fn default() -> Self {
        Self::new()
    }
}

impl NetbenchSharedState {
    pub fn new() -> Self {
        NetbenchSharedState {
            sent: Vec::with_capacity(10000), // TODO: constant or config?
            sent_pos: 0,
        }
    }

    pub fn set(&mut self, sent: SendRecord) {
        if self.sent.len() < self.sent.capacity() {
            self.sent.push(sent);
        } else {
            self.sent[self.sent_pos] = sent;
        }
        self.sent_pos = (self.sent_pos + 1) % self.sent.capacity();
    }

    /// return the record for the request_counter, or `{0, oldest send_micros}`
    /// `Option<SendRecord>` might seem like it would make sense, but we use the send_micros field to return the oldest known message time when we don't find a request_counter match.
    pub fn find(&self, request_counter: u64) -> SendRecord {
        if self.sent.is_empty() {
            return SendRecord {
                request_counter: 0,
                send_micros: 0,
                bytes_sent: 0,
            };
        }
        let mut oldest = self.sent[0].send_micros;
        let capacity = self.sent.len();
        for i in 0..capacity {
            let pos = (self.sent_pos + capacity - (1 + i)) % capacity;
            let rec = self.sent[pos].clone();
            if rec.request_counter == request_counter {
                return rec;
            }
            if rec.send_micros < oldest {
                oldest = rec.send_micros;
            }
        }
        SendRecord {
            request_counter: 0,
            send_micros: oldest,
            bytes_sent: 0,
        }
    }
}

#[derive(Clone)]
pub struct SendRecord {
    pub request_counter: u64,
    pub send_micros: u64,
    pub bytes_sent: usize,
}

pub static VELOR_NETWORK_BENCHMARK_DIRECT_MESSAGES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_network_benchmark_direct_messages",
        "Number of net benchmark direct messages",
        &["state"]
    )
    .unwrap()
});

fn direct_messages(state_label: &'static str) {
    VELOR_NETWORK_BENCHMARK_DIRECT_MESSAGES
        .with_label_values(&[state_label])
        .inc();
}

pub static VELOR_NETWORK_BENCHMARK_DIRECT_BYTES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_network_benchmark_direct_bytes",
        "Number of net benchmark direct bytes",
        &["state"]
    )
    .unwrap()
});

fn direct_bytes(state_label: &'static str, byte_count: u64) {
    VELOR_NETWORK_BENCHMARK_DIRECT_BYTES
        .with_label_values(&[state_label])
        .inc_by(byte_count);
}

pub static VELOR_NETWORK_BENCHMARK_DIRECT_MICROS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_network_benchmark_direct_micros",
        "Number of net benchmark direct micros",
        &["state"]
    )
    .unwrap()
});

fn direct_micros(state_label: &'static str, micros: u64) {
    VELOR_NETWORK_BENCHMARK_DIRECT_MICROS
        .with_label_values(&[state_label])
        .inc_by(micros);
}

pub static VELOR_NETWORK_BENCHMARK_RPC_MESSAGES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_network_benchmark_rpc_messages",
        "Number of net benchmark RPC messages",
        &["state"]
    )
    .unwrap()
});

fn rpc_messages(state_label: &'static str) {
    VELOR_NETWORK_BENCHMARK_RPC_MESSAGES
        .with_label_values(&[state_label])
        .inc();
}

pub static VELOR_NETWORK_BENCHMARK_RPC_BYTES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_network_benchmark_rpc_bytes",
        "Number of net benchmark RPC bytes transferred",
        &["state"]
    )
    .unwrap()
});

pub fn rpc_bytes(state_label: &'static str) -> IntCounter {
    VELOR_NETWORK_BENCHMARK_RPC_BYTES.with_label_values(&[state_label])
}

pub static VELOR_NETWORK_BENCHMARK_RPC_MICROS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_network_benchmark_rpc_micros",
        "Number of net benchmark RPC microseconds used (hint: divide by _messages)",
        &["state"]
    )
    .unwrap()
});

pub fn rpc_micros(state_label: &'static str) -> IntCounter {
    VELOR_NETWORK_BENCHMARK_RPC_MICROS.with_label_values(&[state_label])
}
