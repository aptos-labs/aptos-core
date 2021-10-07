// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

//! Interface between State Sync and Network layers.

use crate::{chunk_request::GetChunkRequest, chunk_response::GetChunkResponse, counters};
use async_trait::async_trait;
use channel::{diem_channel, message_queues::QueueStyle};
use diem_types::PeerId;
use network::{
    error::NetworkError,
    peer_manager::{ConnectionRequestSender, PeerManagerRequestSender},
    protocols::network::{
        AppConfig, ApplicationNetworkSender, NetworkEvents, NetworkSender, NewNetworkSender,
        RpcError,
    },
    ProtocolId,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const STATE_SYNC_MAX_BUFFER_SIZE: usize = 1;

/// State sync network messages
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum StateSyncMessage {
    GetChunkRequest(Box<GetChunkRequest>),
    GetChunkResponse(Box<GetChunkResponse>),
}

/// The interface from Network to StateSync layer.
///
/// `StateSyncEvents` is a `Stream` of `PeerManagerNotification` where the
/// raw `Bytes` direct-send messages are deserialized into `StateSyncMessage`
/// types. `StateSyncEvents` is a thin wrapper around a
/// `channel::Receiver<PeerManagerNotification>`.
pub type StateSyncEvents = NetworkEvents<StateSyncMessage>;

/// The interface from StateSync to Networking layer.
///
/// This is a thin wrapper around a `NetworkSender<StateSyncMessage>`, so it
/// is easy to clone and send off to a separate task. For example, the rpc
/// requests return Futures that encapsulate the whole flow, from sending the
/// request to remote, to finally receiving the response and deserializing. It
/// therefore makes the most sense to make the rpc call on a separate async task,
/// which requires the `StateSyncSender` to be `Clone` and `Send`.
#[derive(Clone)]
pub struct StateSyncSender {
    inner: NetworkSender<StateSyncMessage>,
}

impl NewNetworkSender for StateSyncSender {
    fn new(
        peer_mgr_reqs_tx: PeerManagerRequestSender,
        connection_reqs_tx: ConnectionRequestSender,
    ) -> Self {
        Self {
            inner: NetworkSender::new(peer_mgr_reqs_tx, connection_reqs_tx),
        }
    }
}

#[async_trait]
impl ApplicationNetworkSender<StateSyncMessage> for StateSyncSender {
    fn send_to(&self, recipient: PeerId, message: StateSyncMessage) -> Result<(), NetworkError> {
        let protocol = ProtocolId::StateSyncDirectSend;
        self.inner.send_to(recipient, protocol, message)
    }

    async fn send_rpc(
        &self,
        _recipient: PeerId,
        _req_msg: StateSyncMessage,
        _timeout: Duration,
    ) -> Result<StateSyncMessage, RpcError> {
        unimplemented!()
    }
}

/// Configuration for the network endpoints to support state sync.
pub fn network_endpoint_config() -> AppConfig {
    AppConfig::p2p(
        [ProtocolId::StateSyncDirectSend],
        diem_channel::Config::new(STATE_SYNC_MAX_BUFFER_SIZE)
            .queue_style(QueueStyle::LIFO)
            .counters(&counters::PENDING_STATE_SYNC_NETWORK_EVENTS),
    )
}
