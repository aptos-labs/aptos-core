// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Pooled TCP Transport
use super::tcp::{TCPBufferCfg, TcpSocket};
use crate::transport::Transport;
use aptos_proxy::Proxy;
use aptos_types::{
    network_address::{parse_dns_tcp, parse_ip_tcp, parse_tcp, IpFilter, NetworkAddress},
    PeerId,
};
use futures::{
    future::{self, Either, Future},
    io::{AsyncRead, AsyncWrite},
    ready,
    stream::Stream,
};
use std::{
    collections::{hash_map::Entry, HashMap},
    io,
    net::SocketAddr,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{lookup_host, TcpListener, TcpStream},
};
use url::Url;

/// Transport to build TCP connections
#[derive(Debug, Clone)]
pub struct PooledTcpTransport {
    /// TTL to set for opened sockets, or `None` to keep default.
    pub ttl: Option<u32>,
    /// `TCP_NODELAY` to set for opened sockets, or `None` to keep default.
    pub nodelay: Option<bool>,

    pub tcp_buff_cfg: TCPBufferCfg,

    pub num_connections: usize,
}

impl Default for PooledTcpTransport {
    fn default() -> Self {
        Self {
            ttl: Default::default(),
            nodelay: Default::default(),
            tcp_buff_cfg: Default::default(),
            num_connections: 2,
        }
    }
}

impl PooledTcpTransport {
    fn apply_config(&self, stream: &TcpStream) -> ::std::io::Result<()> {
        if let Some(ttl) = self.ttl {
            stream.set_ttl(ttl)?;
        }

        if let Some(nodelay) = self.nodelay {
            stream.set_nodelay(nodelay)?;
        }

        Ok(())
    }

    pub fn set_tcp_buffers(&mut self, configs: &TCPBufferCfg) {
        self.tcp_buff_cfg = *configs;
    }
}

impl Transport for PooledTcpTransport {
    type Error = ::std::io::Error;
    type Inbound = future::Ready<io::Result<PooledTcpSocket>>;
    type Listener = PooledTcpListenerStream;
    type Outbound = PooledTcpOutbound;
    type Output = PooledTcpSocket;

    fn listen_on(
        &self,
        addr: NetworkAddress,
    ) -> Result<(Self::Listener, NetworkAddress), Self::Error> {
        let ((ipaddr, port), addr_suffix) =
            parse_ip_tcp(addr.as_slice()).ok_or_else(|| invalid_addr_error(&addr))?;
        if !addr_suffix.is_empty() {
            return Err(invalid_addr_error(&addr));
        }

        let addr = SocketAddr::new(ipaddr, port);

        let socket = if ipaddr.is_ipv4() {
            tokio::net::TcpSocket::new_v4()?
        } else {
            tokio::net::TcpSocket::new_v6()?
        };

        if let Some(rx_buf) = self.tcp_buff_cfg.inbound_rx_buffer_bytes {
            socket.set_recv_buffer_size(rx_buf)?;
        }
        if let Some(tx_buf) = self.tcp_buff_cfg.inbound_tx_buffer_bytes {
            socket.set_send_buffer_size(tx_buf)?;
        }
        socket.set_reuseaddr(true)?;
        socket.bind(addr)?;

        let listener = socket.listen(256)?;
        let listen_addr = NetworkAddress::from(listener.local_addr()?);

        Ok((
            PooledTcpListenerStream {
                inner: listener,
                config: self.clone(),
                socket_pool: HashMap::new(),
            },
            listen_addr,
        ))
    }

