// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_bounded_executor::BoundedExecutor;
use velor_consensus_types::common::Author;
use velor_logger::{debug, sample, sample::SampleRate, warn};
use velor_time_service::{TimeService, TimeServiceTrait};
use async_trait::async_trait;
use bytes::Bytes;
use futures::{
    stream::{AbortHandle, FuturesUnordered},
    Future, FutureExt, StreamExt,
};
use std::{collections::HashMap, fmt::Debug, sync::Arc, time::Duration};

pub trait RBMessage: Send + Sync + Clone {}

#[async_trait]
pub trait RBNetworkSender<Req: RBMessage, Res: RBMessage = Req>: Send + Sync {
    async fn send_rb_rpc_raw(
        &self,
        receiver: Author,
        message: Bytes,
        timeout: Duration,
    ) -> anyhow::Result<Res>;

    async fn send_rb_rpc(
        &self,
        receiver: Author,
        message: Req,
        timeout: Duration,
    ) -> anyhow::Result<Res>;

    /// Serializes the given message into bytes using each peers' preferred protocol.
    fn to_bytes_by_protocol(
        &self,
        peers: Vec<Author>,
        message: Req,
    ) -> anyhow::Result<HashMap<Author, Bytes>>;

    fn sort_peers_by_latency(&self, peers: &mut [Author]);
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
    self_author: Author,
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
        self_author: Author,
        validators: Vec<Author>,
        network_sender: Arc<dyn RBNetworkSender<Req, Res>>,
        backoff_policy: TBackoff,
        time_service: TimeService,
        rpc_timeout_duration: Duration,
        executor: BoundedExecutor,
    ) -> Self {
        Self {
            self_author,
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
    ) -> impl Future<Output = anyhow::Result<S::Aggregated>> + 'static
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
    ) -> impl Future<Output = anyhow::Result<S::Aggregated>> + 'static
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
        let self_author = self.self_author;
        async move {
            let message: Req = message.into();

            let peers = receivers.clone();
            let sender = network_sender.clone();
            let message_clone = message.clone();
            let protocols = Arc::new(
                tokio::task::spawn_blocking(move || {
                    sender.to_bytes_by_protocol(peers, message_clone)
                })
                .await??,
            );

            let send_message = |receiver, sleep_duration: Option<Duration>| {
                let network_sender = network_sender.clone();
                let time_service = time_service.clone();
                let message = message.clone();
                let protocols = protocols.clone();
                async move {
                    if let Some(duration) = sleep_duration {
                        time_service.sleep(duration).await;
                    }
                    let send_fut = if receiver == self_author {
                        network_sender.send_rb_rpc(receiver, message, rpc_timeout_duration)
                    } else if let Some(raw_message) = protocols.get(&receiver).cloned() {
                        network_sender.send_rb_rpc_raw(receiver, raw_message, rpc_timeout_duration)
                    } else {
                        network_sender.send_rb_rpc(receiver, message, rpc_timeout_duration)
                    };
                    (receiver, send_fut.await)
                }
                .boxed()
            };

            let mut rpc_futures = FuturesUnordered::new();
            let mut aggregate_futures = FuturesUnordered::new();

            let mut receivers = receivers;
            network_sender.sort_peers_by_latency(&mut receivers);

            for receiver in receivers {
                rpc_futures.push(send_message(receiver, None));
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
                                    return Ok(aggregated);
                                }
                            },
                            Err(e) => {
                                log_rpc_failure(e, receiver);

                                let backoff_strategy = backoff_policies
                                    .get_mut(&receiver)
                                    .expect("should be present");
                                let duration = backoff_strategy.next().expect("should produce value");
                                rpc_futures
                                    .push(send_message(receiver, Some(duration)));
                            },
                        }
                    },
                    else => unreachable!("Should aggregate with all responses")
                }
            }
        }
    }
}

fn log_rpc_failure(error: anyhow::Error, receiver: Author) {
    // Log a sampled warning (to prevent spam)
    sample!(
        SampleRate::Duration(Duration::from_secs(30)),
        warn!("[sampled] rpc to {} failed, error {:#}", receiver, error)
    );

    // Log at the debug level (this is useful for debugging
    // and won't spam the logs in a production environment).
    debug!("rpc to {} failed, error {:#}", receiver, error);
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
