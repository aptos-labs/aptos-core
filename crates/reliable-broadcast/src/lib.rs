// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::common::Author;
use aptos_time_service::{TimeService, TimeServiceTrait};
use async_trait::async_trait;
use futures::{stream::FuturesUnordered, Future, StreamExt};
use std::{collections::HashMap, sync::Arc, time::Duration};

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

pub struct ReliableBroadcast<M: RBMessage, TBackoff> {
    validators: Vec<Author>,
    network_sender: Arc<dyn RBNetworkSender<M>>,
    backoff_policy: TBackoff,
    time_service: TimeService,
}

impl<M, TBackoff> ReliableBroadcast<M, TBackoff>
where
    M: RBMessage,
    TBackoff: Iterator<Item = Duration> + Clone,
{
    pub fn new(
        validators: Vec<Author>,
        network_sender: Arc<dyn RBNetworkSender<M>>,
        backoff_policy: TBackoff,
        time_service: TimeService,
    ) -> Self {
        Self {
            validators,
            network_sender,
            backoff_policy,
            time_service,
        }
    }

    pub fn broadcast<S: BroadcastStatus<M>>(
        &self,
        message: S::Message,
        mut aggregating: S,
    ) -> impl Future<Output = S::Aggregated> {
        let receivers: Vec<_> = self.validators.clone();
        let network_sender = self.network_sender.clone();
        let time_service = self.time_service.clone();
        let mut backoff_policies: HashMap<Author, TBackoff> = self
            .validators
            .iter()
            .cloned()
            .map(|author| (author, self.backoff_policy.clone()))
            .collect();
        async move {
            let mut fut = FuturesUnordered::new();
            let send_message = |receiver, message, sleep_duration: Option<Duration>| {
                let network_sender = network_sender.clone();
                let time_service = time_service.clone();
                async move {
                    if let Some(duration) = sleep_duration {
                        time_service.sleep(duration).await;
                    }
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
                fut.push(send_message(receiver, message.clone(), None));
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
                    Err(_) => {
                        let backoff_strategy = backoff_policies
                            .get_mut(&receiver)
                            .expect("should be present");
                        let duration = backoff_strategy.next().expect("should produce value");
                        fut.push(send_message(receiver, message.clone(), Some(duration)));
                    },
                }
            }
            unreachable!("Should aggregate with all responses");
        }
    }
}

#[cfg(test)]
mod tests;
