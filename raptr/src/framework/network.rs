// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::framework::{timer::NeverReturn, NodeId};
use futures::poll;
use rand::{distributions::Distribution, Rng};
use std::{future::Future, marker::PhantomData, sync::Arc, task::Poll::Ready, time::Duration};
use tokio::sync::mpsc;

pub trait MessageVerifier: Send + Sync + 'static {
    type Message: NetworkMessage;

    /// Verify the message, possibly checking signatures, certificates, etc.
    fn verify(
        &self,
        sender: NodeId,
        message: &Self::Message,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}

pub trait MessageCertifier: Send + Sync + 'static {
    type Message: NetworkMessage;

    /// Certify the message.
    fn certify(
        &self,
        message: &mut Self::Message,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}

pub struct NoopCertifier<M> {
    _phantom: PhantomData<M>,
}

impl<M> NoopCertifier<M> {
    pub fn new() -> Self {
        NoopCertifier {
            _phantom: PhantomData,
        }
    }
}

impl<M: NetworkMessage> MessageCertifier for NoopCertifier<M> {
    type Message = M;

    async fn certify(&self, _message: &mut Self::Message) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct NoopVerifier<M> {
    _phantom: PhantomData<M>,
}

impl<M> NoopVerifier<M> {
    pub fn new() -> Self {
        NoopVerifier {
            _phantom: PhantomData,
        }
    }
}

impl<M: NetworkMessage> MessageVerifier for NoopVerifier<M> {
    type Message = M;

    async fn verify(&self, _sender: NodeId, _message: &Self::Message) -> anyhow::Result<()> {
        Ok(())
    }
}

pub trait NetworkMessage: Send + Sync + 'static {}

impl<T: Send + Sync + 'static> NetworkMessage for T {}

pub trait NetworkSender: Send + Sync + 'static {
    type Message: NetworkMessage;

    fn send(&self, data: Self::Message, targets: Vec<NodeId>) -> impl Future<Output = ()> + Send;

    fn unicast(&self, data: Self::Message, target: NodeId) -> impl Future<Output = ()> + Send {
        self.send(data, vec![target])
    }

    fn multicast(&self, data: Self::Message) -> impl Future<Output = ()> + Send {
        self.send(data, (0..self.n_nodes()).into_iter().collect())
    }

    fn n_nodes(&self) -> usize;
}

pub trait NetworkService: NetworkSender {
    type Sender: NetworkSender<Message = Self::Message>;

    fn new_sender(&self) -> Self::Sender;

    fn recv(&mut self) -> impl Future<Output = (NodeId, Self::Message)> + Send;

    fn drop_one(&mut self) -> impl Future<Output = bool> + Send {
        async {
            let recv = self.recv();
            tokio::pin!(recv);
            matches!(poll!(recv.as_mut()), Ready(_))
        }
    }

    fn clear_inbox(&mut self) -> impl Future<Output = ()> + Send {
        async { while self.drop_one().await {} }
    }
}

pub trait Network {
    type Message;
    type Service: NetworkService<Message = Self::Message>;

    fn service(&mut self, node_id: NodeId) -> Self::Service;
}

pub struct InjectedLocalNetworkSender<M, I, C> {
    send: Vec<mpsc::Sender<(NodeId, M)>>,
    injection: I,
    node_id: NodeId,
    certifier: Arc<C>,
}

// #[derive(Clone)] doesn't work for `C` that is not `Clone`.
impl<M, I: Clone, C> Clone for InjectedLocalNetworkSender<M, I, C> {
    fn clone(&self) -> Self {
        InjectedLocalNetworkSender {
            send: self.send.clone(),
            injection: self.injection.clone(),
            node_id: self.node_id,
            certifier: self.certifier.clone(),
        }
    }
}

impl<M, I, C> NetworkSender for InjectedLocalNetworkSender<M, I, C>
where
    M: NetworkMessage + Clone,
    I: NetworkInjection<M>,
    C: MessageCertifier<Message = M>,
{
    type Message = M;

    /// `send` spawns a separate task for each target that calls `self.injection`
    /// on the message before sending it to `target`. The injection may:
    ///   1. sleep to simulate a message delay
    ///   2. drop the message to simulate message loss;
    ///   3. modify the message to simulate message corruption.
    ///
    /// Since the injection happens in a new task, `send` always returns immediately, not
    /// affected by any injected delay.
    async fn send(&self, mut msg: M, targets: Vec<NodeId>) {
        // TODO: consider spawning a co-routine to certify off the critical path.
        self.certifier.certify(&mut msg).await.unwrap();

        for target in targets {
            let data = msg.clone();
            let sender = self.node_id;
            let channel = self.send[target].clone();
            let injection = self.injection.clone();

            tokio::spawn(async move {
                // if let Some(message) = injection(message, target).await {
                //     send_channel.send(message).await.unwrap();
                // }
                if let Some(data) = injection(sender, target, data).await {
                    // Ignoring send errors.
                    let _ = channel.send((sender, data)).await;
                }
            });
        }
    }

    fn n_nodes(&self) -> usize {
        self.send.len()
    }
}

