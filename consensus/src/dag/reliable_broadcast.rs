// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::network_interface::ConsensusMsg;
use aptos_consensus_types::common::Author;
use async_trait::async_trait;
use futures::{stream::FuturesUnordered, StreamExt};
use std::{future::Future, sync::Arc, time::Duration};
use tokio::sync::oneshot;

pub trait DAGMessage: Sized + Clone {
    fn from_network_message(msg: ConsensusMsg) -> anyhow::Result<Self>;

    fn into_network_message(self) -> ConsensusMsg;
}

pub trait BroadcastStatus {
    type Message: DAGMessage;
    type Ack: DAGMessage;
    type Aggregated;

    fn empty(validators: Vec<Author>) -> Self;

    fn add(&mut self, peer: Author, ack: Self::Ack) -> anyhow::Result<Option<Self::Aggregated>>;
}

#[async_trait]
pub trait DAGNetworkSender: Send + Sync {
    async fn send_rpc(
        &self,
        receiver: Author,
        message: ConsensusMsg,
        timeout: Duration,
    ) -> anyhow::Result<ConsensusMsg>;
}

pub struct ReliableBroadcast {
    validators: Vec<Author>,
    network_sender: Arc<dyn DAGNetworkSender>,
}

impl ReliableBroadcast {
    pub fn new(validators: Vec<Author>, network_sender: Arc<dyn DAGNetworkSender>) -> Self {
        Self {
            validators,
            network_sender,
        }
    }

    pub fn broadcast<S: BroadcastStatus>(
        &self,
        message: S::Message,
        return_tx: oneshot::Sender<S::Aggregated>,
        mut cancel_rx: oneshot::Receiver<()>,
    ) -> impl Future<Output = ()> {
        let receivers: Vec<_> = self.validators.clone();
        let network_message = message.into_network_message();
        let network_sender = self.network_sender.clone();
        async move {
            let mut aggregating = S::empty(receivers.clone());
            let mut fut = FuturesUnordered::new();
            let send_message = |receiver, message| {
                let network_sender = network_sender.clone();
                async move {
                    (
                        receiver,
                        network_sender
                            .send_rpc(receiver, message, Duration::from_millis(500))
                            .await,
                    )
                }
            };
            for receiver in receivers {
                fut.push(send_message(receiver, network_message.clone()));
            }
            loop {
                tokio::select! {
                    Some((receiver, result)) = fut.next() => {
                        match result {
                            Ok(msg) =>  {
                                if let Ok(ack) = S::Ack::from_network_message(msg) {
                                    if let Ok(Some(aggregated)) = aggregating.add(receiver, ack) {
                                        let _ = return_tx.send(aggregated);
                                        return;
                                    }
                                }
                            },
                            Err(_) => fut.push(send_message(receiver, network_message.clone())),
                        }
                    }
                    _ = &mut cancel_rx => {
                        return;
                    }
                }
            }
        }
    }
}
