// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::types::{BatchId, PersistedValue};
use anyhow::Result;
use aptos_crypto::HashValue;
use schemadb::{
    schema::{KeyCodec, Schema, ValueCodec},
    ColumnFamilyName,
};

pub(crate) const BATCH_CF_NAME: ColumnFamilyName = "batch";
pub(crate) const BATCH_ID_CF_NAME: ColumnFamilyName = "batch_ID";

pub(crate) struct BatchSchema;

impl Schema for BatchSchema {
    const COLUMN_FAMILY_NAME: schemadb::ColumnFamilyName = BATCH_CF_NAME;
    type Key = HashValue;
    type Value = PersistedValue;
}

impl KeyCodec<BatchSchema> for HashValue {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(self.to_vec())
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        Ok(HashValue::from_slice(data)?)
    }
}

impl ValueCodec<BatchSchema> for PersistedValue {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

pub(crate) struct BatchIdSchema;

impl Schema for BatchIdSchema {
    const COLUMN_FAMILY_NAME: schemadb::ColumnFamilyName = BATCH_ID_CF_NAME;
    type Key = u64;
    type Value = BatchId;
}

impl KeyCodec<BatchIdSchema> for u64 {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl ValueCodec<BatchIdSchema> for BatchId {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}
