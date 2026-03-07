// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Wire-format packet definitions for the QUIC-like UDP transport.
//!
//! Packet layout (all multi-byte fields are big-endian):
//!
//! ```text
//! ┌─────────┬──────────┬──────────┬──────────┬───────────┬──────────┬─────────────────┐
//! │ version │ pkt_type │ conn_id  │ stream   │ seq_num   │ payload  │ (noise auth tag │
//! │  1 byte │  1 byte  │ 8 bytes  │ 8 bytes  │ 8 bytes   │ len 2B   │  if encrypted)  │
//! └─────────┴──────────┴──────────┴──────────┴───────────┴──────────┴─────────────────┘
//! ```

use crate::error::{QnucError, Result};
use bytes::{Buf, BufMut, Bytes, BytesMut};

pub const PROTOCOL_VERSION: u8 = 1;
pub const HEADER_SIZE: usize = 1 + 1 + 8 + 8 + 8 + 2; // 28 bytes

/// Safe MTU for UDP, accounting for IP (20) + UDP (8) headers.
/// Using 1200 bytes as QUIC does for initial packets.
pub const MAX_UDP_PAYLOAD: usize = 1200;

pub const MAX_PACKET_PAYLOAD: usize = MAX_UDP_PAYLOAD - HEADER_SIZE;

/// Maximum reassembled message size (16 MiB).
pub const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PacketType {
    /// Noise IK handshake initiation (client -> server).
    HandshakeInit = 0x01,
    /// Noise IK handshake response (server -> client).
    HandshakeResp = 0x02,
    /// Encrypted data packet carrying stream payload.
    Data = 0x03,
    /// Selective acknowledgement packet.
    Ack = 0x04,
    /// Connection close.
    Close = 0x05,
    /// Ping / keepalive.
    Ping = 0x06,
    /// Pong / keepalive response.
    Pong = 0x07,
}

impl TryFrom<u8> for PacketType {
    type Error = QnucError;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0x01 => Ok(Self::HandshakeInit),
            0x02 => Ok(Self::HandshakeResp),
            0x03 => Ok(Self::Data),
            0x04 => Ok(Self::Ack),
            0x05 => Ok(Self::Close),
            0x06 => Ok(Self::Ping),
            0x07 => Ok(Self::Pong),
            v => Err(QnucError::InvalidPacket(format!(
                "unknown packet type: 0x{:02x}",
                v
            ))),
        }
    }
}

/// Represents a parsed packet header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketHeader {
    pub version: u8,
    pub packet_type: PacketType,
    pub connection_id: u64,
    pub stream_id: u64,
    pub sequence_number: u64,
    pub payload_length: u16,
}

impl PacketHeader {
    pub fn new(
        packet_type: PacketType,
        connection_id: u64,
        stream_id: u64,
        sequence_number: u64,
        payload_length: u16,
    ) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            packet_type,
            connection_id,
            stream_id,
            sequence_number,
            payload_length,
        }
    }

    pub fn encode(&self, buf: &mut BytesMut) {
        buf.put_u8(self.version);
        buf.put_u8(self.packet_type as u8);
        buf.put_u64(self.connection_id);
        buf.put_u64(self.stream_id);
        buf.put_u64(self.sequence_number);
        buf.put_u16(self.payload_length);
    }

    pub fn decode(buf: &mut Bytes) -> Result<Self> {
        if buf.remaining() < HEADER_SIZE {
            return Err(QnucError::InvalidPacket(format!(
                "packet too small: {} < {}",
                buf.remaining(),
                HEADER_SIZE,
            )));
        }
        let version = buf.get_u8();
        if version != PROTOCOL_VERSION {
            return Err(QnucError::InvalidPacket(format!(
                "unsupported version: {}",
                version
            )));
        }
        let packet_type = PacketType::try_from(buf.get_u8())?;
        let connection_id = buf.get_u64();
        let stream_id = buf.get_u64();
        let sequence_number = buf.get_u64();
        let payload_length = buf.get_u16();

        Ok(Self {
            version,
            packet_type,
            connection_id,
            stream_id,
            sequence_number,
            payload_length,
        })
    }
}

/// A complete wire packet (header + payload).
#[derive(Debug, Clone)]
pub struct Packet {
    pub header: PacketHeader,
    pub payload: Bytes,
}

impl Packet {
    pub fn new(header: PacketHeader, payload: Bytes) -> Self {
        Self { header, payload }
    }

    pub fn encode(&self) -> BytesMut {
        let mut buf = BytesMut::with_capacity(HEADER_SIZE + self.payload.len());
        self.header.encode(&mut buf);
        buf.extend_from_slice(&self.payload);
        buf
    }

    pub fn decode(data: &[u8]) -> Result<Self> {
        let mut bytes = Bytes::copy_from_slice(data);
        let header = PacketHeader::decode(&mut bytes)?;
        let payload_len = header.payload_length as usize;
        if bytes.remaining() < payload_len {
            return Err(QnucError::InvalidPacket(format!(
                "payload truncated: have {} need {}",
                bytes.remaining(),
                payload_len,
            )));
        }
        let payload = bytes.split_to(payload_len);
        Ok(Self { header, payload })
    }
}

