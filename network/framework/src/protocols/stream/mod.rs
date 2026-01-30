// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::protocols::wire::messaging::v1::{MultiplexMessage, NetworkMessage};
use anyhow::{bail, ensure};
use aptos_channels::Sender;
use aptos_id_generator::{IdGenerator, U32IdGenerator};
use bytes::{Bytes, BytesMut};
use futures_util::SinkExt;
#[cfg(any(test, feature = "fuzzing"))]
use proptest::prelude::*;
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[cfg(any(test, feature = "fuzzing"))]
fn arbitrary_bytes() -> impl Strategy<Value = Bytes> {
    proptest::collection::vec(any::<u8>(), 0..1024).prop_map(Bytes::from)
}

// Estimated overhead per frame (in bytes)
const FRAME_OVERHEAD_BYTES: usize = 64;

/// A stream message (streams begin with a header, followed by multiple fragments)
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub enum StreamMessage {
    Header(StreamHeader),
    Fragment(StreamFragment),
}

/// A header for a stream of fragments
#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct StreamHeader {
    pub request_id: u32,
    pub num_fragments: u8,
    /// original message with chunked raw data
    pub message: NetworkMessage,
}

impl Debug for StreamHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "StreamHeader {{ request_id: {}, num_fragments: {}, message: {:?} }}",
            self.request_id, self.num_fragments, self.message
        )
    }
}

/// A single fragment in a stream
#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct StreamFragment {
    pub request_id: u32,
    pub fragment_id: u8,
    #[serde(with = "crate::protocols::wire::serde_bytes_compat")]
    #[cfg_attr(any(test, feature = "fuzzing"), proptest(strategy = "arbitrary_bytes()"))]
    pub raw_data: Bytes,
}

impl Debug for StreamFragment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "StreamFragment {{ request_id: {}, fragment_id: {}, size: {} }}",
            self.request_id,
            self.fragment_id,
            self.raw_data.len()
        )
    }
}

/// A buffer for a single inbound fragment stream
pub struct InboundStreamBuffer {
    stream: Option<InboundStream>,
    max_fragments: usize,
}

impl InboundStreamBuffer {
    pub fn new(max_fragments: usize) -> Self {
        Self {
            stream: None,
            max_fragments,
        }
    }

    /// Start a new inbound stream (returns an error if an existing stream was in progress)
    pub fn new_stream(&mut self, header: StreamHeader) -> anyhow::Result<()> {
        let inbound_stream = InboundStream::new(header, self.max_fragments)?;
        if let Some(old) = self.stream.replace(inbound_stream) {
            bail!(
                "Discarding existing stream for request ID: {}",
                old.request_id
            )
        } else {
            Ok(())
        }
    }

    /// Append a fragment to the existing stream (returns the completed message if the stream ends)
    pub fn append_fragment(
        &mut self,
        fragment: StreamFragment,
    ) -> anyhow::Result<Option<NetworkMessage>> {
        // Append the fragment to the existing stream
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("No stream exists!"))?;
        let stream_end = stream.append_fragment(fragment)?;

        // If the stream is complete, take it out and return the message
        if stream_end {
            Ok(Some(self.stream.take().unwrap().message))
        } else {
            Ok(None)
        }
    }
}

/// A single inbound stream (for streaming large messages in fragments)
pub struct InboundStream {
    request_id: u32,
    num_fragments: u8,
    received_fragment_id: u8,
    message: NetworkMessage,
    /// Accumulated fragment data. We collect all fragments and concatenate once at the end
    /// to avoid O(n²) copying that would occur if we appended to a growing buffer on each fragment.
    fragments: Vec<Bytes>,
    /// Total size of accumulated fragments for pre-allocation
    total_fragment_size: usize,
}

