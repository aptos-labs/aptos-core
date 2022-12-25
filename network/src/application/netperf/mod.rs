// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Network stresser
//!
//! NetPerf is used to stress the network laayer to gouge potential performance capabilities and ease
//! network realted performance profiling and debugging
//!

use crate::application::storage::PeerMetadataStorage;
use crate::{
    application::interface::NetworkInterface,
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
use std::sync::Arc;

//Interface

pub enum NetPerfMsg {
    BlockOfBytes64K,
}
/// The interface from Network to NetPerf layer.
///
/// `NetPerfNetworkEvents` is a `Stream` of `PeerManagerNotification` where the
/// raw `Bytes` rpc messages are deserialized into
/// `NetPerfMsg` types. `NetPerfNetworkEvents` is a thin wrapper
/// around an `channel::Receiver<PeerManagerNotification>`.
pub type NetPerfNetworkEvents = NetworkEvents<NetPerfMsg>;

/// The interface from NetPerf to Networking layer.
///
/// This is a thin wrapper around a `NetworkSender<NetPerfMsg>`, so it is
/// easy to clone and send off to a separate task. For example, the rpc requests
/// return Futures that encapsulate the whole flow, from sending the request to
/// remote, to finally receiving the response and deserializing. It therefore
/// makes the most sense to make the rpc call on a separate async task, which
/// requires the `NetPerfNetworkSender` to be `Clone` and `Send`.
pub type NetPerfNetworkSender = NetworkSender<NetPerfMsg>;

//Interface End
pub struct NetPerf {
    network_context: NetworkContext,
    peers: Arc<PeerMetadataStorage>,
    sender: Arc<NetPerfNetworkSender>,
    events: NetPerfNetworkEvents,
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

    pub async fn start(mut self) {}
}
