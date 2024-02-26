// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::protocols::wire::messaging::v1::NetworkMessage;
// use anyhow::{bail, ensure};
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

impl StreamMessage {
    pub fn data_len(&self) -> usize {
        match self {
            StreamMessage::Header(head) => {head.message.data_len()}
            StreamMessage::Fragment(frag) => {frag.raw_data.len()}
        }
    }
    pub fn header_len(&self) -> usize {
        match self {
            StreamMessage::Header(head) => {
                // 5 bytes for {request_id: u32, num_fragments: u8} in StreamMessage::Header(StreamHeader{...})
                head.message.header_len() + 5
            }
            StreamMessage::Fragment(_frag) => {
                // 5 bytes for {request_id: u32, frament_id: u8} in StreamMessage::Fragment(StreamFragment{...})
                5
            }
        }
    }
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

#[cfg(obsolete)]
pub struct InboundStreamBuffer {
    stream: Option<InboundStream>,
    max_fragments: usize,
}

#[cfg(obsolete)]
impl InboundStreamBuffer {
    pub fn new(max_fragments: usize) -> Self {
        Self {
            stream: None,
            max_fragments,
        }
    }

    pub fn new_stream(&mut self, header: StreamHeader) -> anyhow::Result<()> {
        if let Some(old) = self
            .stream
            .replace(InboundStream::new(header, self.max_fragments)?)
        {
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

#[cfg(obsolete)]
pub struct InboundStream {
    request_id: u32,
    num_fragments: u8,
    current_fragment_id: u8,
    message: NetworkMessage,
}

#[cfg(obsolete)]
impl InboundStream {
    fn new(header: StreamHeader, max_fragments: usize) -> anyhow::Result<Self> {
        ensure!(
            !matches!(header.message, NetworkMessage::Error(_)),
            "Error message is not expected for stream"
        );
        ensure!(
            header.num_fragments as usize <= max_fragments,
            "Stream header exceeds max fragments limit"
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
