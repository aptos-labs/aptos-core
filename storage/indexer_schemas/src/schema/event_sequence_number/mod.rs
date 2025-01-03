// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This module defines physical storage schema for event sequence numbers for associated event keys,
//! specifically for translated v1 events.
//!
//! ```text
//! |<--key---->|<-value->|
//! | event_key | seq_num |
//! ```

use crate::schema::EVENT_SEQUENCE_NUMBER_CF_NAME;
use anyhow::Result;
use aptos_schemadb::{
    define_pub_schema,
    schema::{KeyCodec, ValueCodec},
};
use aptos_types::event::EventKey;

define_pub_schema!(
    EventSequenceNumberSchema,
    Key,
    Value,
    EVENT_SEQUENCE_NUMBER_CF_NAME
);

type SeqNum = u64;
type Key = EventKey;
type Value = SeqNum;

impl KeyCodec<EventSequenceNumberSchema> for Key {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl ValueCodec<EventSequenceNumberSchema> for Value {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

#[cfg(test)]
mod test;
