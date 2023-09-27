// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! QUIC Transport
use crate::transport::Transport;
use aptos_proxy::Proxy;
use aptos_types::{
    network_address::{parse_dns_udp, parse_ip_udp, IpFilter, NetworkAddress},
    PeerId,
};
use futures::{
    future::{self, Either, Future},
    io::{AsyncRead, AsyncWrite},
    ready,
    stream::Stream,
};
use quinn::{ClientConfig, ServerConfig};
use std::{
    fmt::Debug,
    io,
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::net::lookup_host;
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use url::Url;

// Useful constants
const SERVER_STRING: &str = "aptos-node";

/// Transport to build QUIC connections
#[derive(Debug, Clone, Default)]
pub struct QuicTransport {
    // TODO: configs?
}

impl QuicTransport {}

impl Transport for QuicTransport {
    type Error = ::std::io::Error;
    type Inbound = future::Ready<io::Result<Self::Output>>;
    type Listener = QuicListenerStream;
    type Outbound = QuicOutbound;
    type Output = QuicSocket;

    fn listen_on(
        &self,
        addr: NetworkAddress,
    ) -> Result<(Self::Listener, NetworkAddress), Self::Error> {
        // Parse the IP address, port and suffix
        let ((ipaddr, port), addr_suffix) =
            parse_ip_udp(addr.as_slice()).ok_or_else(|| invalid_addr_error(&addr))?;
        if !addr_suffix.is_empty() {
            return Err(invalid_addr_error(&addr));
        }

        // Create the QUIC server endpoint. This will call bind on the socket addr.
        let (server_config, _server_certificate) = configure_server()?;
        let socket_addr = SocketAddr::new(ipaddr, port);
        let server_endpoint = quinn::Endpoint::server(server_config, socket_addr)?;

        // Get the listen address
        let listen_addr = NetworkAddress::from(server_endpoint.local_addr()?);

        // Create the QUIC listener stream
        let quic_listener_stream = QuicListenerStream { server_endpoint };

        Ok((quic_listener_stream, listen_addr))
    }

    fn dial(&self, _peer_id: PeerId, addr: NetworkAddress) -> Result<Self::Outbound, Self::Error> {
        let protos = addr.as_slice();

        // ensure addr is well formed to save some work before potentially
        // spawning a dial task that will fail anyway.
        parse_ip_udp(protos)
            .map(|_| ())
            .or_else(|| parse_dns_udp(protos).map(|_| ()))
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

        let f: Pin<Box<dyn Future<Output = io::Result<QuicSocket>> + Send + 'static>> =
            Box::pin(match proxy_addr {
                Some(proxy_addr) => Either::Left(connect_via_proxy(proxy_addr, addr)),
                None => Either::Right(resolve_and_connect(addr)),
            });

        Ok(QuicOutbound { inner: f })
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

pub async fn connect_to_remote(
    port: u16,
    remote_ipaddr: std::net::IpAddr,
) -> io::Result<quinn::Connection> {
    // Create the QUIC client endpoint. This will call bind on 127.0.0.1.
    let mut client_endpoint =
        quinn::Endpoint::client("127.0.0.1:0".parse().unwrap()).map_err(|error| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("could not create client endpoint: {:?}", error),
            )
        })?;
    client_endpoint.set_default_client_config(configure_client());

    // Connect to the remote server
    let socket_addr = SocketAddr::new(remote_ipaddr, port);
    let connecting = client_endpoint
        .connect(socket_addr, SERVER_STRING)
        .map_err(|error| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("could not connect to remote server: {:?}", error),
            )
        })?;

    Ok(connecting.await?)
}

/// Note: we need to take ownership of this `NetworkAddress` (instead of just
/// borrowing the `&[Protocol]` slice) so this future can be `Send + 'static`.
pub async fn resolve_and_connect(addr: NetworkAddress) -> io::Result<QuicSocket> {
    // Open a connection to the remote server
    let connection = open_connection_to_remote(addr).await?;

    // Create the QUIC socket
    QuicSocket::new(connection).await
}

