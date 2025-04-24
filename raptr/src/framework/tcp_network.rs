// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::framework::{
    injection::{delay_injection, drop_injection},
    network::{MessageCertifier, MessageVerifier, NetworkMessage, NetworkSender, NetworkService},
    NodeId,
};
use anyhow::Context;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::{
    marker::PhantomData,
    mem::size_of,
    net::SocketAddr,
    sync::{atomic::AtomicUsize, Arc},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{Mutex, OwnedMutexGuard},
};

pub type MessageSizeTag = u32;

pub const RETRY_MILLIS: u64 = 1000;

pub struct Config {
    pub peers: Vec<SocketAddr>,
    pub streams_per_peer: usize,
}

struct TcpNetworkSenderInner<M, C> {
    node_id: NodeId,
    self_send: aptos_channel::Sender<NodeId, (NodeId, M)>,
    streams: Vec<PeerStreams>,
    certifier: Arc<C>,
    max_message_size: usize,
}

impl<M, C> TcpNetworkSenderInner<M, C> {
    fn self_send(&self, msg: M) {
        self.self_send
            .push(self.node_id, (self.node_id, msg))
            .unwrap();
    }
}

struct PeerStreams {
    streams: Vec<Arc<Mutex<TcpStream>>>,
    next: AtomicUsize,
}

impl PeerStreams {
    async fn next(&self) -> OwnedMutexGuard<TcpStream> {
        let next = self.next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.streams[next % self.streams.len()]
            .clone()
            .lock_owned()
            .await
    }
}

pub struct TcpNetworkSender<M, C> {
    inner: Arc<TcpNetworkSenderInner<M, C>>,
}

// #[derive(Clone)] doesn't work for `M` and `C` that are not `Clone`.
impl<M, C> Clone for TcpNetworkSender<M, C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

async fn send_msg_to_stream(
    data: Arc<Vec<u8>>,
    mut stream: OwnedMutexGuard<TcpStream>,
) -> anyhow::Result<()> {
    stream
        .write_all(&(data.len() as MessageSizeTag).to_be_bytes())
        .await?;
    stream.write_all(&data).await?;
    stream.flush().await?;
    Ok(())
}

impl<M, C> NetworkSender for TcpNetworkSender<M, C>
where
    M: NetworkMessage + Serialize + for<'de> Deserialize<'de> + std::fmt::Debug,
    C: MessageCertifier<Message = M>,
{
    type Message = M;

    async fn send(&self, mut msg: Self::Message, targets: Vec<NodeId>) {
        let inner = self.inner.clone();

        tokio::spawn(async move {
            if let Err(err) = inner.certifier.certify(&mut msg).await {
                panic!("TCPNET: Failed to sign message: {:#}", err);
            }

            // Avoid serializing the message if we are sending the message only to ourselves.
            if targets.len() == 1 && targets[0] == inner.node_id {
                inner.self_send(msg);
                return;
            }

            let data = Arc::new(bcs::to_bytes(&msg).unwrap());

            if data.len() > inner.max_message_size {
                // Panicking because this is most likely caused by a bug in the code and needs
                // to be discovered ASAP.
                panic!("Trying to send a message that is too large: {}", data.len());
            }

            // Bypass the network for self-sends.
            if targets.contains(&inner.node_id) {
                inner.self_send(msg);
            }

            for peer_id in targets {
                if peer_id == inner.node_id {
                    continue;
                }

                let data = data.clone();

                let inner = inner.clone();
                tokio::spawn(async move {
                    let stream = inner.streams[peer_id].next().await;

                    if let Err(err) = send_msg_to_stream(data, stream).await {
                        aptos_logger::error!(
                            "TCPNET: Failed to send message to peer {}: {:#}",
                            peer_id,
                            err,
                        );
                    }
                });
            }
        });
    }

    fn n_nodes(&self) -> usize {
        self.inner.streams.len()
    }
}

pub struct TcpNetworkService<M, C, V> {
    recv: aptos_channel::Receiver<NodeId, (NodeId, M)>,
    sender: TcpNetworkSender<M, C>,
    _phantom: PhantomData<V>,
}

