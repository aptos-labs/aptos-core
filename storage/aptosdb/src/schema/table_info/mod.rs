// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module defines physical storage schema mapping table handles to its metadata including
//! key / value type tags.
//!
//! ```text
//! |<--key-->|<---value-->|
//! | handle  | table_info |
//! ```

use crate::schema::TABLE_INFO_CF_NAME;
use anyhow::Result;
use aptos_types::state_store::table::{TableHandle, TableInfo};
use schemadb::{
    define_schema,
    schema::{KeyCodec, ValueCodec},
};

define_schema!(TableInfoSchema, TableHandle, TableInfo, TABLE_INFO_CF_NAME);

impl KeyCodec<TableInfoSchema> for TableHandle {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl ValueCodec<TableInfoSchema> for TableInfo {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

#[cfg(test)]
mod test;
