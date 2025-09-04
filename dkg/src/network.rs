// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network_interface::{DKGNetworkClient, RPC},
    DKGMessage,
};
use anyhow::bail;
use velor_channels::{velor_channel, message_queues::QueueStyle};
use velor_config::network_id::NetworkId;
use velor_infallible::RwLock;
use velor_logger::warn;
use velor_network::{
    application::interface::{NetworkClient, NetworkServiceEvents},
    protocols::network::{Event, RpcError},
    ProtocolId,
};
use velor_reliable_broadcast::RBNetworkSender;
use async_trait::async_trait;
use bytes::Bytes;
use futures::{
    stream::{select, select_all},
    SinkExt, Stream, StreamExt,
};
use futures_channel::oneshot;
use move_core_types::account_address::AccountAddress;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time::timeout;

pub struct IncomingRpcRequest {
    pub msg: DKGMessage,
    pub sender: AccountAddress,
    pub response_sender: Box<dyn RpcResponseSender>,
}

/// Implements the actual networking support for all DKG messaging.
#[derive(Clone)]
pub struct NetworkSender {
    author: AccountAddress,
    dkg_network_client: DKGNetworkClient<NetworkClient<DKGMessage>>,
    // Self sender and self receivers provide a shortcut for sending the messages to itself.
    // (self sending is not supported by the networking API).
    self_sender: velor_channels::Sender<Event<DKGMessage>>,
}

impl NetworkSender {
    pub fn new(
        author: AccountAddress,
        dkg_network_client: DKGNetworkClient<NetworkClient<DKGMessage>>,
        self_sender: velor_channels::Sender<Event<DKGMessage>>,
    ) -> Self {
        NetworkSender {
            author,
            dkg_network_client,
            self_sender,
        }
    }

    pub fn author(&self) -> AccountAddress {
        self.author
    }

    pub async fn send_rpc(
        &self,
        receiver: AccountAddress,
        msg: DKGMessage,
        timeout_duration: Duration,
    ) -> anyhow::Result<DKGMessage> {
        if receiver == self.author() {
            let (tx, rx) = oneshot::channel();
            let protocol = RPC[0];
            let self_msg = Event::RpcRequest(self.author, msg.clone(), RPC[0], tx);
            self.self_sender.clone().send(self_msg).await?;
            if let Ok(Ok(Ok(bytes))) = timeout(timeout_duration, rx).await {
                let response_msg =
                    tokio::task::spawn_blocking(move || protocol.from_bytes(&bytes)).await??;
                Ok(response_msg)
            } else {
                bail!("self rpc failed");
            }
        } else {
            Ok(self
                .dkg_network_client
                .send_rpc(receiver, msg, timeout_duration)
                .await?)
        }
    }
}

#[async_trait]
impl RBNetworkSender<DKGMessage> for NetworkSender {
    async fn send_rb_rpc_raw(
        &self,
        receiver: AccountAddress,
        raw_message: Bytes,
        timeout: Duration,
    ) -> anyhow::Result<DKGMessage> {
        Ok(self
            .dkg_network_client
            .send_rpc_raw(receiver, raw_message, timeout)
            .await?)
    }

    async fn send_rb_rpc(
        &self,
        receiver: AccountAddress,
        message: DKGMessage,
        timeout: Duration,
    ) -> anyhow::Result<DKGMessage> {
        self.send_rpc(receiver, message, timeout).await
    }

    fn to_bytes_by_protocol(
        &self,
        peers: Vec<AccountAddress>,
        message: DKGMessage,
    ) -> anyhow::Result<HashMap<AccountAddress, Bytes>> {
        self.dkg_network_client.to_bytes_by_protocol(peers, message)
    }

    fn sort_peers_by_latency(&self, peers: &mut [AccountAddress]) {
        self.dkg_network_client.sort_peers_by_latency(peers)
    }
}

pub struct NetworkReceivers {
    pub rpc_rx: velor_channel::Receiver<AccountAddress, (AccountAddress, IncomingRpcRequest)>,
}

pub struct NetworkTask {
    all_events: Box<dyn Stream<Item = Event<DKGMessage>> + Send + Unpin>,
    rpc_tx: velor_channel::Sender<AccountAddress, (AccountAddress, IncomingRpcRequest)>,
}

impl NetworkTask {
    /// Establishes the initial connections with the peers and returns the receivers.
    pub fn new(
        network_service_events: NetworkServiceEvents<DKGMessage>,
        self_receiver: velor_channels::Receiver<Event<DKGMessage>>,
    ) -> (NetworkTask, NetworkReceivers) {
        let (rpc_tx, rpc_rx) = velor_channel::new(QueueStyle::FIFO, 10, None);

        let network_and_events = network_service_events.into_network_and_events();
        if (network_and_events.values().len() != 1)
            || !network_and_events.contains_key(&NetworkId::Validator)
        {
            panic!("The network has not been setup correctly for DKG!");
        }

        // Collect all the network events into a single stream
        let network_events: Vec<_> = network_and_events.into_values().collect();
        let network_events = select_all(network_events).fuse();
        let all_events = Box::new(select(network_events, self_receiver));

        (NetworkTask { rpc_tx, all_events }, NetworkReceivers {
            rpc_rx,
        })
    }

    pub async fn start(mut self) {
        while let Some(message) = self.all_events.next().await {
            match message {
                Event::RpcRequest(peer_id, msg, protocol, response_sender) => {
                    let req = IncomingRpcRequest {
                        msg,
                        sender: peer_id,
                        response_sender: Box::new(RealRpcResponseSender {
                            inner: Some(response_sender),
                            protocol,
                        }),
                    };

                    if let Err(e) = self.rpc_tx.push(peer_id, (peer_id, req)) {
                        warn!(error = ?e, "velor channel closed");
                    };
                },
                _ => {
                    // Ignored. Currently only RPC is used.
                },
            }
        }
    }
}

pub trait RpcResponseSender: Send + Sync {
    fn send(&mut self, response: anyhow::Result<DKGMessage>);
}

pub struct RealRpcResponseSender {
    pub inner: Option<oneshot::Sender<Result<Bytes, RpcError>>>,
    pub protocol: ProtocolId,
}

impl RealRpcResponseSender {
    pub fn new(raw_sender: oneshot::Sender<Result<Bytes, RpcError>>, protocol: ProtocolId) -> Self {
        Self {
            inner: Some(raw_sender),
            protocol,
        }
    }
}

impl RpcResponseSender for RealRpcResponseSender {
    fn send(&mut self, response: anyhow::Result<DKGMessage>) {
        let rpc_response = response
            .and_then(|dkg_msg| self.protocol.to_bytes(&dkg_msg).map(Bytes::from))
            .map_err(RpcError::ApplicationError);
        let _ = self.inner.take().unwrap().send(rpc_response); // May not succeed.
    }
}

pub struct DummyRpcResponseSender {
    pub rpc_response_collector: Arc<RwLock<Vec<anyhow::Result<DKGMessage>>>>,
}

impl DummyRpcResponseSender {
    pub fn new(rpc_response_collector: Arc<RwLock<Vec<anyhow::Result<DKGMessage>>>>) -> Self {
        Self {
            rpc_response_collector,
        }
    }
}

impl RpcResponseSender for DummyRpcResponseSender {
    fn send(&mut self, response: anyhow::Result<DKGMessage>) {
        self.rpc_response_collector.write().push(response);
    }
}
