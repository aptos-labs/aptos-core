// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_config::network_id::{NetworkId, PeerNetworkId};
use velor_network::{
    application::interface::NetworkServiceEvents,
    protocols::network::{Event, RpcError},
    ProtocolId,
};
use velor_peer_monitoring_service_types::{
    request::PeerMonitoringServiceRequest, response::PeerMonitoringServiceResponse,
    PeerMonitoringServiceMessage, Result,
};
use bytes::Bytes;
use futures::{
    channel::oneshot,
    future,
    stream::{select_all, BoxStream, Stream, StreamExt},
};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// A simple wrapper for each network request
pub struct NetworkRequest {
    pub peer_network_id: PeerNetworkId,
    pub protocol_id: ProtocolId,
    pub peer_monitoring_service_request: PeerMonitoringServiceRequest,
    pub response_sender: ResponseSender,
}

/// A stream of requests from the network. Each request also comes with a
/// callback to send the response.
pub struct PeerMonitoringServiceNetworkEvents {
    network_request_stream: BoxStream<'static, NetworkRequest>,
}

impl PeerMonitoringServiceNetworkEvents {
    pub fn new(network_service_events: NetworkServiceEvents<PeerMonitoringServiceMessage>) -> Self {
        // Transform the event streams to also include the network ID
        let network_events: Vec<_> = network_service_events
            .into_network_and_events()
            .into_iter()
            .map(|(network_id, events)| events.map(move |event| (network_id, event)))
            .collect();
        let network_events = select_all(network_events).fuse();

        // Transform each event to a network request
        let network_request_stream = network_events
            .filter_map(|(network_id, event)| {
                future::ready(Self::event_to_request(network_id, event))
            })
            .boxed();

        Self {
            network_request_stream,
        }
    }

    /// Filters out everything except Rpc requests
    fn event_to_request(
        network_id: NetworkId,
        event: Event<PeerMonitoringServiceMessage>,
    ) -> Option<NetworkRequest> {
        match event {
            Event::RpcRequest(
                peer_id,
                PeerMonitoringServiceMessage::Request(peer_monitoring_service_request),
                protocol_id,
                response_tx,
            ) => {
                let response_sender = ResponseSender::new(response_tx);
                let peer_network_id = PeerNetworkId::new(network_id, peer_id);
                Some(NetworkRequest {
                    peer_network_id,
                    protocol_id,
                    peer_monitoring_service_request,
                    response_sender,
                })
            },
            _ => None, // We don't use direct send and don't care about connection events
        }
    }
}

impl Stream for PeerMonitoringServiceNetworkEvents {
    type Item = NetworkRequest;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.network_request_stream).poll_next(cx)
    }
}

/// A channel for fulfilling a pending PeerMonitoringService RPC request.
/// Provides a more strongly typed interface around the raw RPC response channel.
pub struct ResponseSender {
    response_tx: oneshot::Sender<Result<Bytes, RpcError>>,
}

impl ResponseSender {
    pub fn new(response_tx: oneshot::Sender<Result<Bytes, RpcError>>) -> Self {
        Self { response_tx }
    }

    pub fn send(self, response: Result<PeerMonitoringServiceResponse>) {
        let msg = PeerMonitoringServiceMessage::Response(response);
        let result = bcs::to_bytes(&msg)
            .map(Bytes::from)
            .map_err(RpcError::BcsError);
        let _ = self.response_tx.send(result);
    }
}
