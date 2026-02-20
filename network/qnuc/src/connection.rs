// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Connection management: ties together the UDP socket, Noise crypto, reliability,
//! ordering, and stream multiplexing into a single connection abstraction.

use crate::{
    crypto::{DatagramCrypto, NoiseHandshake},
    error::{QnucError, Result},
    packet::{Packet, PacketHeader, PacketType},
    reliability::ReliabilityConfig,
    stream::Stream,
};
use aptos_crypto::x25519;
use bytes::Bytes;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::net::UdpSocket;

/// Configuration for a QUIC-like connection.
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub reliability: ReliabilityConfig,
    pub idle_timeout: Duration,
    pub keepalive_interval: Duration,
    pub max_streams: u64,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            reliability: ReliabilityConfig::default(),
            idle_timeout: Duration::from_secs(30),
            keepalive_interval: Duration::from_secs(5),
            max_streams: 256,
        }
    }
}

/// State of the connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Handshake in progress.
    Handshaking,
    /// Connection established, data can flow.
    Established,
    /// Connection is being closed.
    Closing,
    /// Connection is fully closed.
    Closed,
}

/// A QUIC-like UDP connection to a single remote peer.
pub struct Connection {
    connection_id: u64,
    state: ConnectionState,
    remote_addr: SocketAddr,
    socket: Arc<UdpSocket>,
    crypto: Option<DatagramCrypto>,
    streams: HashMap<u64, Stream>,
    config: ConnectionConfig,
    last_activity: Instant,
    remote_public_key: Option<x25519::PublicKey>,
}

impl Connection {
    /// Create a new connection (pre-handshake).
    pub fn new(
        connection_id: u64,
        remote_addr: SocketAddr,
        socket: Arc<UdpSocket>,
        config: ConnectionConfig,
    ) -> Self {
        Self {
            connection_id,
            state: ConnectionState::Handshaking,
            remote_addr,
            socket,
            crypto: None,
            streams: HashMap::new(),
            config,
            last_activity: Instant::now(),
            remote_public_key: None,
        }
    }

    /// Perform the Noise IK handshake as the initiator (dialer).
    pub async fn connect_outbound(
        &mut self,
        local_private_key: x25519::PrivateKey,
        remote_public_key: x25519::PublicKey,
    ) -> Result<()> {
        let handshake = NoiseHandshake::new(local_private_key);
        let prologue = self.build_prologue();

        // Timestamp payload for anti-replay
        let timestamp = aptos_infallible::duration_since_epoch()
            .as_millis() as u64;
        let payload = timestamp.to_le_bytes();

        // Build and send initiator message
        let (init_state, init_msg) =
            handshake.build_initiator_message(&prologue, remote_public_key, &payload)?;

        let header = PacketHeader::new(
            PacketType::HandshakeInit,
            self.connection_id,
            0,
            0,
            init_msg.len() as u16,
        );
        let pkt = Packet::new(header, Bytes::from(init_msg));
        self.send_raw(&pkt.encode()).await?;

        // Receive responder message
        let mut recv_buf = vec![0u8; 65535];
        let (n, _addr) = tokio::time::timeout(
            Duration::from_secs(10),
            self.socket.recv_from(&mut recv_buf),
        )
        .await
        .map_err(|_| QnucError::ConnectionTimeout)?
        .map_err(QnucError::Io)?;

        let resp_pkt = Packet::decode(&recv_buf[..n])?;
        if resp_pkt.header.packet_type != PacketType::HandshakeResp {
            return Err(QnucError::NoiseHandshake(
                "expected HandshakeResp".to_string(),
            ));
        }

        let (_, session) = handshake.finalize_initiator(init_state, &resp_pkt.payload)?;
        self.crypto = Some(DatagramCrypto::new(session));
        self.remote_public_key = Some(remote_public_key);
        self.state = ConnectionState::Established;
        self.last_activity = Instant::now();

        Ok(())
    }

