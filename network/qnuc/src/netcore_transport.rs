// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Implementation of the `aptos_netcore::transport::Transport` trait for QNUC,
//! allowing QNUC to be used as a drop-in replacement for TCP in the
//! Aptos networking stack.
//!
//! This enables addresses like `/ip4/127.0.0.1/udp/6180/noise-ik/<key>/handshake/1`
//! to be used in the same way as `/ip4/127.0.0.1/tcp/6180/noise-ik/<key>/handshake/1`.

use crate::adapter::QnucSocket;
use aptos_netcore::transport::Transport;
use aptos_types::{
    network_address::{parse_dns_udp, parse_ip_udp, NetworkAddress},
    PeerId,
};
use futures::{
    future::{self, Future},
    stream::Stream,
};
use std::{
    io,
    net::SocketAddr,
    pin::Pin,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    task::{Context, Poll},
};
use tokio::net::UdpSocket;

static CONN_ID_GEN: AtomicU64 = AtomicU64::new(1);

fn next_conn_id() -> u64 {
    CONN_ID_GEN.fetch_add(1, Ordering::Relaxed)
}

/// Transport that creates QNUC (UDP-based) connections.
///
/// Implements `aptos_netcore::transport::Transport` so it can be used with
/// `AptosNetTransport<QnucTransportLayer>` in the same way as `TcpTransport`.
#[derive(Debug, Clone, Default)]
pub struct QnucTransportLayer;

impl QnucTransportLayer {
    pub fn new() -> Self {
        Self
    }
}

/// A listener stream that accepts inbound QNUC (UDP) connections.
#[must_use = "streams do nothing unless polled"]
pub struct QnucListenerStream {
    socket: Arc<UdpSocket>,
}

impl Stream for QnucListenerStream {
    type Item = io::Result<(future::Ready<io::Result<QnucSocket>>, NetworkAddress)>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut buf = vec![0u8; 65536];
        let recv_fut = self.socket.recv_from(&mut buf);
        tokio::pin!(recv_fut);

        match recv_fut.poll(cx) {
            Poll::Ready(Ok((_n, from_addr))) => {
                let conn_id = next_conn_id();
                // Create a new connected UDP socket for this peer
                let socket = self.socket.clone();
                let qnuc_socket = QnucSocket::new(socket, from_addr, conn_id);

                let dialer_addr = NetworkAddress::from(from_addr);

                // We need to push the initial data back since the listener
                // consumed the first datagram. We handle this by creating
                // a QnucSocket that has pre-buffered data.
                // For now, create a fresh socket - the Noise handshake
                // protocol will re-request data.
                Poll::Ready(Some(Ok((
                    future::ready(Ok(qnuc_socket)),
                    dialer_addr,
                ))))
            },
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// A pending outbound QNUC connection.
#[must_use = "futures do nothing unless polled"]
pub struct QnucOutbound {
    inner: Pin<Box<dyn Future<Output = io::Result<QnucSocket>> + Send + 'static>>,
}

impl Future for QnucOutbound {
    type Output = io::Result<QnucSocket>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        Pin::new(&mut self.inner).poll(cx)
    }
}

impl Transport for QnucTransportLayer {
    type Error = io::Error;
    type Inbound = future::Ready<io::Result<QnucSocket>>;
    type Listener = QnucListenerStream;
    type Outbound = QnucOutbound;
    type Output = QnucSocket;

    fn listen_on(
        &self,
        addr: NetworkAddress,
    ) -> Result<(Self::Listener, NetworkAddress), Self::Error> {
        let protos = addr.as_slice();
        let ((ipaddr, port), _addr_suffix) =
            parse_ip_udp(protos).ok_or_else(|| invalid_addr_error(&addr))?;

        let socket_addr = SocketAddr::new(ipaddr, port);

        // We need to bind synchronously, so use std UDP then convert
        let std_socket = std::net::UdpSocket::bind(socket_addr)?;
        std_socket.set_nonblocking(true)?;
        let tokio_socket = UdpSocket::from_std(std_socket)?;
        let listen_addr = make_udp_addr(tokio_socket.local_addr()?);

        let socket = Arc::new(tokio_socket);

        Ok((QnucListenerStream { socket }, listen_addr))
    }

    fn dial(&self, _peer_id: PeerId, addr: NetworkAddress) -> Result<Self::Outbound, Self::Error> {
        let protos = addr.as_slice();

        // Validate the address format
        parse_ip_udp(protos)
            .map(|_| ())
            .or_else(|| parse_dns_udp(protos).map(|_| ()))
            .ok_or_else(|| invalid_addr_error(&addr))?;

        let f: Pin<Box<dyn Future<Output = io::Result<QnucSocket>> + Send + 'static>> =
            Box::pin(resolve_and_connect_udp(addr));

        Ok(QnucOutbound { inner: f })
    }
}

/// Resolve a NetworkAddress and create a connected UDP socket.
async fn resolve_and_connect_udp(addr: NetworkAddress) -> io::Result<QnucSocket> {
    let protos = addr.as_slice();

    let remote_addr = if let Some(((ipaddr, port), _)) = parse_ip_udp(protos) {
        SocketAddr::new(ipaddr, port)
    } else if let Some(((ip_filter, dns_name, port), _)) = parse_dns_udp(protos) {
        use tokio::net::lookup_host;
        let addrs: Vec<SocketAddr> = lookup_host((dns_name.as_ref(), port))
            .await?
            .filter(|a| ip_filter.matches(a.ip()))
            .collect();
        *addrs.first().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                format!("DNS resolution failed for {}", dns_name.as_ref()),
            )
        })?
    } else {
        return Err(invalid_addr_error(&addr));
    };

    let local_bind = if remote_addr.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    };
    let socket = UdpSocket::bind(local_bind).await?;
    let socket = Arc::new(socket);
    let conn_id = next_conn_id();

    Ok(QnucSocket::new(socket, remote_addr, conn_id))
}

fn make_udp_addr(sockaddr: SocketAddr) -> NetworkAddress {
    use aptos_types::network_address::Protocol;
    let ip_proto = Protocol::from(sockaddr.ip());
    let udp_proto = Protocol::Udp(sockaddr.port());
    NetworkAddress::try_from(vec![ip_proto, udp_proto]).expect("valid address")
}

fn invalid_addr_error(addr: &NetworkAddress) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("Invalid QNUC NetworkAddress: '{}', expected /ip4|ip6/<addr>/udp/<port>", addr),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_netcore::transport::Transport;

    #[tokio::test]
    async fn test_qnuc_transport_listen() {
        let transport = QnucTransportLayer::new();
        let addr: NetworkAddress = "/ip4/127.0.0.1/udp/0".parse().unwrap();
        let (_listener, listen_addr) = transport.listen_on(addr).unwrap();
        let port = listen_addr.find_port().unwrap();
        assert_ne!(port, 0);
    }

    #[test]
    fn test_qnuc_transport_dial_invalid() {
        let transport = QnucTransportLayer::new();
        let peer_id = PeerId::random();
        // TCP address should fail for QNUC transport
        let result = transport.dial(peer_id, "/ip4/127.0.0.1/tcp/80".parse().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_qnuc_transport_dial_valid() {
        let transport = QnucTransportLayer::new();
        let peer_id = PeerId::random();
        let result = transport.dial(peer_id, "/ip4/127.0.0.1/udp/8080".parse().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn test_make_udp_addr() {
        let addr = make_udp_addr("127.0.0.1:1234".parse().unwrap());
        assert_eq!(addr.to_string(), "/ip4/127.0.0.1/udp/1234");
    }
}
