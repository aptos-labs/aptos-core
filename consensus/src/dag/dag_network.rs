// Copyright © Aptos Foundation

use super::types::DAGMessage;
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
        message: DAGMessage,
        timeout: Duration,
    ) -> anyhow::Result<DAGMessage>;

    /// Given a list of potential responders, sending rpc to get response from any of them and could
    /// fallback to more in case of failures.
    async fn send_rpc_with_fallbacks(
        &self,
        responders: Vec<Author>,
        message: DAGMessage,
        retry_interval: Duration,
        rpc_timeout: Duration,
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
            self.peers
                .split_off(self.peers.len().saturating_sub(count as usize)),
        )
    }
}

pub struct RpcWithFallback {
    responders: Responders,
    message: DAGMessage,
    rpc_timeout: Duration,

    terminated: bool,
    futures: Pin<
        Box<FuturesUnordered<Pin<Box<dyn Future<Output = anyhow::Result<DAGMessage>> + Send>>>>,
    >,
    sender: Arc<dyn DAGNetworkSender>,
    interval: Pin<Box<Interval>>,
}

impl RpcWithFallback {
    pub fn new(
        responders: Vec<Author>,
        message: DAGMessage,
        retry_interval: Duration,
        rpc_timeout: Duration,
        sender: Arc<dyn DAGNetworkSender>,
        time_service: TimeService,
    ) -> Self {
        Self {
            responders: Responders::new(responders, 1, 4),
            message,
            rpc_timeout,

            terminated: false,
            futures: Box::pin(FuturesUnordered::new()),
            sender,
            interval: Box::pin(time_service.interval(retry_interval)),
        }
    }
}

async fn send_rpc(
    sender: Arc<dyn DAGNetworkSender>,
    peer: Author,
    message: DAGMessage,
    timeout: Duration,
) -> anyhow::Result<DAGMessage> {
    sender.send_rpc(peer, message, timeout).await
}

impl Stream for RpcWithFallback {
    type Item = anyhow::Result<DAGMessage>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if !self.futures.is_empty() {
            // Check if any of the futures is ready
            if let Poll::Ready(result) = self.futures.as_mut().poll_next(cx) {
                return Poll::Ready(result);
            }
        }

        // Check if the timeout has happened
        let timeout = matches!(self.interval.as_mut().poll_next(cx), Poll::Ready(_));

        if self.futures.is_empty() || timeout {
            // try to find more responders and queue futures
            if let Some(peers) = Pin::new(&mut self.responders).next_to_request() {
                for peer in peers {
                    let future = Box::pin(send_rpc(
                        self.sender.clone(),
                        peer,
                        self.message.clone(),
                        self.rpc_timeout,
                    ));
                    self.futures.push(future);
                }
            } else if self.futures.is_empty() {
                self.terminated = true;
                return Poll::Ready(None);
            }
        }

        self.futures.as_mut().poll_next(cx)
    }
}

impl FusedStream for RpcWithFallback {
    fn is_terminated(&self) -> bool {
        self.terminated
    }
}

struct ExponentialNumberGenerator {
    current: u32,
    factor: u32,
    max_limit: u32,
}

impl ExponentialNumberGenerator {
    fn new(starting_value: u32, factor: u32, max_limit: u32) -> Self {
        Self {
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
            self.current = self
                .current
                .checked_mul(self.factor)
                .unwrap_or(self.max_limit)
                .min(self.max_limit)
        }

        Some(result)
    }
}
