// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines representation of VelorDB indexer data structures at physical level via schemas
//! that implement [`velor_schemadb::schema::Schema`].
//!
//! All schemas are `pub(crate)` so not shown in rustdoc, refer to the source code to see details.

pub mod event_by_key;
pub mod event_by_version;
pub mod event_sequence_number;
pub mod indexer_metadata;
pub mod ordered_transaction_by_account;
pub mod state_keys;
pub mod table_info;
pub mod translated_v1_event;

use anyhow::ensure;
use velor_schemadb::ColumnFamilyName;

pub const DEFAULT_COLUMN_FAMILY_NAME: ColumnFamilyName = "default";
pub const INDEXER_METADATA_CF_NAME: ColumnFamilyName = "indexer_metadata";
pub const INTERNAL_INDEXER_METADATA_CF_NAME: ColumnFamilyName = "internal_indexer_metadata";
pub const TABLE_INFO_CF_NAME: ColumnFamilyName = "table_info";
pub const EVENT_BY_KEY_CF_NAME: ColumnFamilyName = "event_by_key";
pub const EVENT_BY_VERSION_CF_NAME: ColumnFamilyName = "event_by_version";
pub const ORDERED_TRANSACTION_BY_ACCOUNT_CF_NAME: ColumnFamilyName = "transaction_by_account";
pub const STATE_KEYS_CF_NAME: ColumnFamilyName = "state_keys";
pub const TRANSLATED_V1_EVENT_CF_NAME: ColumnFamilyName = "translated_v1_event";
pub const EVENT_SEQUENCE_NUMBER_CF_NAME: ColumnFamilyName = "event_sequence_number";

pub fn column_families() -> Vec<ColumnFamilyName> {
    vec![
        /* empty cf */ DEFAULT_COLUMN_FAMILY_NAME,
        INDEXER_METADATA_CF_NAME,
        TABLE_INFO_CF_NAME,
    ]
}

pub fn internal_indexer_column_families() -> Vec<ColumnFamilyName> {
    vec![
        /* empty cf */ DEFAULT_COLUMN_FAMILY_NAME,
        INTERNAL_INDEXER_METADATA_CF_NAME,
        EVENT_BY_KEY_CF_NAME,
        EVENT_BY_VERSION_CF_NAME,
        ORDERED_TRANSACTION_BY_ACCOUNT_CF_NAME,
        STATE_KEYS_CF_NAME,
        TRANSLATED_V1_EVENT_CF_NAME,
        EVENT_SEQUENCE_NUMBER_CF_NAME,
    ]
}

fn ensure_slice_len_eq(data: &[u8], len: usize) -> anyhow::Result<()> {
    ensure!(
        data.len() == len,
        "Unexpected data len {}, expected {}.",
        data.len(),
        len,
    );
    Ok(())
}