impl InboundStream {
    fn new(header: StreamHeader, max_fragments: usize) -> anyhow::Result<Self> {
        // Verify that max fragments is within reasonable bounds
        ensure!(
            max_fragments > 0,
            "Max fragments must be greater than zero!"
        );
        ensure!(
            max_fragments <= (u8::MAX as usize),
            "Max fragments exceeded the u8 limit: {} (max: {})!",
            max_fragments,
            u8::MAX
        );

        // Verify the header message type
        let header_message = header.message;
        ensure!(
            !matches!(header_message, NetworkMessage::Error(_)),
            "Error messages cannot be streamed!"
        );

        // Verify the number of fragments specified in the header
        let header_num_fragments = header.num_fragments;
        ensure!(
            header_num_fragments > 0,
            "Stream header must specify at least one fragment!"
        );
        ensure!(
            (header_num_fragments as usize) <= max_fragments,
            "Stream header exceeds max fragments limit!"
        );

        Ok(Self {
            request_id: header.request_id,
            num_fragments: header_num_fragments,
            received_fragment_id: 0,
            message: header_message,
            fragments: Vec::with_capacity(header_num_fragments as usize),
            total_fragment_size: 0,
        })
    }

    /// Append a fragment to the stream (returns true if the stream is complete)
    fn append_fragment(&mut self, fragment: StreamFragment) -> anyhow::Result<bool> {
        // Verify the stream request ID and fragment request ID
        ensure!(
            self.request_id == fragment.request_id,
            "Stream fragment from a different request! Expected {}, got {}.",
            self.request_id,
            fragment.request_id
        );

        // Verify the fragment ID
        let fragment_id = fragment.fragment_id;
        ensure!(fragment_id > 0, "Fragment ID must be greater than zero!");
        ensure!(
            fragment_id <= self.num_fragments,
            "Fragment ID {} exceeds number of fragments {}!",
            fragment_id,
            self.num_fragments
        );

        // Verify the fragment ID is the expected next fragment
        let expected_fragment_id = self.received_fragment_id.checked_add(1).ok_or_else(|| {
            anyhow::anyhow!(
                "Current fragment ID overflowed when adding 1: {}",
                self.received_fragment_id
            )
        })?;
        ensure!(
            expected_fragment_id == fragment_id,
            "Unexpected fragment ID, expected {}, got {}!",
            expected_fragment_id,
            fragment_id
        );

        // Update the received fragment ID
        self.received_fragment_id = expected_fragment_id;

        // Store the fragment data (zero-copy, just increments refcount)
        self.total_fragment_size += fragment.raw_data.len();
        self.fragments.push(fragment.raw_data);

        // Check if the stream is complete
        let is_stream_complete = self.received_fragment_id == self.num_fragments;

        // If complete, concatenate all fragments into the message in one pass (O(n) total)
        if is_stream_complete {
            self.finalize_message();
        }

        Ok(is_stream_complete)
    }

    /// Concatenate all accumulated fragments into the message payload.
    /// This is called once when all fragments are received, making the total
    /// copy operation O(n) instead of O(n²).
    fn finalize_message(&mut self) {
        // Get the initial data from the header and calculate total size
        let (initial_data, total_size) = match &self.message {
            NetworkMessage::Error(_) => {
                panic!("StreamHeader for NetworkMessage::Error(_) should be rejected!")
            },
            NetworkMessage::RpcRequest(request) => {
                (request.raw_request.clone(), request.raw_request.len() + self.total_fragment_size)
            },
            NetworkMessage::RpcResponse(response) => {
                (response.raw_response.clone(), response.raw_response.len() + self.total_fragment_size)
            },
            NetworkMessage::DirectSendMsg(message) => {
                (message.raw_msg.clone(), message.raw_msg.len() + self.total_fragment_size)
            },
        };

        // Allocate once with exact capacity needed
        let mut buf = BytesMut::with_capacity(total_size);
        buf.extend_from_slice(&initial_data);
        for fragment in &self.fragments {
            buf.extend_from_slice(fragment);
        }
        let final_data = buf.freeze();

        // Update the message with the final concatenated data
        match &mut self.message {
            NetworkMessage::Error(_) => unreachable!(),
            NetworkMessage::RpcRequest(request) => {
                request.raw_request = final_data;
            },
            NetworkMessage::RpcResponse(response) => {
                response.raw_response = final_data;
            },
            NetworkMessage::DirectSendMsg(message) => {
                message.raw_msg = final_data;
            },
        }

        // Clear fragments to free memory
        self.fragments.clear();
    }
}

