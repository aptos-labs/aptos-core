// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use bytes::Bytes;
use channel::{diem_channel, message_queues::QueueStyle};
use diem_types::PeerId;
use futures::{
    channel::oneshot,
    future,
    stream::{BoxStream, Stream, StreamExt},
};
use network::{
    peer_manager::{ConnectionNotification, PeerManagerNotification},
    protocols::network::{AppConfig, Event, NetworkEvents, NewNetworkEvents, RpcError},
    ProtocolId,
};
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use storage_service_types::{
    Result, StorageServiceMessage, StorageServiceRequest, StorageServiceResponse,
};

const INBOUND_CHANNEL_SIZE: usize = 100;

pub fn network_endpoint_config() -> AppConfig {
    AppConfig::service(
        [ProtocolId::StorageServiceRpc],
        diem_channel::Config::new(INBOUND_CHANNEL_SIZE).queue_style(QueueStyle::FIFO),
    )
}

pub type NetworkRequest = (PeerId, ProtocolId, StorageServiceRequest, ResponseSender);

/// A stream of requests from network. Each request also comes with a callback to
/// send the response.
pub struct StorageServiceNetworkEvents(BoxStream<'static, NetworkRequest>);

impl NewNetworkEvents for StorageServiceNetworkEvents {
    fn new(
        peer_mgr_notifs_rx: diem_channel::Receiver<(PeerId, ProtocolId), PeerManagerNotification>,
        connection_notifs_rx: diem_channel::Receiver<PeerId, ConnectionNotification>,
    ) -> Self {
        let events = NetworkEvents::new(peer_mgr_notifs_rx, connection_notifs_rx)
            .filter_map(|event| future::ready(Self::event_to_request(event)))
            .boxed();

        Self(events)
    }
}

impl StorageServiceNetworkEvents {
    /// Filters out everything except Rpc requests
    fn event_to_request(event: Event<StorageServiceMessage>) -> Option<NetworkRequest> {
        // TODO(philiphayes): logging
        match event {
            Event::RpcRequest(
                peer_id,
                StorageServiceMessage::Request(request),
                protocol_id,
                response_tx,
            ) => {
                let response_tx = ResponseSender::new(response_tx);
                Some((peer_id, protocol_id, request, response_tx))
            }
            // We don't use DirectSend and don't care about connection events.
            _ => None,
        }
    }
}

impl Stream for StorageServiceNetworkEvents {
    type Item = NetworkRequest;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.0).poll_next(cx)
    }
}

/// A channel for fulfilling a pending StorageService RPC request.
/// Provides a more strongly typed interface around the raw RPC response channel.
pub struct ResponseSender {
    response_tx: oneshot::Sender<Result<Bytes, RpcError>>,
}

impl ResponseSender {
    pub fn new(response_tx: oneshot::Sender<Result<Bytes, RpcError>>) -> Self {
        Self { response_tx }
    }

    pub fn send(self, response: Result<StorageServiceResponse>) {
        let msg = StorageServiceMessage::Response(response);
        let result = bcs::to_bytes(&msg)
            .map(Bytes::from)
            .map_err(RpcError::BcsError);
        let _ = self.response_tx.send(result);
    }
}