impl<M, C, V> TcpNetworkService<M, C, V>
where
    M: NetworkMessage + Serialize + for<'de> Deserialize<'de> + std::fmt::Debug,
    C: MessageCertifier<Message = M>,
    V: MessageVerifier<Message = M> + Send + 'static,
{
    pub async fn new(
        node_id: NodeId,
        addr: SocketAddr,
        config: Config,
        certifier: Arc<C>,
        verifier: Arc<V>,
        max_message_size: usize,
    ) -> Self {
        aptos_logger::info!(
            "TCPNET: Starting TCP network service for node {} at {}",
            node_id,
            addr
        );

        let (self_send, recv) = aptos_channel::new(QueueStyle::LIFO, 16, None);

        // Start the receiver task
        let listener = Self::create_listener(addr).await;
        tokio::spawn(Self::listen_loop(
            listener,
            self_send.clone(),
            verifier,
            max_message_size,
        ));

        let mut streams = Vec::new();

        // NB: can (should?) be parallelized
        for (peer_id, peer_addr) in config.peers.iter().enumerate() {
            let mut peer_streams = Vec::new();

            if peer_id != node_id {
                for _ in 0..config.streams_per_peer {
                    let mut stream = Self::create_stream(peer_addr).await;
                    stream.write_all(&node_id.to_be_bytes()).await.unwrap();
                    peer_streams.push(Arc::new(Mutex::new(stream)));
                }
            }

            streams.push(PeerStreams {
                streams: peer_streams,
                next: AtomicUsize::new(0),
            });
        }

        TcpNetworkService {
            recv,
            sender: TcpNetworkSender {
                inner: Arc::new(TcpNetworkSenderInner {
                    node_id,
                    self_send,
                    streams,
                    certifier,
                    max_message_size,
                }),
            },
            _phantom: PhantomData,
        }
    }

    async fn create_listener(addr: SocketAddr) -> TcpListener {
        loop {
            match TcpListener::bind(addr).await {
                Ok(listener) => {
                    return listener;
                },
                Err(err) => {
                    aptos_logger::error!(
                        "TCPNET: Failed to bind listener to {}: {}. Retry in {} millis",
                        addr,
                        err,
                        RETRY_MILLIS,
                    );
                    tokio::time::sleep(Duration::from_millis(RETRY_MILLIS)).await;
                },
            }
        }
    }

    async fn create_stream(peer_addr: &SocketAddr) -> TcpStream {
        loop {
            match TcpStream::connect(peer_addr).await {
                Ok(stream) => {
                    aptos_logger::info!("TCPNET: Connected to peer {}", peer_addr);
                    return stream;
                },
                Err(err) => {
                    aptos_logger::error!(
                        "TCPNET: Failed to connect to peer {}: {}. Retry in {} millis",
                        peer_addr,
                        err,
                        RETRY_MILLIS,
                    );
                    tokio::time::sleep(Duration::from_millis(RETRY_MILLIS)).await;
                },
            }
        }
    }

    async fn listen_loop(
        tcp_listener: TcpListener,
        self_send: aptos_channel::Sender<NodeId, (NodeId, M)>,
        verifier: Arc<V>,
        max_message_size: usize,
    ) {
        loop {
            let (stream, _) = tcp_listener.accept().await.unwrap();
            tokio::spawn(Self::listen_stream(
                stream,
                self_send.clone(),
                verifier.clone(),
                max_message_size,
            ));
        }
    }

    async fn listen_stream(
        mut stream: TcpStream,
        self_send: aptos_channel::Sender<NodeId, (NodeId, M)>,
        verifier: Arc<V>,
        max_message_size: usize,
    ) {
        let mut buf = vec![0; max_message_size];

        stream
            .read_exact(&mut buf[..size_of::<NodeId>()])
            .await
            .unwrap();

        // FIXME: this is not Byzantine fault tolerant.
        // TODO: add authentication.
        let peer_id = NodeId::from_be_bytes(buf[..size_of::<NodeId>()].try_into().unwrap());

        while !self_send.receiver_dropped() {
            match Self::read_and_validate_message(
                &mut stream,
                peer_id,
                verifier.clone(),
                &mut buf,
                max_message_size,
            )
            .await
            {
                Ok(msg) => {
                    if drop_injection() {
                        aptos_logger::info!("TCPNET: Dropping message from peer {}", peer_id);
                        continue;
                    }

                    if cfg!(feature = "inject-delays") {
                        let self_send = self_send.clone();
                        tokio::spawn(async move {
                            delay_injection().await;
                            if self_send.push(peer_id, (peer_id, msg)).is_err() {
                                // The receiver has been dropped.
                                return;
                            }
                        });
                    } else {
                        if self_send.push(peer_id, (peer_id, msg)).is_err() {
                            // The receiver has been dropped. Closing the stream.
                            break;
                        }
                    }
                },
                Err(err) => {
                    aptos_logger::error!(
                        "TCPNET: Failed to read message from peer {}, closing the stream: {:#}",
                        peer_id,
                        err
                    );
                    break;
                },
            }
        }
    }

    async fn read_and_validate_message(
        stream: &mut TcpStream,
        peer_id: NodeId,
        verifier: Arc<V>,
        buf: &mut [u8],
        max_message_size: usize,
    ) -> anyhow::Result<M> {
        // Read and check the message size tag.
        stream
            .read_exact(&mut buf[..size_of::<MessageSizeTag>()])
            .await
            .context("Error reading the message size tag")?;
        let msg_size =
            MessageSizeTag::from_be_bytes(buf[..size_of::<MessageSizeTag>()].try_into().unwrap())
                as usize;
        if msg_size > max_message_size {
            return Err(anyhow::anyhow!("Message size too large: {}", msg_size));
        }

        // Read the message, deserialize it, and validate it.
        stream
            .read_exact(&mut buf[..msg_size])
            .await
            .context("Error reading the message")?;
        let msg: M =
            bcs::from_bytes(&buf[..msg_size]).context("Error deserializing the message")?;

        verifier
            .verify(peer_id, &msg)
            .await
            .context("Error verifying the message")?;

        Ok(msg)
    }
}

impl<M, C, V> NetworkSender for TcpNetworkService<M, C, V>
where
    M: NetworkMessage + Serialize + for<'de> Deserialize<'de> + std::fmt::Debug,
    C: MessageCertifier<Message = M>,
    V: MessageVerifier<Message = M>,
{
    type Message = M;

    async fn send(&self, msg: Self::Message, targets: Vec<NodeId>) {
        self.sender.send(msg, targets).await;
    }

    fn n_nodes(&self) -> usize {
        self.sender.n_nodes()
    }
}

impl<M, C, V> NetworkService for TcpNetworkService<M, C, V>
where
    M: NetworkMessage + Serialize + for<'de> Deserialize<'de> + std::fmt::Debug,
    C: MessageCertifier<Message = M>,
    V: MessageVerifier<Message = M>,
{
    type Sender = TcpNetworkSender<M, C>;

    fn new_sender(&self) -> Self::Sender {
        self.sender.clone()
    }

    async fn recv(&mut self) -> (NodeId, M) {
        self.recv.select_next_some().await
    }
}
