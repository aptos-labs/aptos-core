// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! QUIC Transport
use crate::transport::{utils, Transport};
use aptos_logger::prelude::*;
use aptos_types::{
    network_address::{parse_dns_udp, parse_ip_udp, DnsName, IpFilter, NetworkAddress},
    PeerId,
};
use futures::{
    future::{self, join, Either, Future},
    io::{AsyncRead, AsyncWrite},
    ready,
    stream::FuturesUnordered,
    Stream,
};
use quinn::{Connection, VarInt};
use std::{
    fmt::Debug,
    io,
    io::Error,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

// Useful constants
const SERVER_STRING: &str = "aptos-node";
const STREAM_START_MESSAGE: &str = "start-stream";
const STREAM_START_MESSSAGE_LENGTH: usize = 12; // Update this if the stream start message changes!

/// Transport to build QUIC connections
#[derive(Debug, Clone, Default)]
pub struct QuicTransport {
    server_endpoint: Option<quinn::Endpoint>,
}

impl QuicTransport {
    pub fn new() -> Self {
        Self {
            server_endpoint: None,
        }
    }
}

impl Transport for QuicTransport {
    type Error = ::std::io::Error;
    type Inbound = future::Ready<io::Result<Self::Output>>;
    type Listener = QuicConnectionListener;
    type Outbound = QuicOutboundConnection;
    type Output = QuicConnection;

    fn listen_on(
        &mut self,
        addr: NetworkAddress,
    ) -> Result<(Self::Listener, NetworkAddress), Self::Error> {
        info!("QUIC listening on: {}", addr);
        // Parse the IP address, port and suffix
        let ((ipaddr, port), addr_suffix) =
            parse_ip_udp(addr.as_slice()).ok_or_else(|| utils::invalid_addr_error(&addr))?;
        if !addr_suffix.is_empty() {
            return Err(utils::invalid_addr_error(&addr));
        }

        // If the server endpoint doesn't exist, create it
        info!("Creating QUIC server endpoint!");
        if self.server_endpoint.is_none() {
            // Create the QUIC server endpoint. This will call bind on the socket addr.
            let server_endpoint = create_server_endpoint(ipaddr, port)?;

            // Save the server endpoint
            self.server_endpoint = Some(server_endpoint);
        }

        // Create the QUIC connection listener
        info!("Creating QUIC connection listener!");
        if let Some(server_endpoint) = self.server_endpoint.clone() {
            // Get the listen address
            let listen_addr = NetworkAddress::from_udp(server_endpoint.local_addr()?);

            // Create the QUIC connection listener
            let quic_connection_listener = QuicConnectionListener::new(server_endpoint);
            info!(
                "Created the QUIC connection listener at: {:?}!",
                listen_addr
            );

            Ok((quic_connection_listener, listen_addr))
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "Could not create QUIC connection listener! Missing server endpoint!",
            ))
        }
    }

    fn dial(&self, peer_id: PeerId, addr: NetworkAddress) -> Result<Self::Outbound, Self::Error> {
        info!("Dialing peer {:?} at address: {:?}", peer_id, addr);

        // Verify the address is well formed
        let protos = addr.as_slice();
        parse_ip_udp(protos)
            .map(|_| ())
            .or_else(|| parse_dns_udp(protos).map(|_| ()))
            .ok_or_else(|| utils::invalid_addr_error(&addr))?;

        // Get the server endpoint. Note: `listen_on()` should have been called already.
        let server_endpoint = self.server_endpoint.clone().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Other,
                "The server endpoint was missing! Are we sure listen_on() has been called?",
            )
        })?;

        // Create a proxy address (if required)
        let proxy_addr = utils::create_proxy_addr(protos);

        // Create the outbound connection future
        let dial_future: Pin<
            Box<dyn Future<Output = io::Result<QuicConnection>> + Send + 'static>,
        > = Box::pin(match proxy_addr {
            Some(proxy_addr) => Either::Left(connect_via_proxy(proxy_addr, addr)),
            None => Either::Right(resolve_and_connect(server_endpoint, addr)),
        });

        Ok(QuicOutboundConnection { dial_future })
    }
}

