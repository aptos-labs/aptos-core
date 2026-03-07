// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Logical stream multiplexing over a single UDP connection.
//!
//! Each stream has its own:
//! - Sequence number space for ordering
//! - Fragmentation/reassembly for large messages
//! - Send and receive buffers

use crate::{
    error::{QnucError, Result},
    packet::{
        FragmentHeader, Packet, PacketHeader, PacketType, FRAGMENT_HEADER_SIZE,
        MAX_MESSAGE_SIZE, MAX_PACKET_PAYLOAD,
    },
    reliability::{RecvTracker, ReliabilityConfig, SendTracker},
};
use bytes::{Bytes, BytesMut};
use std::collections::BTreeMap;

/// A single logical stream within a connection.
pub struct Stream {
    pub stream_id: u64,
    connection_id: u64,
    send_tracker: SendTracker,
    recv_tracker: RecvTracker,
    /// Reassembly buffer: message_id -> (received fragments, total expected, accumulated data).
    reassembly: BTreeMap<u64, ReassemblyState>,
    /// Messages that have been fully reassembled and are ready for delivery.
    ready_messages: Vec<Bytes>,
    next_message_id: u64,
}

struct ReassemblyState {
    total_fragments: u16,
    received: BTreeMap<u16, Vec<u8>>,
}

impl Stream {
    pub fn new(stream_id: u64, connection_id: u64, config: ReliabilityConfig) -> Self {
        Self {
            stream_id,
            connection_id,
            send_tracker: SendTracker::new(config),
            recv_tracker: RecvTracker::new(),
            reassembly: BTreeMap::new(),
            ready_messages: Vec::new(),
            next_message_id: 0,
        }
    }

    /// Fragment a message into packets ready to send.
    /// Returns a list of (sequence_number, encoded_packet_bytes).
    pub fn prepare_send(&mut self, message: &[u8]) -> Result<Vec<(u64, Vec<u8>)>> {
        if message.len() > MAX_MESSAGE_SIZE {
            return Err(QnucError::PayloadTooLarge {
                size: message.len(),
                max: MAX_MESSAGE_SIZE,
            });
        }

        let usable_payload = MAX_PACKET_PAYLOAD - FRAGMENT_HEADER_SIZE;
        let total_fragments = message.len().div_ceil(usable_payload) as u16;
        let total_fragments = std::cmp::max(total_fragments, 1);
        let message_id = self.next_message_id;
        self.next_message_id += 1;

        let mut packets = Vec::new();

        for frag_idx in 0..total_fragments {
            let start = (frag_idx as usize) * usable_payload;
            let end = std::cmp::min(start + usable_payload, message.len());
            let fragment_data = &message[start..end];

            // Build fragment header + data
            let mut payload_buf = BytesMut::with_capacity(FRAGMENT_HEADER_SIZE + fragment_data.len());
            let frag_header = FragmentHeader {
                message_id,
                fragment_index: frag_idx,
                total_fragments,
            };
            frag_header.encode(&mut payload_buf);
            payload_buf.extend_from_slice(fragment_data);
            let payload_bytes = payload_buf.freeze();

            let seq = self.send_tracker.register_sent(payload_bytes.to_vec());

            let header = PacketHeader::new(
                PacketType::Data,
                self.connection_id,
                self.stream_id,
                seq,
                payload_bytes.len() as u16,
            );
            let pkt = Packet::new(header, payload_bytes);
            packets.push((seq, pkt.encode().to_vec()));
        }

        Ok(packets)
    }

