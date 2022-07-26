// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module defines physical storage schema storing medadata for the internal indexer
//!

use crate::metadata::{Metadata, MetadataTag};
use crate::schema::INDEXER_METADATA_CF_NAME;
use anyhow::{anyhow, Result};
use byteorder::ReadBytesExt;
use num_traits::{FromPrimitive, ToPrimitive};
use schemadb::{
    define_schema,
    schema::{KeyCodec, ValueCodec},
};

define_schema!(
    IndexerMetadataSchema,
    MetadataTag,
    Metadata,
    INDEXER_METADATA_CF_NAME
);

impl KeyCodec<IndexerMetadataSchema> for MetadataTag {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(vec![self.to_u8().unwrap()])
    }

    fn decode_key(mut data: &[u8]) -> Result<Self> {
        Self::from_u8(data.read_u8()?).ok_or_else(|| anyhow!("Failed to parse MetadataTag."))
    }
}

impl ValueCodec<IndexerMetadataSchema> for Metadata {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

#[cfg(test)]
mod test;