/// Connects to the remote server address.
pub async fn connect_to_remote(
    server_endpoint: quinn::Endpoint,
    port: u16,
    remote_ipaddr: IpAddr,
) -> io::Result<quinn::Connection> {
    // Create the socket addr for the remote addr
    let socket_addr = if remote_ipaddr == Ipv4Addr::UNSPECIFIED {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
    } else {
        SocketAddr::new(remote_ipaddr, port)
    };

    // Connect to the remote server
    let connecting = server_endpoint
        .connect(socket_addr, SERVER_STRING)
        .map_err(|error| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Could not connect to remote server! Error: {:?}", error),
            )
        })?;
    info!("Connected to the remote server at: {:?}", socket_addr);

    // Transform the connecting future into a connection
    Ok(connecting.await?)
}

/// Resolves the remote address and connects to it
pub async fn resolve_and_connect(
    server_endpoint: quinn::Endpoint,
    addr: NetworkAddress,
) -> io::Result<QuicConnection> {
    // Open a connection to the remote server
    let connection = open_connection_to_remote(server_endpoint, addr).await?;

    // Create the QUIC connection
    create_quic_connection(connection).await
}

/// Attempts to connect to the remote addressc
async fn open_connection_to_remote(
    server_endpoint: quinn::Endpoint,
    addr: NetworkAddress,
) -> io::Result<quinn::Connection> {
    let protos = addr.as_slice();
    if let Some(((ipaddr, port), _addr_suffix)) = parse_ip_udp(protos) {
        // This is an /ip4 or /ip6 address, so we can connect without resolving of filtering
        connect_to_remote(server_endpoint, port, ipaddr).await
    } else if let Some(((ip_filter, dns_name, port), _addr_suffix)) = parse_dns_udp(protos) {
        // This is a /dns4 or /dns6 address, so we need to resolve the DNS name and filter
        resolve_dns_and_connect(server_endpoint, ip_filter, dns_name, port).await
    } else {
        // This is an invalid address
        Err(utils::invalid_addr_error(&addr))
    }
}

/// Resolves the given DNS name and connects to the first socket address that works
async fn resolve_dns_and_connect(
    server_endpoint: quinn::Endpoint,
    ip_filter: IpFilter,
    dns_name: &DnsName,
    port: u16,
) -> Result<Connection, Error> {
    // Resolve the DNS name and filter
    let socketaddr_iter = utils::resolve_with_filter(ip_filter, dns_name.as_ref(), port).await?;
    let mut last_err = None;

    // Connect to the first socket address that works
    for socketaddr in socketaddr_iter {
        info!(
            "Attempting to connect to DNS resolved socket address: {:?}",
            socketaddr
        );
        match connect_to_remote(server_endpoint.clone(), socketaddr.port(), socketaddr.ip()).await {
            Ok(connection) => {
                info!(
                    "Connected to the remote server socket address: {:?}",
                    socketaddr
                );
                return Ok(connection);
            },
            Err(err) => {
                last_err = {
                    info!(
                        "Failed to connect to socket address: {:?}. Error: {:?}",
                        socketaddr, err
                    );
                    Some(err)
                }
            },
        }
    }

    // We failed to connect to any of the socket addresses
    Err(last_err.unwrap_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Could not resolve the DNS name to any address! Name: {}, IP filter: {:?}",
                dns_name.as_ref(),
                ip_filter,
            ),
        )
    }))
}

// TODO: complete me?
async fn connect_via_proxy(
    _proxy_addr: String,
    _addr: NetworkAddress,
) -> io::Result<QuicConnection> {
    unimplemented!("CONNECT VIA PROXY!!")
}

#[must_use = "streams do nothing unless polled"]
#[allow(dead_code)]
pub struct QuicConnectionListener {
    server_endpoint: Pin<Box<quinn::Endpoint>>,
    pending_connections: Pin<Box<FuturesUnordered<quinn::Connecting>>>,
    pending_quic_connections: Pin<Box<FuturesUnordered<PendingQuicConnection>>>,
}

impl QuicConnectionListener {
    pub fn new(server_endpoint: quinn::Endpoint) -> Self {
        Self {
            server_endpoint: Box::pin(server_endpoint),
            pending_connections: Box::pin(FuturesUnordered::new()),
            pending_quic_connections: Box::pin(FuturesUnordered::new()),
        }
    }
}

