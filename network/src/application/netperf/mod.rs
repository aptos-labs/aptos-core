// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Network Load Generator
//!
//! NetPerf is used to stress the network layer to gouge potential performance capabilities
//! and simplify network-related performance profiling and debugging
//!

use crate::application::storage::PeerMetadataStorage;
use crate::transport::ConnectionMetadata;
use crate::{
    application::netperf::interface::{NetPerfNetworkEvents, NetPerfNetworkSender},
    constants::NETWORK_CHANNEL_SIZE,
    counters,
    error::NetworkError,
    logging::NetworkSchema,
    peer_manager::{ConnectionRequestSender, PeerManagerRequestSender},
    protocols::{
        network::{
            AppConfig, ApplicationNetworkSender, Event, NetworkEvents, NetworkSender,
            NewNetworkSender,
        },
        rpc::error::RpcError,
    },
    ProtocolId,
};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::network_id::{NetworkContext, PeerNetworkId};
use aptos_logger::prelude::*;
use aptos_types::account_address::AccountAddress;
use aptos_types::PeerId;
use axum::{routing::get, Extension, Json, Router};
use dashmap::DashMap;
use futures::StreamExt;
use serde::Serialize;
use std::sync::Arc;

pub mod builder;
mod interface;

pub struct NetPerf {
    network_context: NetworkContext,
    peers: Arc<PeerMetadataStorage>,
    peer_list: Arc<DashMap<PeerId, PeerNetPerfStat>>, //with capacity and hasher
    sender: Arc<NetPerfNetworkSender>,
    events: NetPerfNetworkEvents,
    netperf_port: u16,
}

struct PeerNetPerfStat {}

impl PeerNetPerfStat {
    pub fn new(_md: ConnectionMetadata) -> Self {
        PeerNetPerfStat {}
    }
}

#[derive(Clone)]
struct NetPerfState {
    peers: Arc<PeerMetadataStorage>, //TODO: DO I need this?
    peer_list: Arc<DashMap<PeerId, PeerNetPerfStat>>, //with capacity and hasher
    sender: Arc<NetPerfNetworkSender>,
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
            network_context,
            peers,
            peer_list: Arc::new(DashMap::with_capacity(128)),
            sender,
            events,
            netperf_port,
        }
    }

    /// Configuration for the network endpoints to support NetPerf.
    pub fn network_endpoint_config() -> AppConfig {
        AppConfig::p2p(
            [ProtocolId::NetPerfRpcCompressed],
            aptos_channel::Config::new(NETWORK_CHANNEL_SIZE).queue_style(QueueStyle::LIFO),
        )
    }

    fn net_perf_state(&self) -> NetPerfState {
        NetPerfState {
            peers: self.peers.clone(),
            sender: self.sender.clone(),
            peer_list: self.peer_list.clone(),
        }
    }

    async fn start(mut self) {
        info!(
            NetworkSchema::new(&self.network_context),
            "{} NetPerf Event Listener started", self.network_context
        );

        spawn_named!(
            "NetPerf Axum",
            start_axum(self.net_perf_state(), self.netperf_port)
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

async fn start_axum(state: NetPerfState, netperf_port: u16) {
    let app = Router::new()
        .route("/", get(usage_handler))
        .route("/peers", get(get_peers).layer(Extension(state)));

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], netperf_port));

    // run it with hyper on netperf_port
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn usage_handler() -> &'static str {
    "Usage: curl 127.0.0.01:9107/peers"
}

#[derive(Serialize)]
struct PeerList {
    len: usize,
    peers: Vec<PeerId>,
}

impl PeerList {
    pub fn new(len: usize) -> Self {
        PeerList {
            len,
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
