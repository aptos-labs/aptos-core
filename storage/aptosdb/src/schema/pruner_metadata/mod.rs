// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
//! This module defines the physical storage schema for indexes of min_readable_version of pruners.
//! For the state pruner, the metadata represents the key of the StaleNodeIndexSchema and for the
//! ledger pruner, the metadata represents the key of the TransactionSchema.
//!
//! ```text
//! |<------key---->|<------ value ----->|
//! | pruner tag    | pruned until values|
//! ```
//!

use crate::pruner::pruner_metadata::{PrunerMetadata, PrunerTag};
use crate::schema::DB_METADATA_CF_NAME;
use anyhow::{anyhow, Result};
use byteorder::ReadBytesExt;
use num_traits::{FromPrimitive, ToPrimitive};
use schemadb::{
    define_schema,
    schema::{KeyCodec, ValueCodec},
};

define_schema!(
    PrunerMetadataSchema,
    PrunerTag,
    PrunerMetadata,
    DB_METADATA_CF_NAME
);

impl KeyCodec<PrunerMetadataSchema> for PrunerTag {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(vec![self.to_u8().unwrap()])
    }

    fn decode_key(mut data: &[u8]) -> Result<Self> {
        Self::from_u8(data.read_u8()?).ok_or_else(|| anyhow!("Failed to parse PrunerTag."))
    }
}

impl ValueCodec<PrunerMetadataSchema> for PrunerMetadata {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

#[cfg(test)]
mod test;
