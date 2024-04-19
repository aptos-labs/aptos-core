// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_bounded_executor::BoundedExecutor;
use aptos_consensus_types::common::Author;
use aptos_logger::info;
use aptos_time_service::{TimeService, TimeServiceTrait};
use async_trait::async_trait;
use futures::{
    stream::{AbortHandle, FuturesUnordered},
    Future, FutureExt, StreamExt,
};
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

pub trait BroadcastStatus<Req: RBMessage, Res: RBMessage = Req>: Send + Sync + Clone {
    type Aggregated: Send;
    type Message: Into<Req> + TryFrom<Req> + Clone;
    type Response: Into<Res> + TryFrom<Res> + Clone;

    fn add(
        &self,
        peer: Author,
        response: Self::Response,
    ) -> anyhow::Result<Option<Self::Aggregated>>;
}

pub struct ReliableBroadcast<Req: RBMessage, TBackoff, Res: RBMessage = Req> {
    validators: Vec<Author>,
    network_sender: Arc<dyn RBNetworkSender<Req, Res>>,
    backoff_policy: TBackoff,
    time_service: TimeService,
    rpc_timeout_duration: Duration,
    executor: BoundedExecutor,
}

impl<Req, TBackoff, Res> ReliableBroadcast<Req, TBackoff, Res>
where
    Req: RBMessage + 'static,
    TBackoff: Iterator<Item = Duration> + Clone + 'static,
    Res: RBMessage + 'static,
{
    pub fn new(
        validators: Vec<Author>,
        network_sender: Arc<dyn RBNetworkSender<Req, Res>>,
        backoff_policy: TBackoff,
        time_service: TimeService,
        rpc_timeout_duration: Duration,
        executor: BoundedExecutor,
    ) -> Self {
        Self {
            validators,
            network_sender,
            backoff_policy,
            time_service,
            rpc_timeout_duration,
            executor,
        }
    }

    pub fn broadcast<S: BroadcastStatus<Req, Res> + 'static>(
        &self,
        message: S::Message,
        aggregating: S,
    ) -> impl Future<Output = S::Aggregated> + 'static
    where
        <<S as BroadcastStatus<Req, Res>>::Response as TryFrom<Res>>::Error: Debug,
    {
        let receivers: Vec<_> = self.validators.clone();
        self.multicast(message, aggregating, receivers)
    }

    pub fn multicast<S: BroadcastStatus<Req, Res> + 'static>(
        &self,
        message: S::Message,
        aggregating: S,
        receivers: Vec<Author>,
    ) -> impl Future<Output = S::Aggregated> + 'static
    where
        <<S as BroadcastStatus<Req, Res>>::Response as TryFrom<Res>>::Error: Debug,
    {
        let network_sender = self.network_sender.clone();
        let time_service = self.time_service.clone();
        let rpc_timeout_duration = self.rpc_timeout_duration;
        let mut backoff_policies: HashMap<Author, TBackoff> = self
            .validators
            .iter()
            .cloned()
            .map(|author| (author, self.backoff_policy.clone()))
            .collect();
        let executor = self.executor.clone();
        async move {
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
                .boxed()
            };
            let message: Req = message.into();
            let mut rpc_futures = FuturesUnordered::new();
            let mut aggregate_futures = FuturesUnordered::new();
            for receiver in receivers {
                rpc_futures.push(send_message(receiver, message.clone(), None));
            }
            loop {
                tokio::select! {
                    Some((receiver, result)) = rpc_futures.next() => {
                        let aggregating = aggregating.clone();
                        let future = executor.spawn(async move {
                            (
                                    receiver,
                                    result
                                        .and_then(|msg| {
                                            msg.try_into().map_err(|e| anyhow::anyhow!("{:?}", e))
                                        })
                                        .and_then(|ack| aggregating.add(receiver, ack)),
                            )
                        }).await;
                        aggregate_futures.push(future);
                    },
                    Some(result) = aggregate_futures.next() => {
                        let (receiver, result) = result.expect("spawned task must succeed");
                        match result {
                            Ok(may_be_aggragated) => {
                                if let Some(aggregated) = may_be_aggragated {
                                    return aggregated;
                                }
                            },
                            Err(e) => {
                                info!(error = ?e, "rpc to {} failed", receiver);

                                let backoff_strategy = backoff_policies
                                    .get_mut(&receiver)
                                    .expect("should be present");
                                let duration = backoff_strategy.next().expect("should produce value");
                                rpc_futures
                                    .push(send_message(receiver, message.clone(), Some(duration)));
                            },
                        }
                    },
                    else => unreachable!("Should aggregate with all responses")
                }
            }
        }
    }
}

pub struct DropGuard {
    abort_handle: AbortHandle,
}

impl DropGuard {
    pub fn new(abort_handle: AbortHandle) -> Self {
        Self { abort_handle }
    }
}

impl Drop for DropGuard {
    fn drop(&mut self) {
        self.abort_handle.abort();
    }
}

#[cfg(test)]
mod tests;
