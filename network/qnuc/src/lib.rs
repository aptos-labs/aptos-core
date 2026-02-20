// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! A QUIC-like reliable, ordered UDP transport layer for Aptos node-to-node communication.
//!
//! This crate provides:
//! - **UDP transport**: Raw datagram send/receive with MTU-aware fragmentation
//! - **Reliability**: ACK-based retransmission with exponential backoff and congestion control
//! - **Ordering**: Sequence-numbered packets with a reorder buffer
//! - **Noise encryption**: Noise IK handshake for mutual authentication and key exchange
//! - **Stream multiplexing**: Multiple logical streams over a single UDP connection
//!
//! The design is inspired by QUIC but tailored for Aptos's validator-to-validator
//! and validator-to-fullnode communication patterns.

pub mod adapter;
pub mod connection;
pub mod crypto;
pub mod error;
pub mod netcore_transport;
pub mod packet;
pub mod reliability;
pub mod stream;
pub mod transport;

pub use adapter::QnucSocket;
pub use connection::Connection;
pub use error::QnucError;
pub use netcore_transport::QnucTransportLayer;
pub use transport::QnucTransport;
