// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network_interface::{JWKConsensusNetworkClient, RPC},
    types::JWKConsensusMsg,
};
use anyhow::bail;
use velor_channels::{velor_channel, message_queues::QueueStyle};
use velor_config::network_id::NetworkId;
use velor_consensus_types::common::Author;
#[cfg(test)]
use velor_infallible::RwLock;
use velor_logger::warn;
use velor_network::{
    application::interface::{NetworkClient, NetworkServiceEvents},
    protocols::network::{Event, RpcError},
    ProtocolId,
};
use velor_reliable_broadcast::RBNetworkSender;
use velor_types::account_address::AccountAddress;
use bytes::Bytes;
use futures::Stream;
use futures_channel::oneshot;
use futures_util::{
    stream::{select, select_all, StreamExt},
    SinkExt,
};
#[cfg(test)]
use std::sync::Arc;
use std::{collections::HashMap, time::Duration};

pub struct IncomingRpcRequest {
    pub msg: JWKConsensusMsg,
    pub sender: AccountAddress,
    pub response_sender: Box<dyn RpcResponseSender>,
}

pub struct NetworkSender {
    author: AccountAddress,
    jwk_network_client: JWKConsensusNetworkClient<NetworkClient<JWKConsensusMsg>>,
    self_sender: velor_channels::Sender<Event<JWKConsensusMsg>>,
}

impl NetworkSender {
    pub fn new(
        author: AccountAddress,
        jwk_network_client: JWKConsensusNetworkClient<NetworkClient<JWKConsensusMsg>>,
        self_sender: velor_channels::Sender<Event<JWKConsensusMsg>>,
    ) -> Self {
        Self {
            author,
            jwk_network_client,
            self_sender,
        }
    }
}

#[async_trait::async_trait]
impl RBNetworkSender<JWKConsensusMsg> for NetworkSender {
    async fn send_rb_rpc_raw(
        &self,
        receiver: AccountAddress,
        raw_message: Bytes,
        timeout: Duration,
    ) -> anyhow::Result<JWKConsensusMsg> {
        Ok(self
            .jwk_network_client
            .send_rpc_raw(receiver, raw_message, timeout)
            .await?)
    }

    async fn send_rb_rpc(
        &self,
        receiver: AccountAddress,
        message: JWKConsensusMsg,
        timeout: Duration,
    ) -> anyhow::Result<JWKConsensusMsg> {
        if receiver == self.author {
            let (tx, rx) = oneshot::channel();
            let protocol = RPC[0];
            let self_msg = Event::RpcRequest(self.author, message, protocol, tx);
            self.self_sender.clone().send(self_msg).await?;
            if let Ok(Ok(Ok(bytes))) = tokio::time::timeout(timeout, rx).await {
                let response_msg =
                    tokio::task::spawn_blocking(move || protocol.from_bytes(&bytes)).await??;
                Ok(response_msg)
            } else {
                bail!("self rpc failed");
            }
        } else {
            let result = self
                .jwk_network_client
                .send_rpc(receiver, message, timeout)
                .await?;
            Ok(result)
        }
    }

    fn to_bytes_by_protocol(
        &self,
        peers: Vec<Author>,
        message: JWKConsensusMsg,
    ) -> Result<HashMap<Author, bytes::Bytes>, anyhow::Error> {
        self.jwk_network_client.to_bytes_by_protocol(peers, message)
    }

    fn sort_peers_by_latency(&self, peers: &mut [AccountAddress]) {
        self.jwk_network_client.sort_peers_by_latency(peers)
    }
}

pub trait RpcResponseSender: Send + Sync {
    fn send(&mut self, response: anyhow::Result<JWKConsensusMsg>);
}

pub struct RealRpcResponseSender {
    pub inner: Option<oneshot::Sender<Result<Bytes, RpcError>>>,
    pub protocol: ProtocolId,
}

impl RpcResponseSender for RealRpcResponseSender {
    fn send(&mut self, response: anyhow::Result<JWKConsensusMsg>) {
        let rpc_response = response
            .and_then(|msg| self.protocol.to_bytes(&msg).map(Bytes::from))
            .map_err(RpcError::ApplicationError);
        if let Some(tx) = self.inner.take() {
            let _ = tx.send(rpc_response);
        }
    }
}

#[cfg(test)]
pub struct DummyRpcResponseSender {
    pub rpc_response_collector: Arc<RwLock<Vec<anyhow::Result<JWKConsensusMsg>>>>,
}

#[cfg(test)]
impl DummyRpcResponseSender {
    pub fn new(rpc_response_collector: Arc<RwLock<Vec<anyhow::Result<JWKConsensusMsg>>>>) -> Self {
        Self {
            rpc_response_collector,
        }
    }
}

#[cfg(test)]
impl RpcResponseSender for DummyRpcResponseSender {
    fn send(&mut self, response: anyhow::Result<JWKConsensusMsg>) {
        self.rpc_response_collector.write().push(response);
    }
}

pub struct NetworkReceivers {
    pub rpc_rx: velor_channel::Receiver<AccountAddress, (AccountAddress, IncomingRpcRequest)>,
}

pub struct NetworkTask {
    all_events: Box<dyn Stream<Item = Event<JWKConsensusMsg>> + Send + Unpin>,
    rpc_tx: velor_channel::Sender<AccountAddress, (AccountAddress, IncomingRpcRequest)>,
}

impl NetworkTask {
    /// Establishes the initial connections with the peers and returns the receivers.
    pub fn new(
        network_service_events: NetworkServiceEvents<JWKConsensusMsg>,
        self_receiver: velor_channels::Receiver<Event<JWKConsensusMsg>>,
    ) -> (NetworkTask, NetworkReceivers) {
        let (rpc_tx, rpc_rx) = velor_channel::new(QueueStyle::FIFO, 10, None);

        let network_and_events = network_service_events.into_network_and_events();
        if (network_and_events.values().len() != 1)
            || !network_and_events.contains_key(&NetworkId::Validator)
        {
            panic!("The network has not been setup correctly for JWK consensus!");
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
                    // Ignore
                },
            }
        }
    }
}
