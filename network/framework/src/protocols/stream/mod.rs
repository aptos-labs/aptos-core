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
            StreamMessage::Header(head) => head.message.data_len(),
            StreamMessage::Fragment(frag) => frag.raw_data.len(),
        }
    }

    pub fn header_len(&self) -> usize {
        match self {
            StreamMessage::Header(head) => {
                // 5 bytes for {request_id: u32, num_fragments: u8} in StreamMessage::Header(StreamHeader{...})
                head.message.header_len() + 5
            },
            StreamMessage::Fragment(_frag) => {
                // 5 bytes for {request_id: u32, frament_id: u8} in StreamMessage::Fragment(StreamFragment{...})
                5
            },
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
