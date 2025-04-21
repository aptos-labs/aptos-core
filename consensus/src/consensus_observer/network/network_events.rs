// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::network::observer_message::{
    ConsensusObserverMessage, ConsensusObserverResponse,
};
use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_network::{
    application::interface::NetworkServiceEvents,
    protocols::{
        network::{Event, RpcError},
        wire::handshake::v1::ProtocolId,
    },
};
use bytes::Bytes;
use futures::{
    future,
    stream::{select_all, BoxStream, StreamExt},
    Stream,
};
use futures_channel::oneshot;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// A simple wrapper for each network message
pub struct NetworkMessage {
    pub peer_network_id: PeerNetworkId,
    pub protocol_id: Option<ProtocolId>,
    pub consensus_observer_message: ConsensusObserverMessage,
    pub response_sender: Option<ResponseSender>,
}

/// A stream of messages from the network. Each message also comes with
/// a callback to send the response (if the message is an RPC request).
pub struct ConsensusObserverNetworkEvents {
    network_message_stream: BoxStream<'static, NetworkMessage>,
}

impl ConsensusObserverNetworkEvents {
    pub fn new(network_service_events: NetworkServiceEvents<ConsensusObserverMessage>) -> Self {
        // Transform the event streams to also include the network ID
        let network_events: Vec<_> = network_service_events
            .into_network_and_events()
            .into_iter()
            .map(|(network_id, events)| events.map(move |event| (network_id, event)))
            .collect();
        let network_events = select_all(network_events).fuse();

        // Transform each event to a network message
        let network_message_stream = network_events
            .filter_map(|(network_id, event)| {
                future::ready(Self::event_to_request(network_id, event))
            })
            .boxed();

        Self {
            network_message_stream,
        }
    }

    /// Transforms each network event into a network message
    fn event_to_request(
        network_id: NetworkId,
        network_event: Event<ConsensusObserverMessage>,
    ) -> Option<NetworkMessage> {
        match network_event {
            Event::Message(peer_id, consensus_observer_message) => {
                // Transform the direct send event into a network message
                let peer_network_id = PeerNetworkId::new(network_id, peer_id);
                let network_message = NetworkMessage {
                    peer_network_id,
                    protocol_id: None,
                    consensus_observer_message,
                    response_sender: None,
                };
                Some(network_message)
            },
            Event::RpcRequest(peer_id, consensus_observer_message, protocol_id, response_tx) => {
                // Transform the RPC request event into a network message
                let response_sender = ResponseSender::new(response_tx);
                let peer_network_id = PeerNetworkId::new(network_id, peer_id);
                let network_message = NetworkMessage {
                    peer_network_id,
                    protocol_id: Some(protocol_id),
                    consensus_observer_message,
                    response_sender: Some(response_sender),
                };
                Some(network_message)
            },
        }
    }
}

impl Stream for ConsensusObserverNetworkEvents {
    type Item = NetworkMessage;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.network_message_stream).poll_next(cx)
    }
}

/// A channel for fulfilling a pending consensus observer RPC request
pub struct ResponseSender {
    response_tx: oneshot::Sender<Result<Bytes, RpcError>>,
}

impl ResponseSender {
    pub fn new(response_tx: oneshot::Sender<Result<Bytes, RpcError>>) -> Self {
        Self { response_tx }
    }

    #[cfg(test)]
    /// Creates a new response sender for testing purposes.
    pub fn new_for_test() -> Self {
        Self {
            response_tx: oneshot::channel().0,
        }
    }

    /// Send the response to the pending RPC request
    pub fn send(self, response: ConsensusObserverResponse) {
        // Create and serialize the response message
        let consensus_observer_message = ConsensusObserverMessage::Response(response);
        let result = bcs::to_bytes(&consensus_observer_message)
            .map(Bytes::from)
            .map_err(RpcError::BcsError);

        // Send the response
        let _ = self.response_tx.send(result);
    }
}