impl Stream for QuicConnectionListener {
    type Item = io::Result<(future::Ready<io::Result<QuicConnection>>, NetworkAddress)>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Self::Item>> {
        // Check if there are any new pending connections to accept
        let server_endpoint = self.server_endpoint.clone();
        let mut server_accept = Box::pin(server_endpoint.accept());
        if let Poll::Ready(Some(pending_connection)) = server_accept.as_mut().poll(context) {
            info!("(QuicConnectionListener) Got a new pending connection!");

            // Add the new pending connection to the list of pending connections
            self.pending_connections.as_mut().push(pending_connection);
        }

        // Check if there are any pending connections that are now ready
        if let Poll::Ready(Some(Ok(connection))) =
            self.pending_connections.as_mut().poll_next(context)
        {
            info!("(QuicConnectionListener) Got a new pending QUIC connection!");

            // Create the pending QUIC connection
            let pending_quic_connection = PendingQuicConnection::new(connection);

            // Add the new pending QUIC connection to the list of pending QUIC connections
            self.pending_quic_connections.push(pending_quic_connection);
        }

        // Check if there are any pending QUIC connections that are now ready
        if let Poll::Ready(Some(Ok(quic_connection))) =
            self.pending_quic_connections.as_mut().poll_next(context)
        {
            info!("(QuicConnectionListener) Got a new and ready QUIC connection!");

            // Get the remote address
            let remote_address =
                NetworkAddress::from_udp(quic_connection.connection.remote_address());

            // Return the QUIC connection and remote address
            return Poll::Ready(Some(Ok((
                future::ready(Ok(quic_connection)),
                remote_address,
            ))));
        }

        Poll::Pending
    }
}

/// A future that resolves to a QUIC connection
#[must_use = "futures do nothing unless polled"]
pub struct QuicOutboundConnection {
    dial_future: Pin<Box<dyn Future<Output = io::Result<QuicConnection>> + Send + 'static>>,
}

impl Future for QuicOutboundConnection {
    type Output = io::Result<QuicConnection>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let quic_connection = ready!(Pin::new(&mut self.dial_future).poll(context))?;
        Poll::Ready(Ok(quic_connection))
    }
}

/// A set of pending QUIC connections
#[derive(Debug)]
#[allow(dead_code)]
pub struct PendingQuicConnection {
    pending_quic_connections: Pin<
        Box<
            FuturesUnordered<
                Pin<Box<dyn Future<Output = io::Result<QuicConnection>> + Send + 'static>>,
            >,
        >,
    >,
}

impl PendingQuicConnection {
    pub fn new(connection: quinn::Connection) -> Self {
        let pending_quic_connections = Box::pin(FuturesUnordered::new());
        let future: Pin<Box<dyn Future<Output = io::Result<QuicConnection>> + Send + 'static>> =
            Box::pin(create_quic_connection(connection));
        pending_quic_connections.push(future);
        Self {
            pending_quic_connections,
        }
    }
}

