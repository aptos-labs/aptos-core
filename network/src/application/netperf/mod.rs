// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Network Load Generator
//!
//! NetPerf is used to stress the network layer to gouge potential performance capabilities
//! and simplify network-related performance profiling and debugging
//!

use crate::error::NetworkError;
use crate::{
    application::{
        netperf::interface::{
            NetPerfMsg, NetPerfNetworkEvents, NetPerfNetworkSender, NetPerfPayload,
        },
        storage::PeerMetadataStorage,
    },
    constants::NETWORK_CHANNEL_SIZE,
    counters,
    logging::NetworkSchema,
    protocols::network::{Event, NetworkApplicationConfig},
    transport::ConnectionMetadata,
    ProtocolId,
};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::network_id::NetworkContext;
use aptos_logger::prelude::*;
use aptos_types::PeerId;
use axum::{
    extract::Query,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use dashmap::DashMap;
use futures::StreamExt;
use futures_util::stream::FuturesUnordered;
use serde::Serialize;
use std::{
    fs::OpenOptions,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
use tokio::sync::mpsc::{Receiver, Sender};

pub mod builder;
mod interface;

const NETPERF_COMMAND_CHANNEL_SIZE: usize = 1024;
const NETPERF_DEFAULT_MSG_SIZE: usize = 64 * 1024;
const NETPERF_DEFAULT_DURTAION_SEC: u64 = 10;

struct PeerNetPerfStat {}

impl PeerNetPerfStat {
    pub fn new(_md: ConnectionMetadata) -> Self {
        PeerNetPerfStat {}
    }
}

#[derive(Clone)]
struct NetPerfState {
    network_context: Arc<NetworkContext>,
    peers: Arc<PeerMetadataStorage>,
    peer_list: Arc<DashMap<PeerId, PeerNetPerfStat>>, //with capacity and hasher
    sender: Arc<NetPerfNetworkSender>,
    tx: Sender<NetPerfCommands>,
}

impl NetPerfState {
    fn send_to_many(
        &self,
        recipients: impl Iterator<Item = PeerId>,
        protocol: ProtocolId,
        msg: &NetPerfMsg,
    ) -> Result<(), NetworkError> {
        self.sender.send_to_many(recipients, protocol, msg)
    }
}

#[allow(dead_code)]
pub struct NetPerf {
    network_context: Arc<NetworkContext>,
    peers: Arc<PeerMetadataStorage>,
    peer_list: Arc<DashMap<PeerId, PeerNetPerfStat>>, //with capacity and hasher
    sender: Arc<NetPerfNetworkSender>,
    events: NetPerfNetworkEvents,
    netperf_port: u16,
}

impl NetPerf {
    pub fn new(
        network_context: NetworkContext,
        peers: Arc<PeerMetadataStorage>,
        sender: Arc<NetPerfNetworkSender>,
        events: NetPerfNetworkEvents,
        netperf_port: u16,
    ) -> Self {
        NetPerf {
            network_context: Arc::new(NetworkContext::from(network_context)),
            peers,
            peer_list: Arc::new(DashMap::with_capacity(128)),
            sender,
            events,
            netperf_port,
        }
    }

    /// Configuration for the network endpoints to support NetPerf.
    pub fn network_endpoint_config() -> NetworkApplicationConfig {
        NetworkApplicationConfig::client_and_service(
            [
                ProtocolId::NetPerfDirectSendCompressed,
                ProtocolId::NetPerfRpcCompressed,
            ],
            aptos_channel::Config::new(NETWORK_CHANNEL_SIZE)
                .queue_style(QueueStyle::FIFO)
                .counters(&counters::PENDING_NET_PERF_NETWORK_EVENTS),
        )
    }

    fn net_perf_state(&self, sender: Sender<NetPerfCommands>) -> NetPerfState {
        NetPerfState {
            network_context: self.network_context.clone(),
            peers: self.peers.clone(),
            sender: self.sender.clone(),
            peer_list: self.peer_list.clone(),
            tx: sender,
        }
    }

    async fn start(mut self) {
        let port = preferred_axum_port(self.netperf_port);
        let (tx, _rx) = tokio::sync::mpsc::channel::<NetPerfCommands>(NETPERF_COMMAND_CHANNEL_SIZE);

        info!(
            NetworkSchema::new(&self.network_context),
            "{} NetPerf Event Listener started", self.network_context,
        );

        spawn_named!(
            "NetPerf Axum",
            start_axum(self.net_perf_state(tx.clone()), port)
        );

        loop {
            futures::select! {
                maybe_event = self.events.next() => {
                    // Shutdown the NetPerf when this network instance shuts
                    // down. This happens when the `PeerManager` drops.
                    let event = match maybe_event {
                        Some(event) => event,
                        None => break,
                    };

                    match event {
                        Event::NewPeer(metadata) => {
                            self.peer_list.insert(
                                metadata.remote_peer_id,
                                PeerNetPerfStat::new(metadata)
                            );
                        }
                        Event::LostPeer(metadata) => {
                            self.peer_list.remove(
                                &metadata.remote_peer_id
                            );
                        }
                        Event::Message(_peer_id, msg) =>  {
                            match msg {
                                NetPerfMsg::BlockOfBytes(_bytes) => {
                                    /* maybe add dedicated counters? but network_application_{out/in}bound_traffic
                                     * seems to have us covered
                                     * */
                                }

                            }

                        }
                        _ => {/* Currently ignore all*/}
                    }
                }
            }
        }
        warn!(
            NetworkSchema::new(&self.network_context),
            "{} NetPerf event listener terminated", self.network_context
        );
    }
}
#[allow(dead_code)]
#[derive(Clone)]
enum NetPerfCommands {
    Broadcast,
}

fn preferred_axum_port(netperf_port: u16) -> u16 {
    if netperf_port != 9107 {
        let _ = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open("/tmp/9107.tmp");

        let _ = OpenOptions::new()
            .write(true)
            .create(true)
            .open(format!("/tmp/{}.tmp", netperf_port));
    }
    netperf_port
}

async fn start_axum(state: NetPerfState, mut netperf_port: u16) {
    loop {
        let app = Router::new()
            .route("/", get(usage_handler))
            .route("/peers", get(get_peers).layer(Extension(state.clone())))
            .route(
                "/command",
                post(parse_query).layer(Extension(state.clone())),
            );

        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], netperf_port));

        // run it with hyper on netperf_port
        let res = axum::Server::try_bind(&addr);
        if let Ok(handler) = res {
            let _ = handler.serve(app.into_make_service()).await;
            break;
        } else {
            netperf_port += 1;
        }
    }
    preferred_axum_port(netperf_port);
}