/// An outbound stream for streaming large messages in fragments
pub struct OutboundStream {
    request_id_gen: U32IdGenerator,
    max_frame_size: usize,
    max_message_size: usize,
    stream_tx: Sender<MultiplexMessage>,
}

impl OutboundStream {
    pub fn new(
        max_frame_size: usize,
        max_message_size: usize,
        stream_tx: Sender<MultiplexMessage>,
    ) -> Self {
        // Calculate the effective max frame size (subtracting overhead)
        let max_frame_size = max_frame_size
            .checked_sub(FRAME_OVERHEAD_BYTES)
            .expect("Frame size too small, overhead exceeds frame size!");

        // Ensure that the max message size can be supported with the given frame size
        assert!(
            (max_frame_size * (u8::MAX as usize)) >= max_message_size,
            "Stream only supports {} chunks! Frame size {}, message size {}.",
            u8::MAX,
            max_frame_size,
            max_message_size
        );

        Self {
            request_id_gen: U32IdGenerator::new(),
            max_frame_size,
            max_message_size,
            stream_tx,
        }
    }

    /// Returns true if the message should be streamed
    pub fn should_stream(&self, message: &NetworkMessage) -> bool {
        message.data_len() > self.max_frame_size
    }

    /// Streams a large message in fragments
    pub async fn stream_message(&mut self, mut message: NetworkMessage) -> anyhow::Result<()> {
        // Verify that the message is not an error message
        ensure!(
            !matches!(message, NetworkMessage::Error(_)),
            "Error messages should not be streamed!"
        );

        // Verify that the message size is within limits
        let message_data_len = message.data_len();
        ensure!(
            message_data_len <= self.max_message_size,
            "Message length {} exceeds max message size {}!",
            message_data_len,
            self.max_message_size,
        );

        // Verify that the message size exceeds the frame size
        ensure!(
            message_data_len >= self.max_frame_size,
            "Message length {} is smaller than frame size {}! It should not be streamed.",
            message_data_len,
            self.max_frame_size,
        );

        // Generate a new request ID for the stream
        let request_id = self.request_id_gen.next();

        // Split the message data into chunks using zero-copy slicing
        // The header keeps the first chunk, rest is split into fragments
        let rest = match &mut message {
            NetworkMessage::Error(_) => {
                unreachable!("NetworkMessage::Error(_) should always fit into a single frame!")
            },
            NetworkMessage::RpcRequest(request) => {
                let data = std::mem::take(&mut request.raw_request);
                request.raw_request = data.slice(..self.max_frame_size);
                data.slice(self.max_frame_size..)
            },
            NetworkMessage::RpcResponse(response) => {
                let data = std::mem::take(&mut response.raw_response);
                response.raw_response = data.slice(..self.max_frame_size);
                data.slice(self.max_frame_size..)
            },
            NetworkMessage::DirectSendMsg(message) => {
                let data = std::mem::take(&mut message.raw_msg);
                message.raw_msg = data.slice(..self.max_frame_size);
                data.slice(self.max_frame_size..)
            },
        };
        // Calculate number of chunks (each chunk is max_frame_size except possibly the last)
        let rest_len = rest.len();
        let num_full_chunks = rest_len / self.max_frame_size;
        let has_partial_chunk = (rest_len % self.max_frame_size) > 0;
        let num_chunks = num_full_chunks + if has_partial_chunk { 1 } else { 0 };

        // Ensure that the number of chunks does not exceed u8::MAX
        ensure!(
            num_chunks <= (u8::MAX as usize),
            "Number of fragments overflowed the u8 limit: {} (max: {})!",
            num_chunks,
            u8::MAX
        );

        // Send the stream header
        let header = StreamMessage::Header(StreamHeader {
            request_id,
            num_fragments: num_chunks as u8,
            message,
        });
        self.stream_tx
            .send(MultiplexMessage::Stream(header))
            .await?;

        // Send each fragment using zero-copy slicing
        for index in 0..num_chunks {
            // Calculate the fragment ID (note: fragment IDs start at 1)
            let fragment_id = index.checked_add(1).ok_or_else(|| {
                anyhow::anyhow!("Fragment ID overflowed when adding 1: {}", index)
            })?;

            // Calculate the slice range for this chunk
            let start = index * self.max_frame_size;
            let end = std::cmp::min(start + self.max_frame_size, rest_len);
            let chunk = rest.slice(start..end);

            // Send the fragment message
            let message = StreamMessage::Fragment(StreamFragment {
                request_id,
                fragment_id: fragment_id as u8,
                raw_data: chunk, // Zero-copy: Bytes::slice() shares the underlying buffer
            });
            self.stream_tx
                .send(MultiplexMessage::Stream(message))
                .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::{
        stream::{InboundStreamBuffer, StreamHeader},
        wire::{
            handshake::v1::ProtocolId::ConsensusRpcBcs,
            messaging::v1::{DirectSendMsg, ErrorCode, NetworkMessage, NotSupportedType},
        },
    };

    #[test]
    pub fn test_inbound_stream_buffer_new_stream() {
        // Create an inbound stream buffer
        let max_fragments = 10;
        let mut inbound_stream_buffer = InboundStreamBuffer::new(max_fragments);

        // Start a new stream
        let stream_header = create_stream_header(1, 5);
        assert!(inbound_stream_buffer.new_stream(stream_header).is_ok());

        // Attempt to start another stream without completing the first one
        let another_stream_header = create_stream_header(2, 6);
        assert!(inbound_stream_buffer
            .new_stream(another_stream_header)
            .is_err());
    }

    #[test]
    pub fn test_inbound_stream_buffer_append_fragment() {
        // Create an inbound stream buffer
        let max_fragments = 10;
        let mut inbound_stream_buffer = InboundStreamBuffer::new(max_fragments);

        // Attempt to append a fragment without starting a stream
        assert!(inbound_stream_buffer.stream.is_none());
        let stream_fragment = create_stream_fragment(1, 1);
        assert!(inbound_stream_buffer
            .append_fragment(stream_fragment)
            .is_err());

        // Start a new stream
        let request_id = 1;
        let num_fragments = 3;
        let stream_header = create_stream_header(request_id, num_fragments);
        assert!(inbound_stream_buffer.new_stream(stream_header).is_ok());

        // Append fragments and check for completion
        for fragment_id in 1..=num_fragments {
            // Append the fragment
            let stream_fragment = create_stream_fragment(request_id, fragment_id);
            let result = inbound_stream_buffer.append_fragment(stream_fragment);
            assert!(result.is_ok());

            // Check if the stream is complete
            let is_complete = result.unwrap().is_some();
            assert_eq!(is_complete, fragment_id == num_fragments);
        }

        // Verify that no stream exists after completion
        assert!(inbound_stream_buffer.stream.is_none());
    }

    #[test]
    pub fn test_inbound_stream_creation() {
        // Create an inbound stream with zero max fragments (and verify it fails)
        let stream_header = create_stream_header(1, 5);
        let inbound_stream = InboundStream::new(stream_header, 0);
        assert!(inbound_stream.is_err());

        // Create an inbound stream with excessive max fragments (and verify it fails)
        let stream_header = create_stream_header(1, 5);
        let inbound_stream = InboundStream::new(stream_header, 300);
        assert!(inbound_stream.is_err());

        // Create an inbound stream with an error message (and verify it fails)
        let stream_header = StreamHeader {
            request_id: 1,
            num_fragments: 5,
            message: NetworkMessage::Error(ErrorCode::NotSupported(
                NotSupportedType::DirectSendMsg(ConsensusRpcBcs),
            )),
        };
        let inbound_stream = InboundStream::new(stream_header, 10);
        assert!(inbound_stream.is_err());

        // Create an inbound stream with zero fragments (and verify it fails)
        let stream_header = create_stream_header(1, 0);
        let inbound_stream = InboundStream::new(stream_header, 10);
        assert!(inbound_stream.is_err());

        // Create an inbound stream with fragments exceeding max fragments (and verify it fails)
        let max_fragments = 10;
        let stream_header = create_stream_header(1, max_fragments + 1);
        let inbound_stream = InboundStream::new(stream_header, max_fragments as usize);
        assert!(inbound_stream.is_err());
    }

    #[test]
    pub fn test_inbound_stream_append_fragment() {
        // Create a valid inbound stream
        let request_id = 1;
        let max_fragments = 100;
        let stream_header = create_stream_header(request_id, max_fragments);
        let mut inbound_stream = InboundStream::new(stream_header, max_fragments as usize).unwrap();

        // Append fragments with an incorrect request ID (and verify it fails)
        let invalid_fragment = create_stream_fragment(2, 1);
        let result = inbound_stream.append_fragment(invalid_fragment);
        assert!(result.is_err());

        // Append fragments with a zero fragment ID (and verify it fails)
        let zero_fragment_id = create_stream_fragment(request_id, 0);
        let result = inbound_stream.append_fragment(zero_fragment_id);
        assert!(result.is_err());

        // Append fragments with a fragment ID exceeding num_fragments (and verify it fails)
        let exceeding_fragment_id = create_stream_fragment(request_id, max_fragments + 1);
        let result = inbound_stream.append_fragment(exceeding_fragment_id);
        assert!(result.is_err());

        // Append fragments with an out-of-order fragment ID (and verify it fails)
        let out_of_order_fragment = create_stream_fragment(request_id, 2);
        let result = inbound_stream.append_fragment(out_of_order_fragment);
        assert!(result.is_err());

        // Append valid fragments and check for completion
        for fragment_id in 1..=max_fragments {
            // Append the fragment
            let valid_fragment = create_stream_fragment(request_id, fragment_id);
            let result = inbound_stream.append_fragment(valid_fragment);
            assert!(result.is_ok());

            // Check if the stream is complete
            let is_complete = result.unwrap();
            assert_eq!(is_complete, fragment_id == max_fragments);
        }
    }

    #[test]
    pub fn test_inbound_stream_append_fragment_max() {
        // Create a valid inbound stream with the maximum number of fragments
        let request_id = 1;
        let max_fragments = 255;
        let stream_header = create_stream_header(request_id, max_fragments);
        let mut inbound_stream = InboundStream::new(stream_header, max_fragments as usize).unwrap();

        // Append valid fragments and check for completion
        for fragment_id in 1..=max_fragments {
            // Append the fragment
            let valid_fragment = create_stream_fragment(request_id, fragment_id);
            let result = inbound_stream.append_fragment(valid_fragment);
            assert!(result.is_ok());

            // Check if the stream is complete
            let is_complete = result.unwrap();
            assert_eq!(is_complete, fragment_id == max_fragments);
        }

        // Attempt to add another fragment at the max limit (and verify it fails)
        let exceeding_fragment_id = create_stream_fragment(request_id, max_fragments);
        let result = inbound_stream.append_fragment(exceeding_fragment_id);
        assert!(result.is_err());
    }

    /// Creates a stream fragment for testing purposes
    fn create_stream_fragment(request_id: u32, fragment_id: u8) -> StreamFragment {
        StreamFragment {
            request_id,
            fragment_id,
            raw_data: Bytes::from(vec![0u8; 10]), // Dummy data
        }
    }

    /// Creates a stream header for testing purposes
    fn create_stream_header(request_id: u32, num_fragments: u8) -> StreamHeader {
        StreamHeader {
            request_id,
            num_fragments,
            message: NetworkMessage::DirectSendMsg(DirectSendMsg {
                protocol_id: ConsensusRpcBcs,
                priority: 0,
                raw_msg: Bytes::new(),
            }),
        }
    }
}