    /// Perform the Noise IK handshake as the responder (listener).
    pub async fn accept_inbound(
        &mut self,
        local_private_key: x25519::PrivateKey,
        init_packet: &Packet,
    ) -> Result<x25519::PublicKey> {
        let handshake = NoiseHandshake::new(local_private_key);
        let prologue = self.build_prologue();

        let (remote_pub, session, _payload, resp_msg) =
            handshake.handle_initiator_message(&prologue, &init_packet.payload, None)?;

        // Send response
        let header = PacketHeader::new(
            PacketType::HandshakeResp,
            self.connection_id,
            0,
            0,
            resp_msg.len() as u16,
        );
        let pkt = Packet::new(header, Bytes::from(resp_msg));
        self.send_raw(&pkt.encode()).await?;

        self.crypto = Some(DatagramCrypto::new(session));
        self.remote_public_key = Some(remote_pub);
        self.state = ConnectionState::Established;
        self.last_activity = Instant::now();

        Ok(remote_pub)
    }

    /// Send a message on a given stream.
    pub async fn send_message(&mut self, stream_id: u64, message: &[u8]) -> Result<()> {
        if self.state != ConnectionState::Established {
            return Err(QnucError::ConnectionClosed);
        }

        let stream = self.get_or_create_stream(stream_id);
        let packets = stream.prepare_send(message)?;
        let crypto = self
            .crypto
            .as_mut()
            .ok_or(QnucError::ConnectionClosed)?;

        for (_seq, pkt_bytes) in packets {
            let encrypted = crypto.encrypt(&pkt_bytes)?;
            self.socket
                .send_to(&encrypted, self.remote_addr)
                .await
                .map_err(QnucError::Io)?;
        }

        self.last_activity = Instant::now();
        Ok(())
    }

    /// Process a received raw datagram.
    /// Returns (stream_id, messages) for any fully reassembled messages.
    pub fn process_datagram(&mut self, data: &[u8]) -> Result<Vec<(u64, Bytes)>> {
        self.last_activity = Instant::now();

        // If connection is established, decrypt first
        let decrypted;
        let pkt_data = if self.state == ConnectionState::Established {
            let crypto = self
                .crypto
                .as_mut()
                .ok_or(QnucError::ConnectionClosed)?;
            decrypted = crypto.decrypt(data)?;
            &decrypted[..]
        } else {
            data
        };

        let pkt = Packet::decode(pkt_data)?;
        let mut result = Vec::new();

        match pkt.header.packet_type {
            PacketType::Data => {
                let stream = self.get_or_create_stream(pkt.header.stream_id);
                let messages =
                    stream.process_received(pkt.header.sequence_number, &pkt.payload)?;
                for msg in messages {
                    result.push((pkt.header.stream_id, msg));
                }
            },
            PacketType::Ack => {
                let stream = self.get_or_create_stream(pkt.header.stream_id);
                let _ = stream.process_ack(&pkt.payload)?;
            },
            PacketType::Close => {
                self.state = ConnectionState::Closed;
            },
            PacketType::Ping => {
                // Respond with pong (handled at the transport layer)
            },
            PacketType::Pong => {
                // Update keepalive state
            },
            _ => {},
        }

        Ok(result)
    }

    /// Send ACKs for all streams that have pending acknowledgements.
    pub async fn flush_acks(&mut self) -> Result<()> {
        if self.state != ConnectionState::Established {
            return Ok(());
        }

        let crypto = self
            .crypto
            .as_mut()
            .ok_or(QnucError::ConnectionClosed)?;

        let stream_ids: Vec<u64> = self.streams.keys().copied().collect();
        for sid in stream_ids {
            if let Some(stream) = self.streams.get(&sid) {
                let ack_bytes = stream.generate_ack_packet();
                let encrypted = crypto.encrypt(&ack_bytes)?;
                self.socket
                    .send_to(&encrypted, self.remote_addr)
                    .await
                    .map_err(QnucError::Io)?;
            }
        }

        Ok(())
    }

    /// Retransmit packets for all streams.
    pub async fn retransmit(&mut self) -> Result<()> {
        if self.state != ConnectionState::Established {
            return Ok(());
        }

        let crypto = self
            .crypto
            .as_mut()
            .ok_or(QnucError::ConnectionClosed)?;

        let stream_ids: Vec<u64> = self.streams.keys().copied().collect();
        for sid in stream_ids {
            if let Some(stream) = self.streams.get_mut(&sid) {
                let retransmits = stream.get_retransmissions();
                for (_seq, pkt_bytes) in retransmits {
                    let encrypted = crypto.encrypt(&pkt_bytes)?;
                    self.socket
                        .send_to(&encrypted, self.remote_addr)
                        .await
                        .map_err(QnucError::Io)?;
                }
            }
        }

        Ok(())
    }

