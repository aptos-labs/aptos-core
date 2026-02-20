// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Top-level transport that provides the public API for establishing
//! QUIC-like UDP connections.
//!
//! This module provides `QnucTransport` which manages:
//! - Listening for incoming connections
//! - Dialing out to remote peers
//! - Running the connection event loop (ACKs, retransmissions, keepalives)

use crate::{
    connection::{Connection, ConnectionConfig, ConnectionState},
    error::{QnucError, Result},
    packet::{Packet, PacketType},
};
use aptos_crypto::{x25519, ValidCryptoMaterial};
use bytes::Bytes;
use std::{
    collections::HashMap,
    convert::TryFrom,
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};
use tokio::{
    net::UdpSocket,
    sync::{mpsc, Mutex},
};

/// Event emitted by the transport to the application layer.
#[derive(Debug)]
pub enum TransportEvent {
    /// A new inbound connection was established.
    NewConnection {
        connection_id: u64,
        remote_addr: SocketAddr,
        remote_public_key: x25519::PublicKey,
    },
    /// A message was received on a stream.
    Message {
        connection_id: u64,
        stream_id: u64,
        data: Bytes,
    },
    /// A connection was closed.
    ConnectionClosed {
        connection_id: u64,
    },
}

/// The main QUIC-like UDP transport.
pub struct QnucTransport {
    /// Stored as bytes so we can reconstruct the key for each connection
    /// (x25519::PrivateKey does not implement Clone).
    local_private_key_bytes: Vec<u8>,
    local_public_key: x25519::PublicKey,
    socket: Arc<UdpSocket>,
    config: ConnectionConfig,
    connections: Arc<Mutex<HashMap<SocketAddr, Connection>>>,
    next_connection_id: Arc<Mutex<u64>>,
}

impl QnucTransport {
    /// Bind to a local address and create the transport.
    pub async fn bind(
        addr: SocketAddr,
        private_key: x25519::PrivateKey,
        config: ConnectionConfig,
    ) -> Result<Self> {
        let public_key = private_key.public_key();
        let key_bytes = private_key.to_bytes();
        let socket = UdpSocket::bind(addr).await.map_err(QnucError::Io)?;
        Ok(Self {
            local_private_key_bytes: key_bytes,
            local_public_key: public_key,
            socket: Arc::new(socket),
            config,
            connections: Arc::new(Mutex::new(HashMap::new())),
            next_connection_id: Arc::new(Mutex::new(1)),
        })
    }

    fn reconstruct_private_key(&self) -> x25519::PrivateKey {
        x25519::PrivateKey::try_from(self.local_private_key_bytes.as_slice())
            .expect("stored key bytes should always be valid")
    }