async fn usage_handler() -> &'static str {
    "Usage: curl 127.0.0.01:9107/peers"
}

#[derive(Serialize)]
struct PeerList {
    peers: Vec<PeerId>,
}

impl PeerList {
    pub fn new(len: usize) -> Self {
        PeerList {
            peers: Vec::with_capacity(len),
        }
    }
}

async fn get_peers(Extension(state): Extension<NetPerfState>) -> Json<PeerList> {
    let mut out = PeerList::new(state.peer_list.len());

    let connected = state.peer_list.iter();

    for peer in connected {
        out.peers.push(peer.key().to_owned());
    }

    Json(out)
}

async fn parse_query(
    Extension(state): Extension<NetPerfState>,
    Query(_params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    //TODO(AlexM): Extract size and duration from _params
    let size = NETPERF_DEFAULT_MSG_SIZE;
    let duration = Duration::from_secs(NETPERF_DEFAULT_DURTAION_SEC);

    spawn_named!(
        "[NetPerf] Brodcast Task",
        netperf_broadcast(state, size, duration)
    );

    StatusCode::OK
}

#[allow(dead_code)]
async fn netperf_comp_handler(state: NetPerfState, mut rx: Receiver<NetPerfCommands>) {
    let mut rpc_handlers = FuturesUnordered::new();

    loop {
        tokio::select! {
            opt_cmd = rx.recv() => {
                match opt_cmd {
                    Some(_cmd) => {
                        let msg = NetPerfMsg::BlockOfBytes(NetPerfPayload::new(64 * 1024));

                        for peer in state.peer_list.iter() {
                            //TODO(AlexM): Yet another Alloc + Copy OPs.
                            // Best use Refs - Just ARC
                            rpc_handlers.push(state.sender.send_rpc(
                                peer.key().to_owned(),
                                ProtocolId::NetPerfRpcCompressed,
                                msg.clone(),
                                Duration::from_secs(5),
                            ));
                        }
                    }
                    None => break,
                }
            }
            _res = rpc_handlers.select_next_some() => {break}
        }
    }
}
#[allow(dead_code)]
fn loop_body(state: &NetPerfState, msg: &NetPerfMsg, iter: impl Iterator<Item = PeerId>) -> bool {
    /* TODO(AlexM):
     * 1. Fine grained control with send_to.
     *    Its interesting to see which of the validator queus gets filled.
     * */
    let rc = state
        .send_to_many(iter, ProtocolId::NetPerfDirectSendCompressed, &msg);
    if rc.is_err() {
        true
    } else {
        /* maybe add dedicated pep-peer counters? but network_application_{out/in}bound_traffic
         * seems to have us covered
         * */
        false
    }
}

async fn netperf_broadcast(state: NetPerfState, size: usize, duration: Duration) {
    let mut should_yield;
    let msg = NetPerfMsg::BlockOfBytes(NetPerfPayload::new(size));
    let done = Arc::new(AtomicBool::new(false));
    let stop = done.clone();

    let mut sent = 0;
    let mut yield_count = 0;

    tokio::spawn(async move {
        tokio::time::sleep(duration).await;
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
    });
    info!("Broadcast loop starting");
    loop {
        let iter = state.peer_list.iter().map(|entry| entry.key().to_owned());
        let rc = state
            .send_to_many(iter, ProtocolId::NetPerfDirectSendCompressed, &msg);
        should_yield = if rc.is_err() {
            true
        } else {
            /* maybe add dedicated pep-peer counters? but network_application_{out/in}bound_traffic
             * seems to have us covered
             * */
            false
        };

        if done.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
        if should_yield {
            yield_count += 1;
            tokio::task::yield_now().await;
        } else {
            sent += 1;
        }
    }
    info!(
        NetworkSchema::new(&state.network_context),
        "{} NetPerf Broadcast finished: sent: {} yield {}",
        state.network_context,
        sent,
        yield_count,
    );
}
