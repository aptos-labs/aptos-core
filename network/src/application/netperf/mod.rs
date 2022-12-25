// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Network Load Generator
//!
//! NetPerf is used to stress the network layer to gouge potential performance capabilities
//! and simplify network-related performance profiling and debugging
//!

use crate::application::storage::PeerMetadataStorage;
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
use axum::{routing::get, Router};
use std::sync::Arc;

pub mod builder;
mod interface;

pub struct NetPerf {
    network_context: NetworkContext,
    peers: Arc<PeerMetadataStorage>,
    sender: Arc<NetPerfNetworkSender>,
    events: NetPerfNetworkEvents,
}

struct NetPerfState {
    peers: Arc<PeerMetadataStorage>,
    sender: Arc<NetPerfNetworkSender>,
}

impl NetPerf {
    pub fn new(
        network_context: NetworkContext,
        peers: Arc<PeerMetadataStorage>,
        sender: Arc<NetPerfNetworkSender>,
        events: NetPerfNetworkEvents,
    ) -> Self {
        NetPerf {
            network_context,
            peers,
            sender,
            events,
        }
    }

    /// Configuration for the network endpoints to support NetPerf.
    pub fn network_endpoint_config() -> AppConfig {
        AppConfig::p2p(
            [ProtocolId::NetPerfRpcCompressed],
            aptos_channel::Config::new(NETWORK_CHANNEL_SIZE).queue_style(QueueStyle::LIFO),
        )
    }

    pub async fn start(mut self) {
        let state = NetPerfState {
            peers: self.peers.clone(),
            sender: self.sender.clone(),
        };

        let app = Router::new()
            .route("/", get(usage_handler))
            .route("/peers", get(get_peers));

        // run it with hyper on localhost:9107
        axum::Server::bind(&"0.0.0.0:9107".parse().unwrap())
            .serve(app.into_make_service())
            .await
            .unwrap();
    }
}

async fn usage_handler() -> &'static str {
    "Usage: curl 127.0.0.01:9107/peers"
}

async fn get_peers() -> &'static str {
    "Usage: curl 127.0.0.01:9107/peers"
}