    fn dial(&self, _peer_id: PeerId, addr: NetworkAddress) -> Result<Self::Outbound, Self::Error> {
        let protos = addr.as_slice();

        // ensure addr is well formed to save some work before potentially
        // spawning a dial task that will fail anyway.
        parse_ip_tcp(protos)
            .map(|_| ())
            .or_else(|| parse_dns_tcp(protos).map(|_| ()))
            .ok_or_else(|| invalid_addr_error(&addr))?;

        let proxy = Proxy::new();

        let proxy_addr = {
            use aptos_types::network_address::Protocol::*;

            let addr = match protos.first() {
                Some(Ip4(ip)) => proxy.https(&ip.to_string()),
                Some(Ip6(ip)) => proxy.https(&ip.to_string()),
                Some(Dns(name)) | Some(Dns4(name)) | Some(Dns6(name)) => proxy.https(name.as_ref()),
                _ => None,
            };

            addr.and_then(|https_proxy| Url::parse(https_proxy).ok())
                .and_then(|url| {
                    if url.has_host() && url.scheme() == "http" {
                        Some(format!(
                            "{}:{}",
                            url.host().unwrap(),
                            url.port_or_known_default().unwrap()
                        ))
                    } else {
                        None
                    }
                })
        };

        let num_conn = self.num_connections;
        let cfg = self.tcp_buff_cfg;
        let f: Pin<Box<dyn Future<Output = Vec<io::Result<TcpStream>>> + Send + 'static>> =
            Box::pin(async move {
                let mut streams = Vec::new();
                for _ in 0..num_conn {
                    let result = match proxy_addr.clone() {
                        Some(proxy_addr) => {
                            Either::Left(connect_via_proxy(proxy_addr, addr.clone()))
                        },
                        None => Either::Right(resolve_and_connect(addr.clone(), cfg.clone())),
                    }
                    .await;
                    println!("POOLED_TCP: connect result {:?}", result);
                    streams.push(result);
                }
                streams
            });

        Ok(PooledTcpOutbound {
            inner: f,
            config: self.clone(),
        })
    }
}

/// Try to lookup the dns name, then filter addrs according to the `IpFilter`.
async fn resolve_with_filter(
    ip_filter: IpFilter,
    dns_name: &str,
    port: u16,
) -> io::Result<impl Iterator<Item = SocketAddr> + '_> {
    Ok(lookup_host((dns_name, port))
        .await?
        .filter(move |socketaddr| ip_filter.matches(socketaddr.ip())))
}

pub async fn connect_with_config(
    port: u16,
    ipaddr: std::net::IpAddr,
    tcp_buff_cfg: TCPBufferCfg,
) -> io::Result<TcpStream> {
    let addr = SocketAddr::new(ipaddr, port);

    let socket = if addr.is_ipv4() {
        tokio::net::TcpSocket::new_v4()?
    } else {
        tokio::net::TcpSocket::new_v6()?
    };

    if let Some(rx_buf) = tcp_buff_cfg.outbound_rx_buffer_bytes {
        socket.set_recv_buffer_size(rx_buf)?;
    }
    if let Some(tx_buf) = tcp_buff_cfg.outbound_tx_buffer_bytes {
        socket.set_send_buffer_size(tx_buf)?;
    }
    socket.connect(addr).await
}

/// Note: we need to take ownership of this `NetworkAddress` (instead of just
/// borrowing the `&[Protocol]` slice) so this future can be `Send + 'static`.
pub async fn resolve_and_connect(
    addr: NetworkAddress,
    tcp_buff_cfg: TCPBufferCfg,
) -> io::Result<TcpStream> {
    let protos = addr.as_slice();

    if let Some(((ipaddr, port), _addr_suffix)) = parse_ip_tcp(protos) {
        // this is an /ip4 or /ip6 address, so we can just connect without any
        // extra resolving or filtering.
        connect_with_config(port, ipaddr, tcp_buff_cfg).await
    } else if let Some(((ip_filter, dns_name, port), _addr_suffix)) = parse_dns_tcp(protos) {
        // resolve dns name and filter
        let socketaddr_iter = resolve_with_filter(ip_filter, dns_name.as_ref(), port).await?;
        let mut last_err = None;

        // try to connect until the first succeeds
        for socketaddr in socketaddr_iter {
            match connect_with_config(socketaddr.port(), socketaddr.ip(), tcp_buff_cfg).await {
                Ok(stream) => return Ok(stream),
                Err(err) => last_err = Some(err),
            }
        }

        Err(last_err.unwrap_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "could not resolve dns name to any address: name: {}, ip filter: {:?}",
                    dns_name.as_ref(),
                    ip_filter,
                ),
            )
        }))
    } else {
        Err(invalid_addr_error(&addr))
    }
}

async fn connect_via_proxy(proxy_addr: String, addr: NetworkAddress) -> io::Result<TcpStream> {
    let protos = addr.as_slice();

    if let Some(((host, port), _addr_suffix)) = parse_tcp(protos) {
        let mut stream = TcpStream::connect(proxy_addr).await?;
        let mut buffer = [0; 4096];
        let mut read = 0;

        stream
            .write_all(&format!("CONNECT {0}:{1} HTTP/1.0\r\n\r\n", host, port).into_bytes())
            .await?;

        loop {
            let len = stream.read(&mut buffer[read..]).await?;
            read += len;
            let msg = &buffer[..read];

            if len == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        "HTTP proxy CONNECT failed. Len == 0. Message: {}",
                        String::from_utf8_lossy(msg)
                    ),
                ));
            } else if msg.len() >= 16 {
                if (msg.starts_with(b"HTTP/1.1 200") || msg.starts_with(b"HTTP/1.0 200"))
                    && msg.ends_with(b"\r\n\r\n")
                {
                    return Ok(stream);
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!(
                            "HTTP proxy CONNECT failed! Unexpected message: {}",
                            String::from_utf8_lossy(msg)
                        ),
                    ));
                }
            } else {
                // Keep reading until we get at least 16 bytes
            }
        }
    } else {
        Err(invalid_addr_error(&addr))
    }
}