impl Future for PendingQuicConnection {
    type Output = io::Result<QuicConnection>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        // Poll the pending QUIC connections to see if any of them are ready
        match self.pending_quic_connections.as_mut().poll_next(context) {
            Poll::Ready(Some(Ok(quic_connection))) => {
                info!("(PendingQuicConnection) Got a new and ready QUIC connection!");
                Poll::Ready(Ok(quic_connection))
            },
            Poll::Ready(Some(Err(error))) => {
                // Something went wrong!
                info!("(PendingQuicConnection) Got an error: {:?}", error);
                let error = io::Error::new(
                    io::ErrorKind::Other,
                    format!("Could not accept connection: {:?}", error),
                );
                Poll::Ready(Err(error))
            },
            Poll::Ready(None) => Poll::Ready(Err(io::Error::new(
                io::ErrorKind::Other,
                "No pending QUIC connections!",
            ))),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Creates a QUIC connection from a QUINN connection
async fn create_quic_connection(connection: quinn::Connection) -> io::Result<QuicConnection> {
    QuicConnection::new(connection).await
}

/// A wrapper around a quinn Connection that implements the AsyncRead/AsyncWrite traits
#[derive(Debug)]
#[allow(dead_code)]
pub struct QuicConnection {
    connection: quinn::Connection,
    send_stream: Compat<quinn::SendStream>,
    recv_stream: Compat<quinn::RecvStream>,
}

impl QuicConnection {
    pub async fn new(connection: quinn::Connection) -> io::Result<Self> {
        // Open a uni-directional stream and send a stream start
        // message so that the receiver can accept it.
        let send_connection = connection.clone();
        let send_stream = tokio::task::spawn(async move {
            // Open the stream
            let mut send_stream = send_connection.open_uni().await.unwrap();
            info!(
                "(QUIC remote: {:?}) Opened a new send stream!",
                send_connection.remote_address(),
            );

            // Send a stream start message
            send_stream
                .write_all(STREAM_START_MESSAGE.as_bytes())
                .await
                .unwrap();
            info!(
                "(QUIC remote: {:?}) Wrote stream start message",
                send_connection.remote_address(),
            );

            send_stream
        });

        // Accept the stream so that we have a receiver
        let recv_connection = connection.clone();
        let recv_stream = tokio::task::spawn(async move {
            // Accept the stream
            let mut recv_stream = recv_connection.accept_uni().await.unwrap();
            info!(
                "(QUIC remote: {:?}) Accepted a new recv stream!",
                recv_connection.remote_address(),
            );

            // Read the stream start message
            let mut buf = [0; STREAM_START_MESSSAGE_LENGTH];
            recv_stream.read_exact(&mut buf).await.unwrap();
            info!(
                "(QUIC remote: {:?}) Read stream start message",
                recv_connection.remote_address(),
            );

            // Verify the stream start message
            if buf == STREAM_START_MESSAGE.as_bytes() {
                info!(
                    "(QUIC remote: {:?}) Stream start message is valid!",
                    recv_connection.remote_address(),
                );
            } else {
                panic!("The stream start message is invalid!!");
            }

            recv_stream
        });

        let (send_stream, recv_stream) = join(send_stream, recv_stream).await;
        let send_stream = send_stream?;
        let recv_stream = recv_stream?;

        // Create the QUIC connection
        Ok(Self {
            connection,
            send_stream: send_stream.compat_write(),
            recv_stream: recv_stream.compat(),
        })
    }
}

impl AsyncRead for QuicConnection {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.recv_stream).poll_read(context, buf)
    }
}

