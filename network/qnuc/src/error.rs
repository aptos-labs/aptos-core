// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum QnucError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Noise handshake failed: {0}")]
    NoiseHandshake(String),

    #[error("Noise encryption error: {0}")]
    NoiseEncrypt(String),

    #[error("Noise decryption error: {0}")]
    NoiseDecrypt(String),

    #[error("Connection timed out")]
    ConnectionTimeout,

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Invalid packet: {0}")]
    InvalidPacket(String),

    #[error("Stream not found: {0}")]
    StreamNotFound(u64),

    #[error("Payload too large: {size} exceeds max {max}")]
    PayloadTooLarge { size: usize, max: usize },

    #[error("Max retransmissions exceeded for packet {seq}")]
    MaxRetransmissions { seq: u64 },

    #[error("Address resolution failed: {0}")]
    AddressResolution(String),
}

pub type Result<T> = std::result::Result<T, QnucError>;
