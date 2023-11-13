// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::common::Author;
use aptos_logger::info;
use aptos_time_service::{TimeService, TimeServiceTrait};
use async_trait::async_trait;
use futures::{stream::FuturesUnordered, Future, StreamExt};
use std::{collections::HashMap, fmt::Debug, sync::Arc, time::Duration};

pub trait RBMessage: Send + Sync + Clone {}

#[async_trait]
pub trait RBNetworkSender<Req: RBMessage, Res: RBMessage = Req>: Send + Sync {
    async fn send_rb_rpc(
        &self,
        receiver: Author,
        message: Req,
        timeout: Duration,
    ) -> anyhow::Result<Res>;
}

pub trait BroadcastStatus<Req: RBMessage, Res: RBMessage = Req> {
    type Ack: Into<Res> + TryFrom<Res> + Clone;
    type Aggregated;
    type Message: Into<Req> + TryFrom<Req> + Clone;

    fn add(&mut self, peer: Author, ack: Self::Ack) -> anyhow::Result<Option<Self::Aggregated>>;
}

pub struct ReliableBroadcast<Req: RBMessage, TBackoff, Res: RBMessage = Req> {
    validators: Vec<Author>,
    network_sender: Arc<dyn RBNetworkSender<Req, Res>>,
    backoff_policy: TBackoff,
    time_service: TimeService,
    rpc_timeout_duration: Duration,
}

impl<Req, TBackoff, Res> ReliableBroadcast<Req, TBackoff, Res>
where
    Req: RBMessage,
    TBackoff: Iterator<Item = Duration> + Clone,
    Res: RBMessage,
{
    pub fn new(
        validators: Vec<Author>,
        network_sender: Arc<dyn RBNetworkSender<Req, Res>>,
        backoff_policy: TBackoff,
        time_service: TimeService,
        rpc_timeout_duration: Duration,
    ) -> Self {
        Self {
            validators,
            network_sender,
            backoff_policy,
            time_service,
            rpc_timeout_duration,
        }
    }

    pub fn broadcast<S: BroadcastStatus<Req, Res>>(
        &self,
        message: S::Message,
        mut aggregating: S,
    ) -> impl Future<Output = S::Aggregated>
    where
        <<S as BroadcastStatus<Req, Res>>::Ack as TryFrom<Res>>::Error: Debug,
    {
        let receivers: Vec<_> = self.validators.clone();
        let network_sender = self.network_sender.clone();
        let time_service = self.time_service.clone();
        let rpc_timeout_duration = self.rpc_timeout_duration;
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
                            .send_rb_rpc(receiver, message, rpc_timeout_duration)
                            .await,
                    )
                }
            };
            let message: Req = message.into();
            for receiver in receivers {
                fut.push(send_message(receiver, message.clone(), None));
            }
            while let Some((receiver, result)) = fut.next().await {
                match result.and_then(|msg| msg.try_into().map_err(|e| anyhow::anyhow!("{:?}", e)))
                {
                    Ok(ack) => {
                        if let Ok(Some(aggregated)) = aggregating.add(receiver, ack) {
                            return aggregated;
                        }
                    },
                    Err(e) => {
                        info!(error = ?e, "rpc to {} failed", receiver);

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
