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
use aptos_logger::prelude::*;
use axum::{routing::get, Extension, Router};
use std::sync::Arc;

pub mod builder;
mod interface;

pub struct NetPerf {
    network_context: NetworkContext,
    peers: Arc<PeerMetadataStorage>,
    sender: Arc<NetPerfNetworkSender>,
    events: NetPerfNetworkEvents,
}

#[derive(Clone)]
struct NetPerfState {
    peers: Arc<PeerMetadataStorage>,
    peer_list: DashMap<PeerId, ()>, //with capacity and hasher
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

    async fn event_handler(self) {
        info!(
            NetworkSchema::new(&self.network_context),
            "{} NetPerf Event Listener started", self.network_context
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
                            self.network_interface.app_data().insert(
                                metadata.remote_peer_id,
                                HealthCheckData::new(self.round)
                            );
                        }
                        Event::LostPeer(metadata) => {
                            self.network_interface.app_data().remove(
                                &metadata.remote_peer_id
                            );
                        }
                            /*
                        Event::RpcRequest(peer_id, msg, protocol, res_tx) => {
                            match msg {
                                NetPerfMsg::Ping(ping) => self.handle_ping_request(peer_id, ping, protocol, res_tx),
                                _ => {
                                    warn!(
                                        SecurityEvent::InvalidNetPerfMsg,
                                        NetworkSchema::new(&self.network_context).remote_peer(&peer_id),
                                        rpc_message = msg,
                                        "{} Unexpected RPC message from {}",
                                        self.network_context,
                                        peer_id
                                    );
                                    debug_assert!(false, "Unexpected rpc request");
                                }
                            };
                        }
                        Event::Message(peer_id, msg) => {
                            error!(
                                SecurityEvent::InvalidNetworkEventHC,
                                NetworkSchema::new(&self.network_context).remote_peer(&peer_id),
                                "{} Unexpected direct send from {} msg {:?}",
                                self.network_context,
                                peer_id,
                                msg,
                            );
                            debug_assert!(false, "Unexpected network event");
                        }
                             */
                    }
                }
                _ = ticker.select_next_some() => {
                    self.round += 1;
                    let connected = self.network_interface.connected_peers();
                    if connected.is_empty() {
                        trace!(
                            NetworkSchema::new(&self.network_context),
                            round = self.round,
                            "{} No connected peer to ping round: {}",
                            self.network_context,
                            self.round
                        );
                        continue
                    }

                    for peer_id in connected {
                        let nonce = self.rng.gen::<u32>();
                        trace!(
                            NetworkSchema::new(&self.network_context),
                            round = self.round,
                            "{} Will ping: {} for round: {} nonce: {}",
                            self.network_context,
                            peer_id.short_str(),
                            self.round,
                            nonce
                        );

                        tick_handlers.push(Self::ping_peer(
                            self.network_context,
                            self.network_interface.sender(),
                            peer_id,
                            self.round,
                            nonce,
                            self.ping_timeout,
                        ));
                    }
                }
                res = tick_handlers.select_next_some() => {
                    let (peer_id, round, nonce, ping_result) = res;
                    self.handle_ping_response(peer_id, round, nonce, ping_result).await;
                }
            }
        }
        warn!(
            NetworkSchema::new(&self.network_context),
            "{} NetPerf event listener terminated", self.network_context
        );
    }

    pub async fn start(self) {
        let state = NetPerfState {
            peers: self.peers.clone(),
            sender: self.sender.clone(),
        };

        let app = Router::new()
            .route("/", get(usage_handler))
            .route("/peers", get(get_peers).layer(Extension(state)));

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

//#TODO: Json output
async fn get_peers(Extension(state): Extension<NetPerfState>) -> &'static str {
    let connected = self.network_interface.connected_peers();
    if connected.is_empty() {
        trace!(
            NetworkSchema::new(&self.network_context),
            round = self.round,
            "{} No connected peer to ping round: {}",
            self.network_context,
            self.round
        );
    }

    for peer_id in connected {
        let nonce = self.rng.gen::<u32>();
        trace!(
            NetworkSchema::new(&self.network_context),
            round = self.round,
            "{} Will ping: {} for round: {} nonce: {}",
            self.network_context,
            peer_id.short_str(),
            self.round,
            nonce
        );
    }
    "Usage: curl 127.0.0.01:9107/peers"
}
