// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::metrics;
use aptos_config::config::PeerMonitoringServiceConfig;
use aptos_types::PeerId;
use bytes::Bytes;
use channel::{aptos_channel, message_queues::QueueStyle};
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
use peer_monitoring_service_types::{
    PeerMonitoringServiceMessage, PeerMonitoringServiceRequest, PeerMonitoringServiceResponse,
    Result,
};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

// TODO(joshlind): remove the code duplication and boilerplate between
// the different AptosNet services.

pub fn network_endpoint_config(peer_monitoring_config: PeerMonitoringServiceConfig) -> AppConfig {
    let max_network_channel_size = peer_monitoring_config.max_network_channel_size as usize;
    AppConfig::service(
        [ProtocolId::PeerMonitoringServiceRpc],
        aptos_channel::Config::new(max_network_channel_size)
            .queue_style(QueueStyle::FIFO)
            .counters(&metrics::PENDING_PEER_MONITORING_SERVER_NETWORK_EVENTS),
    )
}

pub type NetworkRequest = (
    PeerId,
    ProtocolId,
    PeerMonitoringServiceRequest,
    ResponseSender,
);

/// A stream of requests from the network. Each request also comes with a
/// callback to send the response.
pub struct PeerMonitoringServiceNetworkEvents(BoxStream<'static, NetworkRequest>);

impl NewNetworkEvents for PeerMonitoringServiceNetworkEvents {
    fn new(
        peer_manager_notification_receiver: aptos_channel::Receiver<
            (PeerId, ProtocolId),
            PeerManagerNotification,
        >,
        connection_notification_receiver: aptos_channel::Receiver<PeerId, ConnectionNotification>,
    ) -> Self {
        let events = NetworkEvents::new(
            peer_manager_notification_receiver,
            connection_notification_receiver,
        )
        .filter_map(|event| future::ready(Self::event_to_request(event)))
        .boxed();

        Self(events)
    }
}

impl PeerMonitoringServiceNetworkEvents {
    fn event_to_request(event: Event<PeerMonitoringServiceMessage>) -> Option<NetworkRequest> {
        match event {
            Event::RpcRequest(
                peer_id,
                PeerMonitoringServiceMessage::Request(request),
                protocol_id,
                response_tx,
            ) => {
                let response_tx = ResponseSender::new(response_tx);
                Some((peer_id, protocol_id, request, response_tx))
            }
            _ => None, // We don't use DirectSend and don't care about connection events
        }
    }
}

impl Stream for PeerMonitoringServiceNetworkEvents {
    type Item = NetworkRequest;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.0).poll_next(cx)
    }
}

/// A channel for fulfilling a pending PeerMonitoringService RPC request.
/// Provides a more strongly typed interface around the raw RPC response channel.
pub struct ResponseSender {
    response_sender: oneshot::Sender<Result<Bytes, RpcError>>,
}

impl ResponseSender {
    pub fn new(response_sender: oneshot::Sender<Result<Bytes, RpcError>>) -> Self {
        Self { response_sender }
    }

    pub fn send(self, response: Result<PeerMonitoringServiceResponse>) {
        let msg = PeerMonitoringServiceMessage::Response(response);
        let result = bcs::to_bytes(&msg)
            .map(Bytes::from)
            .map_err(RpcError::BcsError);
        let _ = self.response_sender.send(result);
    }
}