impl AsyncWrite for QuicConnection {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        // TODO: write to multiple streams?
        let send_stream = &mut self.send_stream;
        Pin::new(send_stream).poll_write(context, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<io::Result<()>> {
        // TODO: flush all streams?
        let send_stream = &mut self.send_stream;
        Pin::new(send_stream).poll_flush(context)
    }

    fn poll_close(mut self: Pin<&mut Self>, _context: &mut Context) -> Poll<io::Result<()>> {
        // TODO: close all streams?
        let send_stream = &mut self.send_stream;
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
fn configure_client() -> quinn::ClientConfig {
    // Create the dummy crypto config
    let crypto_config = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_custom_certificate_verifier(SkipServerVerification::new())
        .with_no_client_auth();

    // Create the client transport config
    let transport_config = create_transport_config();

    // Create the QUIC client configuration
    let mut client = quinn::ClientConfig::new(Arc::new(crypto_config));
    client.transport_config(transport_config);
    client
}

/// Returns a new transport config
fn create_transport_config() -> Arc<quinn::TransportConfig> {
    let mut transport_config = quinn::TransportConfig::default();

    // Set the idle timeout and keep alive
    transport_config.max_idle_timeout(Some(Duration::from_secs(120).try_into().unwrap()));
    transport_config.keep_alive_interval(Some(Duration::from_secs(60)));

    // Optimize the send and receiver buffer sizes according to estimated
    // RTT and bandwidth. This was taken from the QUINN source code...
    let expected_rtt: u32 = 255; // 255 ms
    let max_stream_bandwidth = 300 * 1000 * 1000; // 300 MB
    let stream_receive_window = (max_stream_bandwidth / 1000) * expected_rtt;
    let stream_send_window = 8 * stream_receive_window;
    transport_config.stream_receive_window(VarInt::from_u32(stream_receive_window));
    transport_config.send_window(stream_send_window as u64);

    Arc::new(transport_config)
}

/// Returns the default server configuration along with its dummy certificate
fn configure_server() -> io::Result<(quinn::ServerConfig, Vec<u8>)> {
    // Create the dummy server certificate
    let cert = rcgen::generate_simple_self_signed(vec![SERVER_STRING.into()]).unwrap();
    let cert_der = cert.serialize_der().unwrap();
    let priv_key = cert.serialize_private_key_der();
    let priv_key = rustls::PrivateKey(priv_key);
    let cert_chain = vec![rustls::Certificate(cert_der.clone())];

    // Create the server transport config
    let transport_config = create_transport_config();

    // Create the QUIC server configuration
    let mut server_config =
        quinn::ServerConfig::with_single_cert(cert_chain, priv_key).map_err(|error| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Invalid server certificate: {:?}", error),
            )
        })?;
    server_config.transport_config(transport_config);

    Ok((server_config, cert_der))
}

/// Creates a QUIC server endpoint
fn create_server_endpoint(
    ipaddr: IpAddr,
    port: u16,
) -> Result<quinn::Endpoint, <QuicTransport as Transport>::Error> {
    info!(
        "Creating the server endpoint at: {:?}:{:?}. This will call bind!",
        ipaddr, port
    );

    // Create the QUIC server configuration
    let (server_config, _server_certificate) = configure_server()?;

    // Create the QUIC server endpoint
    let socket_addr = SocketAddr::new(ipaddr, port);
    let mut server_endpoint = quinn::Endpoint::server(server_config, socket_addr)?;
    server_endpoint.set_default_client_config(configure_client()); // Required to skip certificate verification

    Ok(server_endpoint)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::transport::{utils::resolve_with_filter, ConnectionOrigin, Transport, TransportExt};
    use aptos_types::{network_address::IpFilter, PeerId};
    use futures::{
        future::{join, FutureExt},
        io::{AsyncReadExt, AsyncWriteExt},
        StreamExt,
    };
    use std::{str::FromStr, time::Duration};
    use tokio::{runtime::Runtime, time::timeout};

    #[tokio::test]
    async fn simple_listen_and_dial() -> Result<(), ::std::io::Error> {
        let mut transport = QuicTransport::new().and_then(|mut out, addr, origin| async move {
            info!(
                "(simple_listen_and_dial: {:?}) Got a new connection! Addr: {:?}",
                origin, addr
            );
            match origin {
                ConnectionOrigin::Inbound => {
                    info!(
                        "(simple_listen_and_dial: {:?}, {:?}) Reading hello packet!",
                        origin, addr
                    );
                    let mut buf = [0; 1];
                    out.read_exact(&mut buf).await?;
                    assert_eq!(&buf, b"H");

                    info!(
                        "(simple_listen_and_dial: {:?}, {:?}) Reading hello2 packet!",
                        origin, addr
                    );
                    let mut buf = [0; 2];
                    out.read_exact(&mut buf).await?;
                    assert_eq!(&buf, b"HH");

                    info!(
                        "(simple_listen_and_dial: {:?}, {:?}) Writing first data packet!",
                        origin, addr
                    );
                    out.write_all(b"EEE").await?;

                    info!(
                        "(simple_listen_and_dial: {:?}, {:?}) Writing second data packet!",
                        origin, addr
                    );
                    out.write_all(b"FFFF").await?;

                    info!(
                        "(simple_listen_and_dial: {:?}, {:?}) Reading third data packet!",
                        origin, addr
                    );
                    let mut buf = [0; 5];
                    out.read_exact(&mut buf).await?;
                    assert_eq!(&buf, b"AAAAA");

                    info!(
                        "(simple_listen_and_dial: {:?}, {:?}) Writing fourth data packet!",
                        origin, addr
                    );
                    out.write_all(b"WWWWWW").await?;
                    out.close().await?;

                    info!("Everything looks good!");
                },
                ConnectionOrigin::Outbound => {
                    info!(
                        "(simple_listen_and_dial: {:?}, {:?}) Writing hello packet!",
                        origin, addr
                    );
                    out.write_all(b"H").await?;

                    info!(
                        "(simple_listen_and_dial: {:?}, {:?}) Writing hello2 packet!",
                        origin, addr
                    );
                    out.write_all(b"HH").await?;

                    info!(
                        "(simple_listen_and_dial: {:?}, {:?}) Reading first data packet!",
                        origin, addr
                    );

                    let mut buf = [0; 3];
                    out.read_exact(&mut buf).await?;
                    assert_eq!(&buf, b"EEE");

                    info!(
                        "(simple_listen_and_dial: {:?}, {:?}) Reading second data packet!",
                        origin, addr
                    );

                    let mut buf = [0; 4];
                    out.read_exact(&mut buf).await?;
                    assert_eq!(&buf, b"FFFF");

                    info!(
                        "(simple_listen_and_dial: {:?}, {:?}) Writing third data packet!",
                        origin, addr
                    );
                    out.write_all(b"AAAAA").await?;

                    info!(
                        "(simple_listen_and_dial: {:?}, {:?}) Reading fourth data packet!",
                        origin, addr
                    );

                    let mut buf = [0; 6];
                    out.read_exact(&mut buf).await?;
                    assert_eq!(&buf, b"WWWWWW");
                    out.close().await?;

                    info!("Got here!")
                },
            }
            Ok(())
        });

        let (listener, addr) = transport.listen_on("/ip4/127.0.0.1/udp/0".parse().unwrap())?;
        let peer_id = PeerId::random();
        let dial = transport.dial(peer_id, addr)?;
        let listener = listener.into_future().then(|(maybe_result, _stream)| {
            let (incoming, _addr) = maybe_result.unwrap().unwrap();
            incoming.map(Result::unwrap)
        });

        let (result, _) = timeout(Duration::from_secs(15), join(dial, listener))
            .await
            .unwrap();
        result.unwrap();

        Ok(())
    }

    #[tokio::test]
    async fn simple_listen_and_dial_large_data() -> Result<(), ::std::io::Error> {
        let mut transport = QuicTransport::new().and_then(|mut out, _addr, origin| async move {
            for i in 0..100 {
                info!("Executing loop {:?}!", i);
                match origin {
                    ConnectionOrigin::Inbound => {
                        info!("Sending a large chunk of data!!");
                        let large_data = [0; 1024 * 10];
                        out.write_all(&large_data).await?;

                        info!("Receiving a large chunk of data!!");
                        let mut buf = [0; 1024 * 10];
                        out.read_exact(&mut buf).await?;
                        assert_eq!(buf, [1; 1024 * 10]);
                    },
                    ConnectionOrigin::Outbound => {
                        info!("Receiving a large chunk of data!!");
                        let mut buf = [0; 1024 * 10];
                        out.read_exact(&mut buf).await?;
                        assert_eq!(buf, [0; 1024 * 10]);

                        info!("Sending a large chunk of data!!");
                        let large_data = [1; 1024 * 10];
                        out.write_all(&large_data).await?;
                    },
                }
            }

            out.close().await?;
            Ok(())
        });

        let (listener, addr) = transport.listen_on("/ip4/127.0.0.1/udp/0".parse().unwrap())?;
        let peer_id = PeerId::random();
        let dial = transport.dial(peer_id, addr)?;
        let listener = listener.into_future().then(|(maybe_result, _stream)| {
            let (incoming, _addr) = maybe_result.unwrap().unwrap();
            incoming.map(Result::unwrap)
        });

        let (result, _) = timeout(Duration::from_secs(15), join(dial, listener))
            .await
            .unwrap();
        result.unwrap();

        Ok(())
    }

    #[test]
    fn unsupported_multiaddrs() {
        let mut t = QuicTransport::default();

        let result = t.listen_on("/memory/0".parse().unwrap());
        assert!(result.is_err());

        let peer_id = PeerId::random();
        let result = t.dial(peer_id, "/memory/22".parse().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn dns_address_check() {
        // Test all protos
        let address = NetworkAddress::from_str("/dns/aptos-node-1-validator/udp/6180/noise-ik/0x94db6c0c92f719cdd2bce341cfb996aa0f0bd77d35b4deac706158a9128fa01c/handshake/0").unwrap();
        let protos = address.as_slice();
        parse_ip_udp(protos)
            .map(|_| ())
            .or_else(|| parse_dns_udp(protos).map(|_| ()))
            .ok_or_else(|| utils::invalid_addr_error(&address))
            .unwrap();

        // Test only part of the protos
        let address = NetworkAddress::from_str("/dns/aptos-node-1-validator/udp/6180/").unwrap();
        let protos = address.as_slice();
        parse_ip_udp(protos)
            .map(|_| ())
            .or_else(|| parse_dns_udp(protos).map(|_| ()))
            .ok_or_else(|| utils::invalid_addr_error(&address))
            .unwrap();
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