    /// Send a close packet.
    pub async fn close(&mut self) -> Result<()> {
        if self.state == ConnectionState::Closed {
            return Ok(());
        }

        let header = PacketHeader::new(PacketType::Close, self.connection_id, 0, 0, 0);
        let pkt = Packet::new(header, Bytes::new());
        let pkt_bytes = pkt.encode();

        if let Some(crypto) = self.crypto.as_mut() {
            let encrypted = crypto.encrypt(&pkt_bytes)?;
            let _ = self.socket.send_to(&encrypted, self.remote_addr).await;
        } else {
            let _ = self.socket.send_to(&pkt_bytes, self.remote_addr).await;
        }

        self.state = ConnectionState::Closed;
        Ok(())
    }

    /// Check if the connection has timed out due to inactivity.
    pub fn is_timed_out(&self) -> bool {
        self.last_activity.elapsed() > self.config.idle_timeout
    }

    pub fn state(&self) -> ConnectionState {
        self.state
    }

    pub fn connection_id(&self) -> u64 {
        self.connection_id
    }

    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }

    pub fn remote_public_key(&self) -> Option<x25519::PublicKey> {
        self.remote_public_key
    }

    fn get_or_create_stream(&mut self, stream_id: u64) -> &mut Stream {
        self.streams.entry(stream_id).or_insert_with(|| {
            Stream::new(stream_id, self.connection_id, self.config.reliability.clone())
        })
    }

    fn build_prologue(&self) -> Vec<u8> {
        let mut prologue = Vec::with_capacity(24);
        prologue.extend_from_slice(b"aptos-qnuc-v1");
        prologue.extend_from_slice(&self.connection_id.to_be_bytes());
        prologue
    }

    async fn send_raw(&self, data: &[u8]) -> Result<()> {
        self.socket
            .send_to(data, self.remote_addr)
            .await
            .map_err(QnucError::Io)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::Uniform;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn make_keypair(seed: [u8; 32]) -> (x25519::PrivateKey, x25519::PublicKey) {
        let mut rng = StdRng::from_seed(seed);
        let priv_key = x25519::PrivateKey::generate(&mut rng);
        let pub_key = priv_key.public_key();
        (priv_key, pub_key)
    }

    #[tokio::test]
    async fn test_connection_handshake_and_data() {
        let server_sock = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let server_addr = server_sock.local_addr().unwrap();
        let client_sock = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());

        let (client_priv, _client_pub) = make_keypair([1u8; 32]);
        // Generate the same server key twice from the same seed
        let (server_priv_for_server, server_pub) = make_keypair([2u8; 32]);

        let config = ConnectionConfig::default();

        let server_sock_clone = server_sock.clone();
        let server_handle = tokio::spawn(async move {
            let mut recv_buf = vec![0u8; 65535];
            let (n, from_addr) = server_sock_clone.recv_from(&mut recv_buf).await.unwrap();

            let init_pkt = Packet::decode(&recv_buf[..n]).unwrap();
            assert_eq!(init_pkt.header.packet_type, PacketType::HandshakeInit);

            let mut conn = Connection::new(
                init_pkt.header.connection_id,
                from_addr,
                server_sock_clone,
                ConnectionConfig::default(),
            );
            let remote_pub = conn
                .accept_inbound(server_priv_for_server, &init_pkt)
                .await
                .unwrap();
            assert_eq!(conn.state(), ConnectionState::Established);
            (conn, remote_pub)
        });

        let mut client_conn =
            Connection::new(42, server_addr, client_sock.clone(), config);
        client_conn
            .connect_outbound(client_priv, server_pub)
            .await
            .unwrap();
        assert_eq!(client_conn.state(), ConnectionState::Established);

        let (server_conn, _) = server_handle.await.unwrap();
        assert_eq!(server_conn.state(), ConnectionState::Established);
    }
}