/// Selective ACK data: encodes a set of acknowledged sequence numbers.
#[derive(Debug, Clone)]
pub struct SelectiveAck {
    /// The cumulative ACK: all packets up to (and including) this seq are acknowledged.
    pub cumulative_ack: u64,
    /// Additional individual sequence numbers that have been received beyond the cumulative ack.
    pub selective_acks: Vec<u64>,
}

impl SelectiveAck {
    pub fn new(cumulative_ack: u64, selective_acks: Vec<u64>) -> Self {
        Self {
            cumulative_ack,
            selective_acks,
        }
    }

    pub fn encode(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(8 + 2 + self.selective_acks.len() * 8);
        buf.put_u64(self.cumulative_ack);
        buf.put_u16(self.selective_acks.len() as u16);
        for &seq in &self.selective_acks {
            buf.put_u64(seq);
        }
        buf.freeze()
    }

    pub fn decode(data: &[u8]) -> Result<Self> {
        let mut buf = Bytes::copy_from_slice(data);
        if buf.remaining() < 10 {
            return Err(QnucError::InvalidPacket(
                "ACK payload too small".to_string(),
            ));
        }
        let cumulative_ack = buf.get_u64();
        let count = buf.get_u16() as usize;
        if buf.remaining() < count * 8 {
            return Err(QnucError::InvalidPacket(
                "ACK selective list truncated".to_string(),
            ));
        }
        let mut selective_acks = Vec::with_capacity(count);
        for _ in 0..count {
            selective_acks.push(buf.get_u64());
        }
        Ok(Self {
            cumulative_ack,
            selective_acks,
        })
    }
}

/// Fragment header for messages that exceed MAX_PACKET_PAYLOAD.
/// Placed at the beginning of the data payload.
///
/// ```text
/// ┌──────────────┬─────────────┬──────────────┐
/// │ message_id   │ frag_index  │ total_frags  │
/// │ 8 bytes      │ 2 bytes     │ 2 bytes      │
/// └──────────────┴─────────────┴──────────────┘
/// ```
pub const FRAGMENT_HEADER_SIZE: usize = 8 + 2 + 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FragmentHeader {
    pub message_id: u64,
    pub fragment_index: u16,
    pub total_fragments: u16,
}

impl FragmentHeader {
    pub fn encode(&self, buf: &mut BytesMut) {
        buf.put_u64(self.message_id);
        buf.put_u16(self.fragment_index);
        buf.put_u16(self.total_fragments);
    }

    pub fn decode(buf: &mut Bytes) -> Result<Self> {
        if buf.remaining() < FRAGMENT_HEADER_SIZE {
            return Err(QnucError::InvalidPacket(
                "fragment header truncated".to_string(),
            ));
        }
        Ok(Self {
            message_id: buf.get_u64(),
            fragment_index: buf.get_u16(),
            total_fragments: buf.get_u16(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_roundtrip() {
        let header = PacketHeader::new(PacketType::Data, 42, 1, 100, 5);
        let pkt = Packet::new(header, Bytes::from_static(b"hello"));
        let encoded = pkt.encode();
        let decoded = Packet::decode(&encoded).unwrap();
        assert_eq!(decoded.header, pkt.header);
        assert_eq!(decoded.payload, pkt.payload);
    }

    #[test]
    fn test_selective_ack_roundtrip() {
        let sack = SelectiveAck::new(10, vec![12, 15, 20]);
        let encoded = sack.encode();
        let decoded = SelectiveAck::decode(&encoded).unwrap();
        assert_eq!(decoded.cumulative_ack, 10);
        assert_eq!(decoded.selective_acks, vec![12, 15, 20]);
    }

    #[test]
    fn test_fragment_header_roundtrip() {
        let fh = FragmentHeader {
            message_id: 999,
            fragment_index: 2,
            total_fragments: 5,
        };
        let mut buf = BytesMut::new();
        fh.encode(&mut buf);
        let mut bytes = buf.freeze();
        let decoded = FragmentHeader::decode(&mut bytes).unwrap();
        assert_eq!(decoded, fh);
    }

    #[test]
    fn test_invalid_version() {
        let mut buf = BytesMut::new();
        buf.put_u8(99); // bad version
        buf.put_u8(0x03);
        buf.put_u64(0);
        buf.put_u64(0);
        buf.put_u64(0);
        buf.put_u16(0);
        let result = Packet::decode(&buf);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_packet_type() {
        let mut buf = BytesMut::new();
        buf.put_u8(PROTOCOL_VERSION);
        buf.put_u8(0xFF); // bad type
        buf.put_u64(0);
        buf.put_u64(0);
        buf.put_u64(0);
        buf.put_u16(0);
        let result = Packet::decode(&buf);
        assert!(result.is_err());
    }
}
