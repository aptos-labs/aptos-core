// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::protocols::wire::messaging::v1::{MultiplexMessage, NetworkMessage};
use anyhow::{bail, ensure};
use aptos_id_generator::{IdGenerator, U32IdGenerator};
use channel::Sender;
use futures_util::SinkExt;
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub enum StreamMessage {
    Header(StreamHeader),
    Fragment(StreamFragment),
}

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct StreamHeader {
    pub request_id: u32,
    pub num_fragments: u8,
    /// original message with chunked raw data
    pub message: NetworkMessage,
}

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct StreamFragment {
    pub request_id: u32,
    pub fragment_id: u8,
    #[serde(with = "serde_bytes")]
    pub raw_data: Vec<u8>,
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

pub struct InboundStreamBuffer {
    stream: Option<InboundStream>,
}

impl InboundStreamBuffer {
    pub fn new() -> Self {
        Self { stream: None }
    }

    pub fn new_stream(&mut self, header: StreamHeader) -> anyhow::Result<()> {
        if let Some(old) = self.stream.replace(InboundStream::new(header)?) {
            bail!("Discard existing stream {}", old.request_id)
        } else {
            Ok(())
        }
    }

    pub fn append_fragment(
        &mut self,
        fragment: StreamFragment,
    ) -> anyhow::Result<Option<NetworkMessage>> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("No stream exist"))?;
        let stream_end = stream.append_fragment(fragment)?;
        if stream_end {
            Ok(Some(self.stream.take().unwrap().message))
        } else {
            Ok(None)
        }
    }
}

pub struct InboundStream {
    request_id: u32,
    num_fragments: u8,
    current_fragment_id: u8,
    message: NetworkMessage,
}

impl InboundStream {
    fn new(header: StreamHeader) -> anyhow::Result<Self> {
        ensure!(
            !matches!(header.message, NetworkMessage::Error(_)),
            "Error message is not expected for stream"
        );
        Ok(Self {
            request_id: header.request_id,
            num_fragments: header.num_fragments,
            current_fragment_id: 0,
            message: header.message,
        })
    }

    fn append_fragment(&mut self, mut fragment: StreamFragment) -> anyhow::Result<bool> {
        ensure!(
            self.request_id == fragment.request_id,
            "Stream fragment from a different request"
        );
        ensure!(
            self.current_fragment_id + 1 == fragment.fragment_id,
            "Unexpected fragment id, expected {}, got {}",
            self.current_fragment_id + 1,
            fragment.fragment_id
        );
        self.current_fragment_id += 1;
        let raw_data = &mut fragment.raw_data;
        match &mut self.message {
            NetworkMessage::Error(_) => panic!("StreamHeader with Error should be rejected"),
            NetworkMessage::RpcRequest(request) => request.raw_request.append(raw_data),
            NetworkMessage::RpcResponse(response) => response.raw_response.append(raw_data),
            NetworkMessage::DirectSendMsg(message) => message.raw_msg.append(raw_data),
        }
        Ok(self.current_fragment_id == self.num_fragments)
    }
}

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
        // some buffer for headers
        let max_frame_size = max_frame_size - 64;
        assert!(
            max_frame_size * u8::MAX as usize >= max_message_size,
            "Stream only supports maximum 255 chunks, frame size {}, message size {}",
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

    pub fn should_stream(&self, message: &NetworkMessage) -> bool {
        message.data_len() > self.max_frame_size
    }

    pub async fn stream_message(&mut self, mut message: NetworkMessage) -> anyhow::Result<()> {
        ensure!(
            message.data_len() <= self.max_message_size,
            "Message length {} exceed size limit {}",
            message.data_len(),
            self.max_message_size,
        );
        ensure!(
            message.data_len() >= self.max_frame_size,
            "Message length {} is smaller than frame size {}, should not go through stream",
            message.data_len(),
            self.max_frame_size,
        );
        let request_id = self.request_id_gen.next();
        let rest = match &mut message {
            NetworkMessage::Error(_) => {
                unreachable!("NetworkMessage::Error should always fit in a single frame")
            }
            NetworkMessage::RpcRequest(request) => {
                request.raw_request.split_off(self.max_frame_size)
            }
            NetworkMessage::RpcResponse(response) => {
                response.raw_response.split_off(self.max_frame_size)
            }
            NetworkMessage::DirectSendMsg(message) => {
                message.raw_msg.split_off(self.max_frame_size)
            }
        };
        let chunks = rest.chunks(self.max_frame_size);
        ensure!(
            chunks.len() <= u8::MAX as usize,
            "Number of fragments overflowed"
        );
        let header = StreamMessage::Header(StreamHeader {
            request_id,
            num_fragments: chunks.len() as u8,
            message,
        });
        self.stream_tx
            .send(MultiplexMessage::Stream(header))
            .await?;
        for (index, chunk) in chunks.enumerate() {
            let message = StreamMessage::Fragment(StreamFragment {
                request_id,
                fragment_id: index as u8 + 1,
                raw_data: Vec::from(chunk),
            });
            self.stream_tx
                .send(MultiplexMessage::Stream(message))
                .await?;
        }
        Ok(())
    }
}