    /// Get the local address this transport is bound to.
    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.socket.local_addr().map_err(QnucError::Io)
    }

    pub fn local_public_key(&self) -> x25519::PublicKey {
        self.local_public_key
    }

    /// Dial a remote peer. Performs the Noise IK handshake.
    /// Returns the connection_id on success.
    pub async fn dial(
        &self,
        remote_addr: SocketAddr,
        remote_public_key: x25519::PublicKey,
    ) -> Result<u64> {
        let conn_id = {
            let mut id = self.next_connection_id.lock().await;
            let current = *id;
            *id += 1;
            current
        };

        // Create a dedicated socket for this outbound connection
        let dial_socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(QnucError::Io)?;
        let dial_socket = Arc::new(dial_socket);

        let mut conn =
            Connection::new(conn_id, remote_addr, dial_socket.clone(), self.config.clone());

        conn.connect_outbound(self.reconstruct_private_key(), remote_public_key)
            .await?;

        self.connections
            .lock()
            .await
            .insert(remote_addr, conn);

        Ok(conn_id)
    }

    /// Send a message to a connected peer.
    pub async fn send(
        &self,
        remote_addr: SocketAddr,
        stream_id: u64,
        data: &[u8],
    ) -> Result<()> {
        let mut connections = self.connections.lock().await;
        let conn = connections
            .get_mut(&remote_addr)
            .ok_or(QnucError::ConnectionClosed)?;
        conn.send_message(stream_id, data).await
    }

    /// Run the receive loop, dispatching events to the provided channel.
    /// This should be spawned as a background task.
    pub async fn run_recv_loop(
        &self,
        event_tx: mpsc::Sender<TransportEvent>,
    ) -> Result<()> {
        let mut buf = vec![0u8; 65536];

        loop {
            let recv_result = tokio::time::timeout(
                Duration::from_millis(100),
                self.socket.recv_from(&mut buf),
            )
            .await;

            match recv_result {
                Ok(Ok((n, from_addr))) => {
                    let data = &buf[..n];
                    let mut connections = self.connections.lock().await;

                    if let Some(conn) = connections.get_mut(&from_addr) {
                        // Existing connection
                        match conn.process_datagram(data) {
                            Ok(messages) => {
                                let conn_id = conn.connection_id();
                                for (stream_id, msg_data) in messages {
                                    let _ = event_tx
                                        .send(TransportEvent::Message {
                                            connection_id: conn_id,
                                            stream_id,
                                            data: msg_data,
                                        })
                                        .await;
                                }
                            },
                            Err(e) => {
                                aptos_logger::warn!(
                                    "Error processing datagram from {}: {}",
                                    from_addr,
                                    e
                                );
                            },
                        }
                    } else {
                        // Possibly a new inbound connection
                        match Packet::decode(data) {
                            Ok(pkt) if pkt.header.packet_type == PacketType::HandshakeInit => {
                                let conn_id = {
                                    let mut id = self.next_connection_id.lock().await;
                                    let current = *id;
                                    *id += 1;
                                    current
                                };

                                let mut conn = Connection::new(
                                    conn_id,
                                    from_addr,
                                    self.socket.clone(),
                                    self.config.clone(),
                                );

                                match conn
                                    .accept_inbound(self.reconstruct_private_key(), &pkt)
                                    .await
                                {
                                    Ok(remote_pub) => {
                                        let _ = event_tx
                                            .send(TransportEvent::NewConnection {
                                                connection_id: conn_id,
                                                remote_addr: from_addr,
                                                remote_public_key: remote_pub,
                                            })
                                            .await;
                                        connections.insert(from_addr, conn);
                                    },
                                    Err(e) => {
                                        aptos_logger::warn!(
                                            "Failed inbound handshake from {}: {}",
                                            from_addr,
                                            e
                                        );
                                    },
                                }
                            },
                            _ => {
                                // Unknown packet from unknown source; ignore
                            },
                        }
                    }
                },
                Ok(Err(e)) => {
                    aptos_logger::error!("Socket recv error: {}", e);
                    return Err(QnucError::Io(e));
                },
                Err(_) => {
                    // Timeout - run maintenance
                    self.run_maintenance().await;
                },
            }
        }
    }

    /// Run periodic maintenance: ACKs, retransmissions, timeouts.
    async fn run_maintenance(&self) {
        let mut connections = self.connections.lock().await;
        let mut to_remove = Vec::new();

        for (addr, conn) in connections.iter_mut() {
            if conn.is_timed_out() {
                let _ = conn.close().await;
                to_remove.push(*addr);
                continue;
            }

            if conn.state() == ConnectionState::Established {
                let _ = conn.flush_acks().await;
                let _ = conn.retransmit().await;
            }
        }

        for addr in to_remove {
            connections.remove(&addr);
        }
    }

    /// Close all connections and shut down.
    pub async fn shutdown(&self) -> Result<()> {
        let mut connections = self.connections.lock().await;
        for (_, conn) in connections.iter_mut() {
            let _ = conn.close().await;
        }
        connections.clear();
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
    async fn test_transport_bind() {
        let (priv_key, _pub_key) = make_keypair([1u8; 32]);
        let transport = QnucTransport::bind(
            "127.0.0.1:0".parse().unwrap(),
            priv_key,
            ConnectionConfig::default(),
        )
        .await
        .unwrap();

        let addr = transport.local_addr().unwrap();
        assert_ne!(addr.port(), 0);
    }
}
