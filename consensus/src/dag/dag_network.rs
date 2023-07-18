// Copyright © Aptos Foundation

use crate::network_interface::ConsensusMsg;
use aptos_consensus_types::common::Author;
use aptos_time_service::{Interval, TimeService, TimeServiceTrait};
use async_trait::async_trait;
use futures::{
    stream::{FusedStream, FuturesUnordered},
    Future, Stream,
};
use rand::seq::SliceRandom;
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

pub trait RpcHandler {
    type Request;
    type Response;

    fn process(&mut self, message: Self::Request) -> anyhow::Result<Self::Response>;
}

#[async_trait]
pub trait DAGNetworkSender: Send + Sync {
    async fn send_rpc(
        &self,
        receiver: Author,
        message: ConsensusMsg,
        timeout: Duration,
    ) -> anyhow::Result<ConsensusMsg>;

    /// Given a list of potential responders, sending rpc to get response from any of them and could
    /// fallback to more in case of failures.
    async fn send_rpc_with_fallbacks(
        &self,
        responders: Vec<Author>,
        message: ConsensusMsg,
        timeout: Duration,
    ) -> RpcWithFallback;
}

struct Responders {
    peers: Vec<Author>,
    generator: ExponentialNumberGenerator,
}

impl Responders {
    fn new(mut peers: Vec<Author>, initial_request_count: u32, max_request_count: u32) -> Self {
        peers.shuffle(&mut rand::thread_rng());
        Self {
            peers,
            generator: ExponentialNumberGenerator::new(initial_request_count, 2, max_request_count),
        }
    }

    fn next_to_request(&mut self) -> Option<Vec<Author>> {
        let count = self.generator.next().expect("should return a number");

        if self.peers.is_empty() {
            return None;
        }
        Some(
            self.peers.split_off(
                self.peers
                    .len()
                    .checked_sub(count as usize)
                    .unwrap_or(0),
            ),
        )
    }
}

pub struct RpcWithFallback {
    responders: Responders,
    message: ConsensusMsg,
    timeout: Duration,

    terminated: bool,
    futures: Pin<
        Box<FuturesUnordered<Pin<Box<dyn Future<Output = anyhow::Result<ConsensusMsg>> + Send>>>>,
    >,
    sender: Arc<dyn DAGNetworkSender>,
    interval: Pin<Box<Interval>>,
}

impl RpcWithFallback {
    pub fn new(
        responders: Vec<Author>,
        message: ConsensusMsg,
        timeout: Duration,
        sender: Arc<dyn DAGNetworkSender>,
        time_service: TimeService,
    ) -> Self {
        Self {
            responders: Responders::new(responders, 1, 4),
            message,
            timeout,

            terminated: false,
            futures: Box::pin(FuturesUnordered::new()),
            sender,
            interval: Box::pin(time_service.interval(timeout)),
        }
    }
}

impl Stream for RpcWithFallback {
    type Item = anyhow::Result<ConsensusMsg>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let send_rpc = move |sender: Arc<dyn DAGNetworkSender>, peer, message, timeout| async move {
            sender.send_rpc(peer, message, timeout).await
        };

        let timeout = matches!(self.interval.as_mut().poll_next(cx), Poll::Ready(_));

        if self.futures.is_empty() || timeout {
            if let Some(peers) = Pin::new(&mut self.responders).next_to_request() {
                for peer in peers {
                    let future = Box::pin(send_rpc(
                        self.sender.clone(),
                        peer,
                        self.message.clone(),
                        self.timeout,
                    ));
                    self.futures.push(future);
                }
            } else if self.futures.is_empty() {
                return Poll::Ready(None);
            }
        }

        let result = futures::ready!(self.futures.as_mut().poll_next(cx));
        Poll::Ready(result)
    }
}

impl FusedStream for RpcWithFallback {
    fn is_terminated(&self) -> bool {
        self.futures.is_empty()
    }
}

struct ExponentialNumberGenerator {
    current: u32,
    factor: u32,
    max_limit: u32,
}

impl ExponentialNumberGenerator {
    fn new(starting_value: u32, factor: u32, max_limit: u32) -> Self {
        ExponentialNumberGenerator {
            current: starting_value,
            factor,
            max_limit,
        }
    }
}

impl Iterator for ExponentialNumberGenerator {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.current;
        if self.current < self.max_limit {
            self.current = (self.current * self.factor).min(self.max_limit)
        }

        Some(result)
    }
}
