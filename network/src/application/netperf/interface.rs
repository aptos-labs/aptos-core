// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
//
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
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NetPerfPayload {
    byte: Vec<u8>,
}

impl NetPerfPayload {
    pub fn new(len :usize) -> Self {
        let v = Vec::with_capacity(len);
        NetPerfPayload { byte: v }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum NetPerfMsg {
    BlockOfBytes(NetPerfPayload),
}
/// The interface from Network to NetPerf layer.
///
/// `NetPerfNetworkEvents` is a `Stream` of `PeerManagerNotification` where the
/// raw `Bytes` rpc messages are deserialized into
/// `NetPerfMsg` types. `NetPerfNetworkEvents` is a thin wrapper
/// around an `channel::Receiver<PeerManagerNotification>`.
pub type NetPerfNetworkEvents = NetworkEvents<NetPerfMsg>;
/*
impl Stream for HealthCheckNetworkInterface {
    type Item = Event<HealthCheckerMsg>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().receiver).poll_next(cx)
    }
}

 */
/// The interface from NetPerf to Networking layer.
///
/// This is a thin wrapper around a `NetworkSender<NetPerfMsg>`, so it is
/// easy to clone and send off to a separate task. For example, the rpc requests
/// return Futures that encapsulate the whole flow, from sending the request to
/// remote, to finally receiving the response and deserializing. It therefore
/// makes the most sense to make the rpc call on a separate async task, which
/// requires the `NetPerfNetworkSender` to be `Clone` and `Send`.
pub type NetPerfNetworkSender = NetworkSender<NetPerfMsg>;
