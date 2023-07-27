// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::common::Author;
use async_trait::async_trait;
use futures::{stream::FuturesUnordered, Future, StreamExt};
use std::{sync::Arc, time::Duration};

pub trait RBMessage: Send + Sync + Clone {}

#[async_trait]
pub trait RBNetworkSender<M: RBMessage>: Send + Sync {
    async fn send_rpc(&self, receiver: Author, message: M, timeout: Duration) -> anyhow::Result<M>;
}

pub trait BroadcastStatus<M: RBMessage> {
    type Ack: Into<M> + TryFrom<M> + Clone;
    type Aggregated;
    type Message: Into<M> + TryFrom<M> + Clone;

    fn add(&mut self, peer: Author, ack: Self::Ack) -> anyhow::Result<Option<Self::Aggregated>>;
}

pub struct ReliableBroadcast<M: RBMessage> {
    validators: Vec<Author>,
    network_sender: Arc<dyn RBNetworkSender<M>>,
}

impl<M> ReliableBroadcast<M>
where
    M: RBMessage,
{
    pub fn new(validators: Vec<Author>, network_sender: Arc<dyn RBNetworkSender<M>>) -> Self {
        Self {
            validators,
            network_sender,
        }
    }

    pub fn broadcast<S: BroadcastStatus<M>>(
        &self,
        message: S::Message,
        mut aggregating: S,
    ) -> impl Future<Output = S::Aggregated> {
        let receivers: Vec<_> = self.validators.clone();
        let network_sender = self.network_sender.clone();
        async move {
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
            let message: M = message.into();
            for receiver in receivers {
                fut.push(send_message(receiver, message.clone()));
            }
            while let Some((receiver, result)) = fut.next().await {
                match result {
                    Ok(msg) => {
                        if let Ok(ack) = msg.try_into() {
                            if let Ok(Some(aggregated)) = aggregating.add(receiver, ack) {
                                return aggregated;
                            }
                        }
                    },
                    Err(_) => fut.push(send_message(receiver, message.clone())),
                }
            }
            unreachable!("Should aggregate with all responses");
        }
    }
}

#[cfg(test)]
mod tests;
