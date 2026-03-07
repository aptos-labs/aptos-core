# Phase 2: QUIC Transport Implementation Guide

**Timeline**: 3-6 months  
**Focus**: Replace TCP+Noise with QUIC for improved latency and throughput  
**Priority**: High impact on latency (0-RTT connections, no head-of-line blocking)

---

## Table of Contents

1. [Overview and Rationale](#1-overview-and-rationale)
2. [QUIC Benefits for Aptos](#2-quic-benefits-for-aptos)
3. [Architecture Design](#3-architecture-design)
4. [Implementation Plan](#4-implementation-plan)
5. [Migration Strategy](#5-migration-strategy)
6. [Testing and Validation](#6-testing-and-validation)
7. [Rollout Plan](#7-rollout-plan)

---

## 1. Overview and Rationale

### Current Transport Stack

```
┌─────────────────────────────────────────┐
│            Application Data             │
├─────────────────────────────────────────┤
│     BCS Serialization (NetworkMessage)  │
├─────────────────────────────────────────┤
│     Length-Delimited Framing (4 bytes)  │
├─────────────────────────────────────────┤
│     Noise IK Encryption (ChaCha20-Poly) │
├─────────────────────────────────────────┤
│              TCP Transport              │
├─────────────────────────────────────────┤
│                   IP                    │
└─────────────────────────────────────────┘
```

### Proposed QUIC Stack

```
┌─────────────────────────────────────────┐
│            Application Data             │
├─────────────────────────────────────────┤
│     BCS Serialization (NetworkMessage)  │
├─────────────────────────────────────────┤
│         QUIC Streams (multiplexed)      │
├─────────────────────────────────────────┤
│     QUIC Transport (TLS 1.3 built-in)   │
├─────────────────────────────────────────┤
│                   UDP                   │
├─────────────────────────────────────────┤
│                   IP                    │
└─────────────────────────────────────────┘
```

### Why QUIC?

| Feature | TCP + Noise | QUIC |
|---------|-------------|------|
| Connection establishment | 2-3 RTT (TCP + Noise + Handshake) | 1 RTT (0-RTT for resumption) |
| Head-of-line blocking | Yes (single stream) | No (independent streams) |
| Encryption | Noise IK (custom) | TLS 1.3 (standard) |
| Multiplexing | Application layer | Native |
| Connection migration | No | Yes (IP change tolerance) |
| Congestion control | TCP (kernel) | Pluggable (userspace) |

---

## 2. QUIC Benefits for Aptos

### 2.1 Latency Improvements

**Connection Establishment**:
```
Current (TCP + Noise):
  Client                          Server
    │                                │
    │──── SYN ──────────────────────►│  RTT 1
    │◄─── SYN-ACK ───────────────────│
    │──── ACK ──────────────────────►│
    │                                │
    │──── Noise IK Init ────────────►│  RTT 2
    │◄─── Noise IK Response ─────────│
    │                                │
    │──── Handshake Msg ────────────►│  RTT 3
    │◄─── Handshake Msg ─────────────│
    │                                │
    Total: 3 RTT (~150-900ms globally)

QUIC (1-RTT):
  Client                          Server
    │                                │
    │──── Initial + Crypto ─────────►│  RTT 1
    │◄─── Initial + Crypto + Data ───│
    │                                │
    Total: 1 RTT (~50-300ms globally)

QUIC (0-RTT resumption):
  Client                          Server
    │                                │
    │──── 0-RTT Data ───────────────►│  0 RTT!
    │◄─── Response ──────────────────│
    │                                │
    Total: 0 RTT for first message
```

**Expected Improvement**: 66% reduction in connection setup time

### 2.2 Head-of-Line Blocking Elimination

**Current Problem**:
```
Single TCP Stream:
  [Consensus Vote][State Sync Chunk 1][State Sync Chunk 2]...
  
  If Chunk 1 packet is lost:
  [Consensus Vote] ← BLOCKED waiting for retransmit
                    [Chunk 1 RETRANSMIT][Chunk 2]...
```

**QUIC Solution**:
```
Multiple QUIC Streams:
  Stream 1 (Critical):  [Consensus Vote] ← Delivered immediately
  Stream 2 (Normal):    [Chunk 1]...[Chunk 1 RETRANSMIT][Chunk 2]...
  
  Packet loss on Stream 2 doesn't block Stream 1
```

### 2.3 Connection Migration

For validators with dynamic IPs or during network transitions:
```
Before IP Change:
  Validator A (IP: 1.2.3.4) ←──QUIC──► Validator B

After IP Change (seamless):
  Validator A (IP: 5.6.7.8) ←──QUIC──► Validator B
  (Connection maintained via Connection ID)
```

---

## 3. Architecture Design

### 3.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           NETWORK FRAMEWORK                                  │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                          PeerManager                                    │ │
│  │                    (unchanged - transport agnostic)                     │ │
│  └───────────────────────────────┬────────────────────────────────────────┘ │
│                                  │                                           │
│  ┌───────────────────────────────▼────────────────────────────────────────┐ │
│  │                      Transport Abstraction                              │ │
│  │                                                                         │ │
│  │   trait Transport {                                                     │ │
│  │       fn dial(&self, peer_id, addr) -> Connection;                      │ │
│  │       fn listen_on(&self, addr) -> Listener;                           │ │
│  │   }                                                                     │ │
│  └───────────────────────────────┬────────────────────────────────────────┘ │
│                                  │                                           │
│         ┌────────────────────────┼────────────────────────────┐             │
│         │                        │                            │             │
│         ▼                        ▼                            ▼             │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────────────┐ │
│  │ AptosNetTransport│    │  QuicTransport  │    │   HybridTransport      │ │
│  │  (TCP + Noise)  │    │    (NEW)        │    │ (TCP + QUIC fallback)  │ │
│  │                 │    │                 │    │                        │ │
│  │ - Noise IK      │    │ - quinn/quiche  │    │ - Try QUIC first       │ │
│  │ - TCP           │    │ - TLS 1.3       │    │ - Fallback to TCP      │ │
│  │ - Manual stream │    │ - Native streams│    │ - Feature flagged      │ │
│  └─────────────────┘    └─────────────────┘    └─────────────────────────┘ │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 QUIC Stream Mapping

Map message priorities to QUIC streams:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         QUIC Connection                                      │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │  Stream 0 (Bidirectional) - Control Channel                             ││
│  │  • Connection keepalive                                                 ││
│  │  • Protocol negotiation                                                 ││
│  │  • Health checks                                                        ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │  Streams 1-99 (Bidirectional) - Critical Priority                       ││
│  │  • Consensus votes                                                      ││
│  │  • Timeout certificates                                                 ││
│  │  • Block proposals                                                      ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │  Streams 100-999 (Bidirectional) - High Priority                        ││
│  │  • Mempool transactions                                                 ││
│  │  • RPC requests/responses                                               ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │  Streams 1000+ (Unidirectional) - Normal/Low Priority                   ││
│  │  • State sync chunks                                                    ││
│  │  • Large data transfers                                                 ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.3 Authentication Design

**Option A: TLS 1.3 with Custom Certificate Verification**
```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      QUIC + TLS 1.3 Authentication                          │
│                                                                              │
│  1. Generate self-signed certificate from validator's x25519 key            │
│                                                                              │
│  2. Custom certificate verifier:                                            │
│     • Extract public key from peer certificate                              │
│     • Check if public key is in trusted_peers set                          │
│     • Verify certificate signature                                          │
│                                                                              │
│  3. Derive PeerId from verified public key                                  │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Option B: QUIC + Post-Handshake Noise Authentication**
```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      QUIC + Noise Hybrid                                     │
│                                                                              │
│  1. QUIC connection with anonymous TLS (encryption only)                    │
│                                                                              │
│  2. First message on Stream 0: Noise IK handshake                          │
│     • Authenticates both parties                                            │
│     • Establishes shared secret (unused, TLS already provides)              │
│                                                                              │
│  3. After Noise handshake: Normal message exchange                          │
│                                                                              │
│  Pros: Reuses existing Noise authentication code                            │
│  Cons: Extra RTT, complexity                                                │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Recommendation**: Option A (native TLS 1.3) for simplicity and performance.

---

## 4. Implementation Plan

### 4.1 Dependencies

Add to `network/framework/Cargo.toml`:

```toml
[dependencies]
# Option 1: quinn (Rust-native, async, well-maintained)
quinn = "0.11"
rustls = { version = "0.23", features = ["ring"] }
rcgen = "0.12"  # For certificate generation

# Option 2: quiche (C library, used by Cloudflare)
# quiche = "0.20"
```

**Recommendation**: Use `quinn` for better Rust ecosystem integration.

### 4.2 Core Types

**File**: `network/framework/src/transport/quic/mod.rs` (new)

```rust
//! QUIC transport implementation for AptosNet.
//!
//! This module provides a QUIC-based transport that offers:
//! - 0-RTT connection establishment (for resumed connections)
//! - Native stream multiplexing (no head-of-line blocking)
//! - Built-in TLS 1.3 encryption
//! - Connection migration support

use quinn::{
    ClientConfig, Connection, Endpoint, RecvStream, SendStream, ServerConfig,
    TransportConfig, VarInt,
};
use rustls::{Certificate, PrivateKey};
use std::{net::SocketAddr, sync::Arc, time::Duration};

mod config;
mod connection;
mod error;
mod listener;
mod stream;
mod tls;

pub use config::QuicConfig;
pub use connection::QuicConnection;
pub use error::QuicError;
pub use listener::QuicListener;

/// Stream priority levels mapped to QUIC stream IDs
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamPriority {
    /// Control channel (stream 0)
    Control,
    /// Critical priority (streams 1-99): consensus
    Critical,
    /// High priority (streams 100-999): mempool, RPC
    High,
    /// Normal priority (streams 1000+): state sync
    Normal,
}

impl StreamPriority {
    /// Get the stream ID range for this priority
    pub fn stream_id_range(&self) -> std::ops::Range<u64> {
        match self {
            StreamPriority::Control => 0..1,
            StreamPriority::Critical => 1..100,
            StreamPriority::High => 100..1000,
            StreamPriority::Normal => 1000..u64::MAX,
        }
    }

    /// Get the QUIC priority value (lower = higher priority)
    pub fn quic_priority(&self) -> i32 {
        match self {
            StreamPriority::Control => 0,
            StreamPriority::Critical => 1,
            StreamPriority::High => 2,
            StreamPriority::Normal => 3,
        }
    }
}
```

### 4.3 QUIC Configuration

**File**: `network/framework/src/transport/quic/config.rs`

```rust
use quinn::{TransportConfig, VarInt};
use std::time::Duration;

/// Configuration for QUIC transport
#[derive(Clone, Debug)]
pub struct QuicConfig {
    /// Maximum number of bidirectional streams
    pub max_bi_streams: u32,
    /// Maximum number of unidirectional streams
    pub max_uni_streams: u32,
    /// Initial RTT estimate (for congestion control)
    pub initial_rtt: Duration,
    /// Maximum idle timeout
    pub max_idle_timeout: Duration,
    /// Keep-alive interval
    pub keep_alive_interval: Duration,
    /// Enable 0-RTT
    pub enable_0rtt: bool,
    /// Maximum UDP payload size
    pub max_udp_payload_size: u16,
    /// Congestion controller (cubic, bbr, etc.)
    pub congestion_controller: CongestionController,
}

#[derive(Clone, Copy, Debug)]
pub enum CongestionController {
    Cubic,
    NewReno,
    Bbr,
}

impl Default for QuicConfig {
    fn default() -> Self {
        Self {
            max_bi_streams: 1000,
            max_uni_streams: 1000,
            initial_rtt: Duration::from_millis(100),
            max_idle_timeout: Duration::from_secs(30),
            keep_alive_interval: Duration::from_secs(10),
            enable_0rtt: true,
            max_udp_payload_size: 1350,  // Safe for most networks
            congestion_controller: CongestionController::Bbr,
        }
    }
}

impl QuicConfig {
    /// Optimized config for validator networks (low latency)
    pub fn validator() -> Self {
        Self {
            initial_rtt: Duration::from_millis(50),
            max_idle_timeout: Duration::from_secs(60),
            keep_alive_interval: Duration::from_secs(5),
            congestion_controller: CongestionController::Bbr,
            ..Default::default()
        }
    }

    /// Optimized config for full node networks (high throughput)
    pub fn fullnode() -> Self {
        Self {
            max_bi_streams: 100,
            max_uni_streams: 10000,  // More streams for state sync
            max_idle_timeout: Duration::from_secs(120),
            congestion_controller: CongestionController::Cubic,
            ..Default::default()
        }
    }

    /// Convert to quinn TransportConfig
    pub fn to_transport_config(&self) -> TransportConfig {
        let mut config = TransportConfig::default();
        
        config.max_concurrent_bidi_streams(VarInt::from_u32(self.max_bi_streams));
        config.max_concurrent_uni_streams(VarInt::from_u32(self.max_uni_streams));
        config.initial_rtt(self.initial_rtt);
        config.max_idle_timeout(Some(self.max_idle_timeout.try_into().unwrap()));
        config.keep_alive_interval(Some(self.keep_alive_interval));
        
        // Set congestion controller
        match self.congestion_controller {
            CongestionController::Cubic => {
                config.congestion_controller_factory(Arc::new(quinn::congestion::CubicConfig::default()));
            }
            CongestionController::NewReno => {
                config.congestion_controller_factory(Arc::new(quinn::congestion::NewRenoConfig::default()));
            }
            CongestionController::Bbr => {
                config.congestion_controller_factory(Arc::new(quinn::congestion::BbrConfig::default()));
            }
        }
        
        config
    }
}
```

### 4.4 TLS Configuration with Custom Verification

**File**: `network/framework/src/transport/quic/tls.rs`

```rust
use crate::application::storage::PeersAndMetadata;
use aptos_crypto::x25519;
use aptos_types::PeerId;
use rcgen::{Certificate, CertificateParams, DistinguishedName, KeyPair, PKCS_ED25519};
use rustls::{
    client::{ServerCertVerified, ServerCertVerifier},
    server::{ClientCertVerified, ClientCertVerifier},
    Certificate as RustlsCert, DistinguishedNames, PrivateKey, RootCertStore,
};
use std::sync::Arc;

/// Generate a self-signed certificate from the node's identity key
pub fn generate_certificate(
    identity_key: &x25519::PrivateKey,
    peer_id: PeerId,
) -> Result<(Vec<RustlsCert>, PrivateKey), QuicError> {
    // Convert x25519 key to ed25519 for certificate signing
    // Note: In practice, you might use a separate signing key
    let key_pair = KeyPair::generate(&PKCS_ED25519)?;
    
    let mut params = CertificateParams::default();
    params.distinguished_name = DistinguishedName::new();
    params.distinguished_name.push(
        rcgen::DnType::CommonName,
        format!("aptos-node-{}", peer_id),
    );
    
    // Include the x25519 public key in a custom extension
    // This allows peers to extract the network identity
    params.custom_extensions.push(rcgen::CustomExtension::from_oid_content(
        &[1, 3, 6, 1, 4, 1, 99999, 1],  // Custom OID for Aptos
        identity_key.public_key().to_bytes().to_vec(),
    ));
    
    // Self-sign the certificate
    let cert = Certificate::from_params(params)?;
    let cert_der = cert.serialize_der()?;
    let key_der = cert.serialize_private_key_der();
    
    Ok((
        vec![RustlsCert(cert_der)],
        PrivateKey(key_der),
    ))
}

/// Custom certificate verifier for mutual authentication
pub struct AptosCertVerifier {
    peers_and_metadata: Arc<PeersAndMetadata>,
    network_id: NetworkId,
}

impl AptosCertVerifier {
    pub fn new(peers_and_metadata: Arc<PeersAndMetadata>, network_id: NetworkId) -> Self {
        Self {
            peers_and_metadata,
            network_id,
        }
    }

    /// Extract the x25519 public key from a certificate
    fn extract_public_key(&self, cert: &RustlsCert) -> Result<x25519::PublicKey, QuicError> {
        use x509_parser::prelude::*;
        
        let (_, cert) = X509Certificate::from_der(&cert.0)
            .map_err(|_| QuicError::InvalidCertificate)?;
        
        // Find our custom extension containing the x25519 public key
        for ext in cert.extensions() {
            if ext.oid.to_string() == "1.3.6.1.4.1.99999.1" {
                let key_bytes: [u8; 32] = ext.value
                    .try_into()
                    .map_err(|_| QuicError::InvalidCertificate)?;
                return x25519::PublicKey::try_from(&key_bytes)
                    .map_err(|_| QuicError::InvalidCertificate);
            }
        }
        
        Err(QuicError::MissingPublicKey)
    }

    /// Verify that the public key belongs to a trusted peer
    fn verify_trusted(&self, public_key: &x25519::PublicKey) -> Result<PeerId, QuicError> {
        let trusted_peers = self.peers_and_metadata
            .get_trusted_peers(&self.network_id)
            .map_err(|_| QuicError::TrustCheckFailed)?;
        
        for (peer_id, peer) in trusted_peers {
            if peer.keys.contains(public_key) {
                return Ok(peer_id);
            }
        }
        
        Err(QuicError::UntrustedPeer)
    }
}

impl ServerCertVerifier for AptosCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &RustlsCert,
        _intermediates: &[RustlsCert],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let public_key = self.extract_public_key(end_entity)
            .map_err(|_| rustls::Error::InvalidCertificateData(
                "Failed to extract public key".into()
            ))?;
        
        self.verify_trusted(&public_key)
            .map_err(|_| rustls::Error::InvalidCertificateData(
                "Peer not in trusted set".into()
            ))?;
        
        Ok(ServerCertVerified::assertion())
    }
}

impl ClientCertVerifier for AptosCertVerifier {
    fn client_auth_root_subjects(&self) -> Option<DistinguishedNames> {
        Some(vec![])  // Accept any distinguished name
    }

    fn verify_client_cert(
        &self,
        end_entity: &RustlsCert,
        _intermediates: &[RustlsCert],
        _now: std::time::SystemTime,
    ) -> Result<ClientCertVerified, rustls::Error> {
        let public_key = self.extract_public_key(end_entity)
            .map_err(|_| rustls::Error::InvalidCertificateData(
                "Failed to extract public key".into()
            ))?;
        
        self.verify_trusted(&public_key)
            .map_err(|_| rustls::Error::InvalidCertificateData(
                "Peer not in trusted set".into()
            ))?;
        
        Ok(ClientCertVerified::assertion())
    }
}
```

### 4.5 QUIC Transport Implementation

**File**: `network/framework/src/transport/quic/connection.rs`

```rust
use super::{QuicConfig, QuicError, StreamPriority};
use crate::protocols::wire::messaging::v1::{MultiplexMessage, NetworkMessage};
use crate::transport::{Connection, ConnectionMetadata, TSocket};
use bytes::Bytes;
use futures::{Sink, Stream};
use quinn::{Connection as QuinnConnection, RecvStream, SendStream};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// A QUIC-based connection to a remote peer
pub struct QuicConnection {
    /// Underlying quinn connection
    inner: QuinnConnection,
    /// Connection metadata
    pub metadata: ConnectionMetadata,
    /// Stream pool for reusing streams
    stream_pool: Arc<Mutex<StreamPool>>,
    /// Configuration
    config: QuicConfig,
}

impl QuicConnection {
    pub fn new(
        inner: QuinnConnection,
        metadata: ConnectionMetadata,
        config: QuicConfig,
    ) -> Self {
        Self {
            inner,
            metadata,
            stream_pool: Arc::new(Mutex::new(StreamPool::new())),
            config,
        }
    }

    /// Open a new stream with the given priority
    pub async fn open_stream(
        &self,
        priority: StreamPriority,
    ) -> Result<(SendStream, RecvStream), QuicError> {
        let (send, recv) = self.inner.open_bi().await?;
        
        // Set stream priority
        send.set_priority(priority.quic_priority())?;
        
        Ok((send, recv))
    }

    /// Get a stream from the pool or open a new one
    pub async fn get_stream(
        &self,
        priority: StreamPriority,
    ) -> Result<PooledStream, QuicError> {
        let mut pool = self.stream_pool.lock().await;
        
        if let Some(stream) = pool.take(priority) {
            Ok(stream)
        } else {
            let (send, recv) = self.open_stream(priority).await?;
            Ok(PooledStream {
                send,
                recv,
                priority,
                pool: self.stream_pool.clone(),
            })
        }
    }

    /// Accept an incoming stream
    pub async fn accept_stream(&self) -> Result<(SendStream, RecvStream), QuicError> {
        let (send, recv) = self.inner.accept_bi().await?;
        Ok((send, recv))
    }

    /// Send a message on an appropriate stream
    pub async fn send_message(
        &self,
        message: NetworkMessage,
        priority: StreamPriority,
    ) -> Result<(), QuicError> {
        let mut stream = self.get_stream(priority).await?;
        
        // Serialize and send
        let bytes = bcs::to_bytes(&MultiplexMessage::Message(message))
            .map_err(|e| QuicError::Serialization(e))?;
        
        // Write length prefix
        let len = (bytes.len() as u32).to_be_bytes();
        stream.send.write_all(&len).await?;
        stream.send.write_all(&bytes).await?;
        
        // Return stream to pool
        stream.return_to_pool().await;
        
        Ok(())
    }

    /// Receive a message from any stream
    pub async fn recv_message(&self) -> Result<NetworkMessage, QuicError> {
        let (mut send, mut recv) = self.accept_stream().await?;
        
        // Read length prefix
        let mut len_bytes = [0u8; 4];
        recv.read_exact(&mut len_bytes).await?;
        let len = u32::from_be_bytes(len_bytes) as usize;
        
        // Read message
        let mut buf = vec![0u8; len];
        recv.read_exact(&mut buf).await?;
        
        // Deserialize
        let message: MultiplexMessage = bcs::from_bytes(&buf)
            .map_err(|e| QuicError::Deserialization(e))?;
        
        match message {
            MultiplexMessage::Message(msg) => Ok(msg),
            MultiplexMessage::Stream(_) => {
                // Handle streaming messages
                todo!("Implement stream message handling")
            }
        }
    }

    /// Check if the connection is still alive
    pub fn is_alive(&self) -> bool {
        !self.inner.close_reason().is_some()
    }

    /// Close the connection gracefully
    pub async fn close(&self) {
        self.inner.close(0u32.into(), b"closing");
    }
}

/// A stream that can be returned to a pool when done
pub struct PooledStream {
    send: SendStream,
    recv: RecvStream,
    priority: StreamPriority,
    pool: Arc<Mutex<StreamPool>>,
}

impl PooledStream {
    async fn return_to_pool(self) {
        let mut pool = self.pool.lock().await;
        pool.put(self.priority, self.send, self.recv);
    }
}

/// Pool of reusable streams
struct StreamPool {
    streams: HashMap<StreamPriority, Vec<(SendStream, RecvStream)>>,
    max_per_priority: usize,
}

impl StreamPool {
    fn new() -> Self {
        Self {
            streams: HashMap::new(),
            max_per_priority: 10,
        }
    }

    fn take(&mut self, priority: StreamPriority) -> Option<PooledStream> {
        self.streams
            .get_mut(&priority)
            .and_then(|v| v.pop())
            .map(|(send, recv)| PooledStream {
                send,
                recv,
                priority,
                pool: Arc::new(Mutex::new(StreamPool::new())),  // Placeholder
            })
    }

    fn put(&mut self, priority: StreamPriority, send: SendStream, recv: RecvStream) {
        let streams = self.streams.entry(priority).or_insert_with(Vec::new);
        if streams.len() < self.max_per_priority {
            streams.push((send, recv));
        }
        // Otherwise, let the stream be dropped
    }
}
```

### 4.6 QUIC Transport Wrapper

**File**: `network/framework/src/transport/quic/transport.rs`

```rust
use super::{QuicConfig, QuicConnection, QuicError, QuicListener};
use crate::transport::{Connection, ConnectionMetadata, TSocket};
use aptos_config::network_id::NetworkContext;
use aptos_crypto::x25519;
use aptos_netcore::transport::{ConnectionOrigin, Transport};
use aptos_types::network_address::NetworkAddress;
use aptos_types::PeerId;
use quinn::{ClientConfig, Endpoint, ServerConfig};
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

/// QUIC-based transport for AptosNet
pub struct QuicTransport {
    /// QUIC endpoint (handles both client and server)
    endpoint: Endpoint,
    /// Network context
    network_context: NetworkContext,
    /// Configuration
    config: QuicConfig,
    /// Our identity public key
    identity_pubkey: x25519::PublicKey,
}

impl QuicTransport {
    pub async fn new(
        network_context: NetworkContext,
        identity_key: x25519::PrivateKey,
        listen_addr: SocketAddr,
        config: QuicConfig,
        peers_and_metadata: Arc<PeersAndMetadata>,
    ) -> Result<Self, QuicError> {
        let identity_pubkey = identity_key.public_key();
        
        // Generate TLS certificate
        let (certs, key) = tls::generate_certificate(
            &identity_key,
            network_context.peer_id(),
        )?;
        
        // Create TLS configs with custom verification
        let verifier = Arc::new(tls::AptosCertVerifier::new(
            peers_and_metadata,
            network_context.network_id(),
        ));
        
        let server_config = ServerConfig::with_crypto(Arc::new(
            rustls::ServerConfig::builder()
                .with_safe_defaults()
                .with_client_cert_verifier(verifier.clone())
                .with_single_cert(certs.clone(), key.clone())?,
        ));
        
        let client_config = ClientConfig::new(Arc::new(
            rustls::ClientConfig::builder()
                .with_safe_defaults()
                .with_custom_certificate_verifier(verifier)
                .with_single_cert(certs, key)?,
        ));
        
        // Apply transport config
        let transport_config = config.to_transport_config();
        let mut server_config = server_config;
        server_config.transport_config(Arc::new(transport_config.clone()));
        
        // Create endpoint
        let mut endpoint = Endpoint::server(server_config, listen_addr)?;
        endpoint.set_default_client_config(client_config);
        
        Ok(Self {
            endpoint,
            network_context,
            config,
            identity_pubkey,
        })
    }

    /// Dial a remote peer
    pub async fn dial(
        &self,
        peer_id: PeerId,
        addr: NetworkAddress,
    ) -> Result<QuicConnection, QuicError> {
        let socket_addr = addr.to_socket_addr()
            .ok_or(QuicError::InvalidAddress)?;
        
        let connection = self.endpoint
            .connect(socket_addr, &peer_id.to_string())?
            .await?;
        
        let metadata = ConnectionMetadata::new(
            peer_id,
            ConnectionId::new(),
            addr,
            ConnectionOrigin::Outbound,
            MessagingProtocolVersion::V1,
            ProtocolIdSet::all_known(),
            PeerRole::Unknown,  // Will be determined after handshake
        );
        
        Ok(QuicConnection::new(connection, metadata, self.config.clone()))
    }

    /// Listen for incoming connections
    pub async fn listen(&self) -> QuicListener {
        QuicListener::new(self.endpoint.clone(), self.config.clone())
    }
}

impl Transport for QuicTransport {
    type Output = QuicConnection;
    type Error = QuicError;
    type Listener = QuicListener;
    type Inbound = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;
    type Outbound = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn dial(&self, peer_id: PeerId, addr: NetworkAddress) -> Result<Self::Outbound, Self::Error> {
        let this = self.clone();
        Ok(Box::pin(async move {
            this.dial(peer_id, addr).await
        }))
    }

    fn listen_on(&self, addr: NetworkAddress) -> Result<(Self::Listener, NetworkAddress), Self::Error> {
        let listener = self.listen();
        let listen_addr = self.endpoint.local_addr()?;
        Ok((listener, NetworkAddress::from(listen_addr)))
    }
}
```

### 4.7 Integration with Peer Actor

**File**: `network/framework/src/peer/mod.rs` (modifications)

```rust
// Add support for QUIC connections in the Peer actor

impl<TSocket> Peer<TSocket>
where
    TSocket: AsyncRead + AsyncWrite + Send + 'static,
{
    // ... existing code ...

    /// Create a Peer from a QUIC connection
    #[cfg(feature = "quic")]
    pub fn from_quic(
        network_context: NetworkContext,
        executor: Handle,
        time_service: TimeService,
        quic_connection: QuicConnection,
        connection_notifs_tx: Sender<TransportNotification<QuicConnection>>,
        peer_reqs_rx: Receiver<ProtocolId, PeerRequest>,
        upstream_handlers: Arc<HashMap<ProtocolId, Sender<ReceivedMessage>>>,
        inbound_rpc_timeout: Duration,
        max_concurrent_inbound_rpcs: u32,
        max_concurrent_outbound_rpcs: u32,
    ) -> Self {
        // QUIC-specific initialization
        // Key difference: No need for manual framing or encryption
        // Streams handle multiplexing natively
        
        Self {
            network_context,
            executor,
            time_service,
            connection_metadata: quic_connection.metadata.clone(),
            connection: ConnectionType::Quic(quic_connection),
            // ... rest of initialization
        }
    }
}

/// Enum to handle both TCP and QUIC connections
#[cfg(feature = "quic")]
enum ConnectionType<TSocket> {
    Tcp(TSocket),
    Quic(QuicConnection),
}
```

---

## 5. Migration Strategy

### 5.1 Feature Flags

```toml
# Cargo.toml
[features]
default = []
quic = ["quinn", "rcgen"]
quic-only = ["quic"]  # Disable TCP entirely
```

### 5.2 Hybrid Transport

Support both TCP and QUIC simultaneously during migration:

```rust
/// Hybrid transport that tries QUIC first, falls back to TCP
pub struct HybridTransport {
    quic: Option<QuicTransport>,
    tcp: AptosNetTransport,
    prefer_quic: bool,
}

impl HybridTransport {
    pub async fn dial(
        &self,
        peer_id: PeerId,
        addr: NetworkAddress,
    ) -> Result<Connection, TransportError> {
        // Try QUIC first if available and preferred
        if self.prefer_quic {
            if let Some(ref quic) = self.quic {
                match quic.dial(peer_id, addr.clone()).await {
                    Ok(conn) => return Ok(Connection::Quic(conn)),
                    Err(e) => {
                        warn!("QUIC dial failed, falling back to TCP: {:?}", e);
                    }
                }
            }
        }
        
        // Fall back to TCP
        let conn = self.tcp.dial(peer_id, addr).await?;
        Ok(Connection::Tcp(conn))
    }
}
```

### 5.3 Network Address Format

Extend network addresses to indicate QUIC support:

```
# TCP address (current)
/ip4/1.2.3.4/tcp/6180/noise-ik/<pubkey>/handshake/0

# QUIC address (new)
/ip4/1.2.3.4/udp/6180/quic/<pubkey>

# Hybrid address (supports both)
/ip4/1.2.3.4/tcp/6180/noise-ik/<pubkey>/handshake/0
/ip4/1.2.3.4/udp/6180/quic/<pubkey>
```

---

## 6. Testing and Validation

### 6.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_quic_connection_establishment() {
        // Test basic connection
    }

    #[tokio::test]
    async fn test_quic_stream_multiplexing() {
        // Test multiple concurrent streams
    }

    #[tokio::test]
    async fn test_quic_priority_ordering() {
        // Test that high-priority messages aren't blocked
    }

    #[tokio::test]
    async fn test_quic_0rtt_resumption() {
        // Test 0-RTT connection resumption
    }

    #[tokio::test]
    async fn test_quic_connection_migration() {
        // Test connection survives IP change
    }
}
```

### 6.2 Integration Tests

```rust
#[tokio::test]
async fn test_consensus_over_quic() {
    // Full consensus round with QUIC transport
}

#[tokio::test]
async fn test_state_sync_over_quic() {
    // Large state sync transfer
}

#[tokio::test]
async fn test_mixed_tcp_quic_network() {
    // Nodes with different transports can communicate
}
```

### 6.3 Performance Benchmarks

```rust
#[bench]
fn bench_connection_establishment_tcp() {
    // Measure TCP+Noise connection time
}

#[bench]
fn bench_connection_establishment_quic() {
    // Measure QUIC connection time
}

#[bench]
fn bench_throughput_under_packet_loss() {
    // Compare head-of-line blocking impact
}
```

### 6.4 Chaos Testing

- **Packet loss**: Verify QUIC handles 1-5% packet loss gracefully
- **Latency injection**: Test with 100-500ms added latency
- **Connection interruption**: Verify reconnection works
- **IP change**: Test connection migration

---

## 7. Rollout Plan

### Phase 2a: Development (Weeks 1-6)

| Week | Milestone |
|------|-----------|
| 1-2 | Core QUIC types, TLS integration |
| 3-4 | QuicTransport implementation |
| 5-6 | Peer actor integration, unit tests |

### Phase 2b: Testing (Weeks 7-10)

| Week | Milestone |
|------|-----------|
| 7 | Integration tests, benchmark suite |
| 8-9 | Testnet deployment (feature-flagged) |
| 10 | Performance validation, bug fixes |

### Phase 2c: Rollout (Weeks 11-16)

| Week | Milestone |
|------|-----------|
| 11-12 | Devnet: 100% QUIC |
| 13-14 | Testnet: Gradual rollout (10% → 50% → 100%) |
| 15-16 | Mainnet: Gradual rollout with monitoring |

### Rollback Criteria

- Connection failure rate > 5%
- Consensus latency regression > 20%
- Any consensus safety issues

### Success Metrics

| Metric | Target |
|--------|--------|
| Connection establishment time | < 150ms (p99) |
| Consensus vote latency | < 100ms (p99) |
| Connection success rate | > 99.5% |
| 0-RTT resumption rate | > 90% |

---

## Appendix: Key Files to Create/Modify

### New Files

| File | Purpose |
|------|---------|
| `transport/quic/mod.rs` | Module entry point |
| `transport/quic/config.rs` | QUIC configuration |
| `transport/quic/connection.rs` | Connection wrapper |
| `transport/quic/listener.rs` | Connection listener |
| `transport/quic/tls.rs` | TLS/certificate handling |
| `transport/quic/error.rs` | Error types |
| `transport/quic/stream.rs` | Stream management |

### Modified Files

| File | Changes |
|------|---------|
| `Cargo.toml` | Add quinn, rustls dependencies |
| `transport/mod.rs` | Export QUIC module |
| `peer/mod.rs` | Support QUIC connections |
| `peer_manager/mod.rs` | Handle QUIC in connection management |
| `config/network_config.rs` | Add QUIC configuration options |

---

*Document Version: 1.0*  
*Last Updated: January 27, 2026*