    /// Process a received data packet.
    /// Returns fully reassembled messages (if any).
    pub fn process_received(&mut self, seq: u64, payload: &[u8]) -> Result<Vec<Bytes>> {
        // Feed into reliability layer for ordering
        let delivered = self.recv_tracker.receive(seq, payload.to_vec());
        let had_deliveries = !delivered.is_empty();

        let mut complete_messages = Vec::new();

        for (_delivered_seq, delivered_data) in delivered {
            let mut dbytes = Bytes::copy_from_slice(&delivered_data);
            let fh = FragmentHeader::decode(&mut dbytes)?;
            let fdata = dbytes.to_vec();

            let state = self.reassembly.entry(fh.message_id).or_insert_with(|| {
                ReassemblyState {
                    total_fragments: fh.total_fragments,
                    received: BTreeMap::new(),
                }
            });

            state.received.insert(fh.fragment_index, fdata);

            if state.received.len() == state.total_fragments as usize {
                let mut message = Vec::new();
                for idx in 0..state.total_fragments {
                    if let Some(data) = state.received.get(&idx) {
                        message.extend_from_slice(data);
                    } else {
                        return Err(QnucError::InvalidPacket(format!(
                            "missing fragment {} of message {}",
                            idx, fh.message_id,
                        )));
                    }
                }
                self.reassembly.remove(&fh.message_id);
                complete_messages.push(Bytes::from(message));
            }
        }

        // For out-of-order packets that didn't get delivered by recv_tracker,
        // store the fragment in the reassembly buffer anyway.
        if !had_deliveries {
            let mut bytes = Bytes::copy_from_slice(payload);
            let frag_header = FragmentHeader::decode(&mut bytes)?;
            let fragment_data = bytes.to_vec();

            let state = self
                .reassembly
                .entry(frag_header.message_id)
                .or_insert_with(|| ReassemblyState {
                    total_fragments: frag_header.total_fragments,
                    received: BTreeMap::new(),
                });
            state
                .received
                .insert(frag_header.fragment_index, fragment_data);
        }

        Ok(complete_messages)
    }

    /// Generate an ACK packet for this stream.
    pub fn generate_ack_packet(&self) -> Vec<u8> {
        let sack = self.recv_tracker.generate_ack();
        let ack_payload = sack.encode();
        let header = PacketHeader::new(
            PacketType::Ack,
            self.connection_id,
            self.stream_id,
            0,
            ack_payload.len() as u16,
        );
        let pkt = Packet::new(header, ack_payload);
        pkt.encode().to_vec()
    }

    /// Process an ACK received for this stream.
    pub fn process_ack(&mut self, payload: &[u8]) -> Result<Vec<u64>> {
        let sack = crate::packet::SelectiveAck::decode(payload)?;
        Ok(self.send_tracker.process_ack(&sack))
    }

    /// Get packets needing retransmission.
    pub fn get_retransmissions(&mut self) -> Vec<(u64, Vec<u8>)> {
        self.send_tracker.get_retransmissions()
    }

    /// Pop any ready messages.
    pub fn take_ready_messages(&mut self) -> Vec<Bytes> {
        std::mem::take(&mut self.ready_messages)
    }

    pub fn send_tracker(&self) -> &SendTracker {
        &self.send_tracker
    }

    pub fn can_send(&self) -> bool {
        self.send_tracker.congestion.can_send(self.send_tracker.in_flight())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_send_small_message() {
        let mut stream = Stream::new(1, 100, ReliabilityConfig::default());
        let packets = stream.prepare_send(b"hello world").unwrap();
        assert_eq!(packets.len(), 1);
    }

    #[test]
    fn test_stream_send_large_message() {
        let mut stream = Stream::new(1, 100, ReliabilityConfig::default());
        let large_msg = vec![0xABu8; MAX_PACKET_PAYLOAD * 3];
        let packets = stream.prepare_send(&large_msg).unwrap();
        assert!(packets.len() >= 3);
    }

    #[test]
    fn test_stream_send_receive_roundtrip() {
        let config = ReliabilityConfig::default();
        let mut sender = Stream::new(1, 100, config.clone());
        let mut receiver = Stream::new(1, 100, config);

        let original = b"test message for roundtrip";
        let packets = sender.prepare_send(original).unwrap();

        let mut all_messages = Vec::new();
        for (seq, pkt_bytes) in &packets {
            let pkt = Packet::decode(pkt_bytes).unwrap();
            let msgs = receiver.process_received(*seq, &pkt.payload).unwrap();
            all_messages.extend(msgs);
        }

        assert_eq!(all_messages.len(), 1);
        assert_eq!(all_messages[0].as_ref(), original);
    }

    #[test]
    fn test_stream_ack_generation() {
        let config = ReliabilityConfig::default();
        let mut stream = Stream::new(1, 100, config);

        // Simulate receiving some packets
        let _ = stream.recv_tracker.receive(0, b"data0".to_vec());
        let _ = stream.recv_tracker.receive(1, b"data1".to_vec());

        let ack_bytes = stream.generate_ack_packet();
        let ack_pkt = Packet::decode(&ack_bytes).unwrap();
        assert_eq!(ack_pkt.header.packet_type, PacketType::Ack);
    }

    #[test]
    fn test_message_too_large() {
        let mut stream = Stream::new(1, 100, ReliabilityConfig::default());
        let huge = vec![0u8; MAX_MESSAGE_SIZE + 1];
        let result = stream.prepare_send(&huge);
        assert!(result.is_err());
    }
}