pub struct InjectedLocalNetworkService<M, I, C> {
    sender: InjectedLocalNetworkSender<M, I, C>,
    recv: mpsc::Receiver<(NodeId, M)>,
}

impl<M, I, C> NetworkSender for InjectedLocalNetworkService<M, I, C>
where
    M: NetworkMessage + Clone,
    I: NetworkInjection<M>,
    C: MessageCertifier<Message = M>,
{
    type Message = M;

    async fn send(&self, msg: Self::Message, targets: Vec<NodeId>) {
        self.sender.send(msg, targets).await;
    }

    fn n_nodes(&self) -> usize {
        self.sender.n_nodes()
    }
}

impl<M, I, C> NetworkService for InjectedLocalNetworkService<M, I, C>
where
    M: NetworkMessage + Clone,
    I: NetworkInjection<M>,
    C: MessageCertifier<Message = M>,
{
    type Sender = InjectedLocalNetworkSender<M, I, C>;

    fn new_sender(&self) -> Self::Sender {
        self.sender.clone()
    }

    async fn recv(&mut self) -> (NodeId, M) {
        // TODO: add verification
        self.recv.recv().await.unwrap()
    }
}

pub struct InjectedLocalNetwork<M, I> {
    send: Vec<mpsc::Sender<(NodeId, M)>>,
    recv: Vec<Option<mpsc::Receiver<(NodeId, M)>>>,
    injection: I,
}

impl<M, I> InjectedLocalNetwork<M, I>
where
    M: NetworkMessage + Clone,
    I: NetworkInjection<M>,
{
    pub fn new(n_nodes: usize, injection: I) -> Self {
        let (send, recv) = (0..n_nodes)
            .map(|_| {
                let (send, recv) = mpsc::channel(1024);
                (send, Some(recv))
            })
            .unzip();
        InjectedLocalNetwork {
            send,
            recv,
            injection,
        }
    }

    pub fn service<C>(
        &mut self,
        node_id: NodeId,
        certifier: Arc<C>,
    ) -> InjectedLocalNetworkService<M, I, C>
    where
        C: MessageCertifier<Message = M>,
    {
        InjectedLocalNetworkService {
            sender: InjectedLocalNetworkSender {
                send: self.send.clone(),
                injection: self.injection.clone(),
                node_id,
                certifier,
            },
            recv: self.recv[node_id].take().unwrap(),
        }
    }
}

pub trait NetworkInjection<M>:
    Fn(NodeId, NodeId, M) -> Self::Future + Send + Sync + Clone + 'static
{
    type Future: Future<Output = Option<M>> + Send;
}

impl<I, F, M> NetworkInjection<M> for I
where
    I: Fn(NodeId, NodeId, M) -> F + Send + Sync + Clone + 'static,
    F: Future<Output = Option<M>> + Send,
{
    type Future = F;
}

pub fn random_delay_injection<M, D>(distr: D) -> impl NetworkInjection<M>
where
    M: Send,
    D: Distribution<f64> + Copy + Send + Sync + 'static,
{
    move |_, _, message| async move {
        let delay = {
            let mut rng = rand::thread_rng();
            rng.sample(distr)
        };
        tokio::time::sleep(Duration::from_secs_f64(delay)).await;
        Some(message)
    }
}

#[derive(Clone)]
pub struct DropAllNetworkService<M> {
    n_nodes: usize,
    _phantom: PhantomData<M>,
}

impl<M> DropAllNetworkService<M> {
    pub fn new(n_nodes: usize) -> Self {
        DropAllNetworkService {
            n_nodes,
            _phantom: PhantomData,
        }
    }
}

impl<M> NetworkSender for DropAllNetworkService<M>
where
    M: NetworkMessage + Clone,
{
    type Message = M;

    async fn send(&self, _: M, _: Vec<NodeId>) {}

    fn n_nodes(&self) -> usize {
        self.n_nodes
    }
}

impl<M> NetworkService for DropAllNetworkService<M>
where
    M: NetworkMessage + Clone,
{
    type Sender = Self;

    fn new_sender(&self) -> Self::Sender {
        self.clone()
    }

    async fn recv(&mut self) -> (NodeId, M) {
        NeverReturn {}.await;
        unreachable!()
    }
}