/// Attempts to connect to the remote address
async fn open_connection_to_remote(addr: NetworkAddress) -> io::Result<quinn::Connection> {
    let protos = addr.as_slice();
    if let Some(((ipaddr, port), _addr_suffix)) = parse_ip_udp(protos) {
        // this is an /ip4 or /ip6 address, so we can just connect without any
        // extra resolving or filtering.
        connect_to_remote(port, ipaddr).await
    } else if let Some(((ip_filter, dns_name, port), _addr_suffix)) = parse_dns_udp(protos) {
        // resolve dns name and filter
        let socketaddr_iter = resolve_with_filter(ip_filter, dns_name.as_ref(), port).await?;
        let mut last_err = None;

        // try to connect until the first succeeds
        for socketaddr in socketaddr_iter {
            match connect_to_remote(socketaddr.port(), socketaddr.ip()).await {
                Ok(connection) => return Ok(connection),
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

async fn connect_via_proxy(_proxy_addr: String, _addr: NetworkAddress) -> io::Result<QuicSocket> {
    unimplemented!("CONNECT VIA PROXY!!")
}

fn invalid_addr_error(addr: &NetworkAddress) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("Invalid NetworkAddress: '{}'", addr),
    )
}

#[must_use = "streams do nothing unless polled"]
#[allow(dead_code)]
pub struct QuicListenerStream {
    server_endpoint: quinn::Endpoint,
}

impl Stream for QuicListenerStream {
    type Item = io::Result<(future::Ready<io::Result<QuicSocket>>, NetworkAddress)>;

    fn poll_next(self: Pin<&mut Self>, _context: &mut Context) -> Poll<Option<Self::Item>> {
        unimplemented!("POLL NEXT?")
    }
}

#[must_use = "futures do nothing unless polled"]
pub struct QuicOutbound {
    inner: Pin<Box<dyn Future<Output = io::Result<QuicSocket>> + Send + 'static>>,
}

impl Future for QuicOutbound {
    type Output = io::Result<QuicSocket>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let quic_socket = ready!(Pin::new(&mut self.inner).poll(context))?;
        Poll::Ready(Ok(quic_socket))
    }
}

/// A wrapper around a quinn Connection that implements the AsyncRead/AsyncWrite traits
#[derive(Debug)]
#[allow(dead_code)]
pub struct QuicSocket {
    connection: quinn::Connection,
    send_streams: Vec<Compat<quinn::SendStream>>,
    recv_streams: Vec<Compat<quinn::RecvStream>>,
}

impl QuicSocket {
    pub async fn new(connection: quinn::Connection) -> io::Result<Self> {
        // Open several connection streams
        let mut send_streams = vec![];
        let mut recv_streams = vec![];
        for _ in 0..3 {
            let (send_stream, recv_stream) = connection.open_bi().await?;
            send_streams.push(send_stream.compat_write());
            recv_streams.push(recv_stream.compat());
        }

        // Create the QUIC socket
        Ok(Self {
            connection,
            send_streams,
            recv_streams,
        })
    }
}

impl AsyncRead for QuicSocket {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        // TODO: read from multiple streams?
        let recv_stream = self.recv_streams.first_mut().unwrap();
        Pin::new(recv_stream).poll_read(context, buf)
    }
}

impl AsyncWrite for QuicSocket {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        // TODO: write to multiple streams?
        let send_stream = self.send_streams.first_mut().unwrap();
        Pin::new(send_stream).poll_write(context, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<io::Result<()>> {
        // TODO: flush all streams?
        let send_stream = self.send_streams.first_mut().unwrap();
        Pin::new(send_stream).poll_flush(context)
    }

    fn poll_close(mut self: Pin<&mut Self>, _context: &mut Context) -> Poll<io::Result<()>> {
        // TODO: close all streams?
        let send_stream = self.send_streams.first_mut().unwrap();
        Pin::new(send_stream).poll_close(_context)
    }
}

/// Dummy certificate verifier that treats any certificate as valid
struct SkipServerVerification;

impl SkipServerVerification {
    fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl rustls::client::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::ServerCertVerified::assertion())
    }
}

/// Returns the default client configured that ignores the server certificate
fn configure_client() -> ClientConfig {
    // Create the dummy crypto config
    let crypto_config = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_custom_certificate_verifier(SkipServerVerification::new())
        .with_no_client_auth();

    // Create the QUIC client configuration
    ClientConfig::new(Arc::new(crypto_config))
}

/// Returns the default server configuration along with its dummy certificate
fn configure_server() -> io::Result<(ServerConfig, Vec<u8>)> {
    // Create the dummy server certificate
    let cert = rcgen::generate_simple_self_signed(vec![SERVER_STRING.into()]).unwrap();
    let cert_der = cert.serialize_der().unwrap();
    let priv_key = cert.serialize_private_key_der();
    let priv_key = rustls::PrivateKey(priv_key);
    let cert_chain = vec![rustls::Certificate(cert_der.clone())];

    // Create the QUIC server configuration
    let server_config = ServerConfig::with_single_cert(cert_chain, priv_key).map_err(|error| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Invalid server certificate: {:?}", error),
        )
    })?;

    Ok((server_config, cert_der))
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
        let t = QuicTransport::default().and_then(|mut out, _addr, origin| async move {
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

        let (listener, addr) = t.listen_on("/ip4/127.0.0.1/udp/0".parse().unwrap())?;
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
        let t = QuicTransport::default();

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
