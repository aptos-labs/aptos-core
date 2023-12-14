// Copyright © Aptos Foundation

use crate::{
    monitor,
    network_interface::{DKGMsg, DKGNetworkClient, RPC_DKG},
    DKGMessage, DKGNetworkMessage,
};
use anyhow::{anyhow, bail};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::network_id::NetworkId;
use aptos_consensus_types::common::Author;
use aptos_logger::{debug, warn};
use aptos_network::{
    application::interface::{NetworkClient, NetworkServiceEvents},
    protocols::network::{Event, RpcError},
    ProtocolId,
};
use aptos_reliable_broadcast::RBNetworkSender;
use aptos_types::validator_verifier::ValidatorVerifier;
use async_trait::async_trait;
use bytes::Bytes;
use futures::{
    stream::{select, select_all},
    SinkExt, Stream, StreamExt,
};
use futures_channel::oneshot;
use move_core_types::account_address::AccountAddress;
use std::{
    mem::{discriminant, Discriminant},
    time::Duration,
};
use tokio::time::timeout;

#[derive(Debug)]
pub enum IncomingRpcRequest {
    DKG(IncomingDKGRequest),
}

#[derive(Debug)]
pub struct IncomingDKGRequest {
    pub req: DKGNetworkMessage,
    pub sender: Author,
    pub protocol: ProtocolId,
    pub response_sender: oneshot::Sender<Result<Bytes, RpcError>>,
}

/// Implements the actual networking support for all oracle messaging.
#[derive(Clone)]
pub struct NetworkSender {
    author: Author,
    oracle_network_client: DKGNetworkClient<NetworkClient<DKGMsg>>,
    // Self sender and self receivers provide a shortcut for sending the messages to itself.
    // (self sending is not supported by the networking API).
    self_sender: aptos_channels::Sender<Event<DKGMsg>>,
    _validators: ValidatorVerifier,
    _time_service: aptos_time_service::TimeService,
}

impl NetworkSender {
    pub fn new(
        author: Author,
        oracle_network_client: DKGNetworkClient<NetworkClient<DKGMsg>>,
        self_sender: aptos_channels::Sender<Event<DKGMsg>>,
        validators: ValidatorVerifier,
    ) -> Self {
        NetworkSender {
            author,
            oracle_network_client,
            self_sender,
            _validators: validators,
            _time_service: aptos_time_service::TimeService::real(),
        }
    }

    pub fn author(&self) -> Author {
        self.author
    }

    pub async fn send_rpc(
        &self,
        receiver: Author,
        msg: DKGMsg,
        timeout_duration: Duration,
    ) -> anyhow::Result<DKGMsg> {
        debug!("[DKG] network::send_rpc: BEGIN");
        if receiver == self.author() {
            let (tx, rx) = oneshot::channel();
            let protocol = RPC_DKG[0];
            let self_msg = Event::RpcRequest(receiver, msg.clone(), RPC_DKG[0], tx);
            self.self_sender.clone().send(self_msg).await?;
            if let Ok(Ok(Ok(bytes))) = timeout(timeout_duration, rx).await {
                Ok(protocol.from_bytes(&bytes)?)
            } else {
                bail!("self rpc failed");
            }
        } else {
            Ok(monitor!(
                "send_rpc",
                self.oracle_network_client
                    .send_rpc(receiver, msg, timeout_duration)
                    .await
            )?)
        }
    }
}

#[async_trait]
impl RBNetworkSender<DKGMessage> for NetworkSender {
    async fn send_rb_rpc(
        &self,
        receiver: Author,
        message: DKGMessage,
        timeout: Duration,
    ) -> anyhow::Result<DKGMessage> {
        self.send_rpc(receiver, DKGMsg::from(message), timeout)
            .await
            .map_err(|e| anyhow!("invalid rpc response: {}", e))
            .and_then(DKGMessage::try_from)
    }
}

pub struct NetworkReceivers {
    /// Provide a LIFO buffer for each (Author, MessageType) key
    pub dkg_messages:
        aptos_channel::Receiver<(AccountAddress, Discriminant<DKGMsg>), (AccountAddress, DKGMsg)>,
    pub rpc_rx: aptos_channel::Receiver<
        (AccountAddress, Discriminant<IncomingRpcRequest>),
        (AccountAddress, IncomingRpcRequest),
    >,
}

pub struct NetworkTask {
    _oracle_messages_tx:
        aptos_channel::Sender<(AccountAddress, Discriminant<DKGMsg>), (AccountAddress, DKGMsg)>,
    all_events: Box<dyn Stream<Item = Event<DKGMsg>> + Send + Unpin>,
    rpc_tx: aptos_channel::Sender<
        (AccountAddress, Discriminant<IncomingRpcRequest>),
        (AccountAddress, IncomingRpcRequest),
    >,
}

impl NetworkTask {
    /// Establishes the initial connections with the peers and returns the receivers.
    pub fn new(
        network_service_events: NetworkServiceEvents<DKGMsg>,
        self_receiver: aptos_channels::Receiver<Event<DKGMsg>>,
    ) -> (NetworkTask, NetworkReceivers) {
        let (oracle_messages_tx, oracle_messages) = aptos_channel::new(QueueStyle::FIFO, 10, None);

        let (rpc_tx, rpc_rx) = aptos_channel::new(QueueStyle::FIFO, 10, None);

        let network_and_events = network_service_events.into_network_and_events();
        if (network_and_events.values().len() != 1)
            || !network_and_events.contains_key(&NetworkId::Validator)
        {
            panic!("The network has not been setup correctly for consensus!");
        }

        // Collect all the network events into a single stream
        let network_events: Vec<_> = network_and_events.into_values().collect();
        let network_events = select_all(network_events).fuse();
        let all_events = Box::new(select(network_events, self_receiver));

        (
            NetworkTask {
                _oracle_messages_tx: oracle_messages_tx,
                rpc_tx,
                all_events,
            },
            NetworkReceivers {
                dkg_messages: oracle_messages,
                rpc_rx,
            },
        )
    }

    pub async fn start(mut self) {
        while let Some(message) = self.all_events.next().await {
            match message {
                Event::Message(_peer_id, msg) => match msg {
                    DKGMsg::DKGMessage(_msg) => {
                        todo!()
                    },
                },
                Event::RpcRequest(peer_id, msg, protocol, response_sender) => {
                    let req = match msg {
                        DKGMsg::DKGMessage(obj) => IncomingRpcRequest::DKG(IncomingDKGRequest {
                            req: *obj,
                            sender: peer_id,
                            protocol,
                            response_sender,
                        }),
                    };

                    if let Err(e) = self
                        .rpc_tx
                        .push((peer_id, discriminant(&req)), (peer_id, req))
                    {
                        warn!(error = ?e, "aptos channel closed");
                    };
                },
                _ => {
                    // Ignore `NewPeer` and `LostPeer` events
                },
            }
        }
    }
}
