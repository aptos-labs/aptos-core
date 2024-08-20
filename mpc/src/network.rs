// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use anyhow::bail;
use bytes::Bytes;
use futures_channel::oneshot;
use futures_util::SinkExt;
use futures_util::stream::{select, select_all, Stream, StreamExt};
use tokio::time::timeout;
use aptos_channels::aptos_channel;
use aptos_channels::message_queues::QueueStyle;
use aptos_config::network_id::NetworkId;
use aptos_infallible::RwLock;
use aptos_logger::warn;
use aptos_network::application::interface::{NetworkClient, NetworkServiceEvents};
use aptos_network::protocols::network::{Event, RpcError};
use aptos_network::protocols::wire::handshake::v1::ProtocolId;
use aptos_reliable_broadcast::RBNetworkSender;
use move_core_types::account_address::AccountAddress;
use crate::network_interface::{MPCNetworkClient, RPC};
use crate::types::MPCMessage;
use async_trait::async_trait;


pub struct IncomingRpcRequest {
    pub msg: MPCMessage,
    pub sender: AccountAddress,
    pub response_sender: Box<dyn RpcResponseSender>,
}

/// Implements the actual networking support for all MPC messaging.
#[derive(Clone)]
pub struct NetworkSender {
    author: AccountAddress,
    mpc_network_client: MPCNetworkClient<NetworkClient<MPCMessage>>,
    // Self sender and self receivers provide a shortcut for sending the messages to itself.
    // (self sending is not supported by the networking API).
    self_sender: aptos_channels::Sender<Event<MPCMessage>>,
}

impl NetworkSender {
    pub fn new(
        author: AccountAddress,
        mpc_network_client: MPCNetworkClient<NetworkClient<MPCMessage>>,
        self_sender: aptos_channels::Sender<Event<MPCMessage>>,
    ) -> Self {
        NetworkSender {
            author,
            mpc_network_client,
            self_sender,
        }
    }

    pub fn author(&self) -> AccountAddress {
        self.author
    }

    pub async fn send_rpc(
        &self,
        receiver: AccountAddress,
        msg: MPCMessage,
        timeout_duration: Duration,
    ) -> anyhow::Result<MPCMessage> {
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
                .mpc_network_client
                .send_rpc(receiver, msg, timeout_duration)
                .await?)
        }
    }
}

#[async_trait]
impl RBNetworkSender<MPCMessage> for NetworkSender {
    async fn send_rb_rpc_raw(
        &self,
        receiver: AccountAddress,
        raw_message: Bytes,
        timeout: Duration,
    ) -> anyhow::Result<MPCMessage> {
        Ok(self
            .mpc_network_client
            .send_rpc_raw(receiver, raw_message, timeout)
            .await?)
    }

    async fn send_rb_rpc(
        &self,
        receiver: AccountAddress,
        message: MPCMessage,
        timeout: Duration,
    ) -> anyhow::Result<MPCMessage> {
        self.send_rpc(receiver, message, timeout).await
    }

    fn to_bytes_by_protocol(
        &self,
        peers: Vec<AccountAddress>,
        message: MPCMessage,
    ) -> anyhow::Result<HashMap<AccountAddress, Bytes>> {
        self.mpc_network_client.to_bytes_by_protocol(peers, message)
    }

    fn sort_peers_by_latency(&self, peers: &mut [AccountAddress]) {
        self.mpc_network_client.sort_peers_by_latency(peers)
    }
}

pub struct NetworkReceivers {
    pub rpc_rx: aptos_channel::Receiver<AccountAddress, (AccountAddress, IncomingRpcRequest)>,
}

pub struct NetworkTask {
    all_events: Box<dyn Stream<Item = Event<MPCMessage>> + Send + Unpin>,
    rpc_tx: aptos_channel::Sender<AccountAddress, (AccountAddress, IncomingRpcRequest)>,
}

impl NetworkTask {
    /// Establishes the initial connections with the peers and returns the receivers.
    pub fn new(
        network_service_events: NetworkServiceEvents<MPCMessage>,
        self_receiver: aptos_channels::Receiver<Event<MPCMessage>>,
    ) -> (NetworkTask, NetworkReceivers) {
        let (rpc_tx, rpc_rx) = aptos_channel::new(QueueStyle::FIFO, 10, None);

        let network_and_events = network_service_events.into_network_and_events();
        if (network_and_events.values().len() != 1)
            || !network_and_events.contains_key(&NetworkId::Validator)
        {
            panic!("The network has not been setup correctly for MPC!");
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
                        warn!(error = ?e, "aptos channel closed");
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
    fn send(&mut self, response: anyhow::Result<MPCMessage>);
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
    fn send(&mut self, response: anyhow::Result<MPCMessage>) {
        let rpc_response = response
            .and_then(|mpc_msg| self.protocol.to_bytes(&mpc_msg).map(Bytes::from))
            .map_err(RpcError::ApplicationError);
        let _ = self.inner.take().unwrap().send(rpc_response); // May not succeed.
    }
}

pub struct DummyRpcResponseSender {
    pub rpc_response_collector: Arc<RwLock<Vec<anyhow::Result<MPCMessage>>>>,
}

impl DummyRpcResponseSender {
    pub fn new(rpc_response_collector: Arc<RwLock<Vec<anyhow::Result<MPCMessage>>>>) -> Self {
        Self {
            rpc_response_collector,
        }
    }
}

impl RpcResponseSender for DummyRpcResponseSender {
    fn send(&mut self, response: anyhow::Result<MPCMessage>) {
        self.rpc_response_collector.write().push(response);
    }
}