fn invalid_addr_error(addr: &NetworkAddress) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("Invalid NetworkAddress: '{}'", addr),
    )
}

#[must_use = "streams do nothing unless polled"]
pub struct PooledTcpListenerStream {
    inner: TcpListener,
    config: PooledTcpTransport,
    socket_pool: HashMap<NetworkAddress, PooledTcpSocket>,
}

impl Stream for PooledTcpListenerStream {
    type Item = io::Result<(future::Ready<io::Result<PooledTcpSocket>>, NetworkAddress)>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Self::Item>> {
        match self.inner.poll_accept(context) {
            Poll::Ready(Ok((socket, addr))) => {
                if let Err(e) = self.config.apply_config(&socket) {
                    return Poll::Ready(Some(Err(e)));
                }
                let dialer_addr = NetworkAddress::from(addr);
                match self.socket_pool.entry(dialer_addr.clone()) {
                    Entry::Occupied(mut entry) => {
                        if entry.get().inner.lock().unwrap().closed {
                            let socket = PooledTcpSocket::new(socket);
                            entry.insert(socket.clone());
                            return Poll::Ready(Some(Ok((future::ready(Ok(socket)), dialer_addr))));
                        } else {
                            entry.get().push(socket);
                        }
                        Poll::Pending
                    },
                    Entry::Vacant(entry) => {
                        let socket = PooledTcpSocket::new(socket);
                        entry.insert(socket.clone());
                        Poll::Ready(Some(Ok((future::ready(Ok(socket)), dialer_addr))))
                    },
                }
            },
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[must_use = "futures do nothing unless polled"]
pub struct PooledTcpOutbound {
    inner: Pin<Box<dyn Future<Output = Vec<io::Result<TcpStream>>> + Send + 'static>>,
    config: PooledTcpTransport,
}

impl Future for PooledTcpOutbound {
    type Output = io::Result<PooledTcpSocket>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let conn_results = ready!(Pin::new(&mut self.inner).poll(context));
        let mut sockets = Vec::new();
        let all_errors = conn_results.iter().all(|r| r.is_err());
        for conn in conn_results.into_iter() {
            if all_errors {
                println!("POOLED_TCP: poll outbound result {:?}", conn);
                let err = conn.unwrap_err();
                return Poll::Ready(Err(err));
            }
            if let Ok(socket) = conn {
                self.config.apply_config(&socket)?;
                sockets.push(socket);
            }
        }
        Poll::Ready(Ok(PooledTcpSocket::new_pool(sockets)))
    }
}

/// A wrapper around a tokio TcpStream
///
/// In order to properly implement the AsyncRead/AsyncWrite traits we need to wrap a TcpStream to
/// ensure that the "close" method actually closes the write half of the TcpStream.  This is
/// because the "close" method on a TcpStream just performs a no-op instead of actually shutting
/// down the write side of the TcpStream.
//TODO Probably should add some tests for this
#[derive(Debug, Clone)]
pub struct PooledTcpSocket {
    inner: Arc<Mutex<PooledTcpSocketInner>>,
}

impl PooledTcpSocket {
    pub fn new(socket: TcpStream) -> Self {
        Self {
            inner: Arc::new(Mutex::new(PooledTcpSocketInner::new(vec![TcpSocket::new(
                socket,
            )]))),
        }
    }

    pub fn new_pool(sockets: Vec<TcpStream>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(PooledTcpSocketInner::new(
                sockets.into_iter().map(TcpSocket::new).collect(),
            ))),
        }
    }

    pub fn push(&self, socket: TcpStream) {
        self.inner
            .lock()
            .unwrap()
            .sockets
            .push(TcpSocket::new(socket));
    }
}

#[derive(Debug)]
pub struct PooledTcpSocketInner {
    sockets: Vec<TcpSocket>,
    read_idx: usize,
    write_idx: usize,
    closed: bool,
}

