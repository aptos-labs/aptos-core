// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module defines physical storage schema storing metadata for the internal indexer
//!

use crate::metadata::{MetadataKey, MetadataValue};
use crate::schema::INDEXER_METADATA_CF_NAME;
use anyhow::Result;
use schemadb::{
    define_schema,
    schema::{KeyCodec, ValueCodec},
};

define_schema!(
    IndexerMetadataSchema,
    MetadataKey,
    MetadataValue,
    INDEXER_METADATA_CF_NAME
);

impl KeyCodec<IndexerMetadataSchema> for MetadataKey {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl ValueCodec<IndexerMetadataSchema> for MetadataValue {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

#[cfg(test)]
mod test;