impl PooledTcpSocketInner {
    fn new(sockets: Vec<TcpSocket>) -> Self {
        Self {
            sockets,
            read_idx: 0,
            write_idx: 0,
            closed: false,
        }
    }
}

impl AsyncRead for PooledTcpSocket {
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let mut inner = self.inner.lock().unwrap();
        let read_idx = inner.read_idx;
        inner.read_idx += 1;
        if inner.read_idx == inner.sockets.len() {
            inner.read_idx = 0;
        }
        Pin::new(&mut inner.sockets[read_idx]).poll_read(context, buf)
    }
}

impl AsyncWrite for PooledTcpSocket {
    fn poll_write(
        self: Pin<&mut Self>,
        context: &mut Context,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut inner = self.inner.lock().unwrap();
        let write_idx = inner.write_idx;
        inner.write_idx += 1;
        if inner.write_idx == inner.sockets.len() {
            inner.write_idx = 0;
        }
        Pin::new(&mut inner.sockets[write_idx]).poll_write(context, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, context: &mut Context) -> Poll<io::Result<()>> {
        let mut inner = self.inner.lock().unwrap();
        for socket in &mut inner.sockets {
            ready!(Pin::new(socket).poll_flush(context)).ok();
        }
        Poll::Ready(io::Result::Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, context: &mut Context) -> Poll<io::Result<()>> {
        let mut inner = self.inner.lock().unwrap();
        for socket in &mut inner.sockets {
            ready!(Pin::new(socket).poll_close(context)).ok();
        }
        inner.closed = true;
        Poll::Ready(io::Result::Ok(()))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::transport::{ConnectionOrigin, Transport, TransportExt};
    use aptos_types::PeerId;
    use futures::{
        future::{join, FutureExt},
        io::{AsyncReadExt, AsyncWriteExt},
        stream::StreamExt,
    };
    use tokio::runtime::Runtime;

    #[tokio::test]
    async fn simple_listen_and_dial() -> Result<(), ::std::io::Error> {
        let t = PooledTcpTransport::default().and_then(|mut out, _addr, origin| async move {
            match origin {
                ConnectionOrigin::Inbound => {
                    out.write_all(b"Earth").await?;
                    let mut buf = [0; 3];
                    out.read_exact(&mut buf).await?;
                    assert_eq!(&buf, b"Air");
                },
                ConnectionOrigin::Outbound => {
                    let mut buf = [0; 5];
                    out.read_exact(&mut buf).await?;
                    assert_eq!(&buf, b"Earth");
                    out.write_all(b"Air").await?;
                },
            }
            Ok(())
        });

        let (listener, addr) = t.listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())?;
        let peer_id = PeerId::random();
        let dial = t.dial(peer_id, addr)?;
        let listener = listener.into_future().then(|(maybe_result, _stream)| {
            let (incoming, _addr) = maybe_result.unwrap().unwrap();
            incoming.map(Result::unwrap)
        });

        let (outgoing, _incoming) = join(dial, listener).await;
        assert!(outgoing.is_ok());
        Ok(())
    }

    #[test]
    fn unsupported_multiaddrs() {
        let t = PooledTcpTransport::default();

        let result = t.listen_on("/memory/0".parse().unwrap());
        assert!(result.is_err());

        let peer_id = PeerId::random();
        let result = t.dial(peer_id, "/memory/22".parse().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_with_filter() {
        let rt = Runtime::new().unwrap();

        // note: we only lookup "localhost", which is not really a DNS name, but
        // should always resolve to something and keep this test from being flaky.

        let f = async move {
            // this should always return something
            let addrs = resolve_with_filter(IpFilter::Any, "localhost", 1234)
                .await
                .unwrap()
                .collect::<Vec<_>>();
            assert!(!addrs.is_empty(), "addrs: {:?}", addrs);

            // we should only get Ip4 addrs
            let addrs = resolve_with_filter(IpFilter::OnlyIp4, "localhost", 1234)
                .await
                .unwrap()
                .collect::<Vec<_>>();
            assert!(addrs.iter().all(SocketAddr::is_ipv4), "addrs: {:?}", addrs);

            // we should only get Ip6 addrs
            let addrs = resolve_with_filter(IpFilter::OnlyIp6, "localhost", 1234)
                .await
                .unwrap()
                .collect::<Vec<_>>();
            assert!(addrs.iter().all(SocketAddr::is_ipv6), "addrs: {:?}", addrs);
        };

        rt.block_on(f);
    }
}
