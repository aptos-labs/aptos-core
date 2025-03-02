// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines representation of AptosDB indexer data structures at physical level via schemas
//! that implement [`aptos_schemadb::schema::Schema`].
//!
//! All schemas are `pub(crate)` so not shown in rustdoc, refer to the source code to see details.

pub mod event_by_key;
pub mod event_by_type;
pub mod event_by_version;
pub mod event_sequence_number;
pub mod indexer_metadata;
pub mod state_keys;
pub mod table_info;
pub mod transaction_by_account;
pub mod translated_v1_event;

use anyhow::ensure;
use aptos_config::config::RocksdbConfig;
use aptos_schemadb::{
    BlockBasedOptions, Cache, ColumnFamilyDescriptor, ColumnFamilyName, DBCompressionType, Options,
    SliceTransform,
};
use aptos_types::transaction::Version;
use std::mem::size_of;

pub const DEFAULT_COLUMN_FAMILY_NAME: ColumnFamilyName = "default";
pub const INDEXER_METADATA_CF_NAME: ColumnFamilyName = "indexer_metadata";
pub const INTERNAL_INDEXER_METADATA_CF_NAME: ColumnFamilyName = "internal_indexer_metadata";
pub const TABLE_INFO_CF_NAME: ColumnFamilyName = "table_info";
pub const EVENT_BY_KEY_CF_NAME: ColumnFamilyName = "event_by_key";
pub const EVENT_BY_TYPE_CF_NAME: ColumnFamilyName = "event_by_type";
pub const EVENT_BY_VERSION_CF_NAME: ColumnFamilyName = "event_by_version";
pub const TRANSACTION_BY_ACCOUNT_CF_NAME: ColumnFamilyName = "transaction_by_account";
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
        EVENT_BY_TYPE_CF_NAME,
        EVENT_BY_VERSION_CF_NAME,
        TRANSACTION_BY_ACCOUNT_CF_NAME,
        STATE_KEYS_CF_NAME,
        TRANSLATED_V1_EVENT_CF_NAME,
        EVENT_SEQUENCE_NUMBER_CF_NAME,
    ]
}

pub fn gen_internal_indexer_cfds(
    rocksdb_config: &RocksdbConfig,
    cfs: Vec<ColumnFamilyName>,
) -> Vec<ColumnFamilyDescriptor> {
    let mut table_options = BlockBasedOptions::default();
    table_options.set_cache_index_and_filter_blocks(rocksdb_config.cache_index_and_filter_blocks);
    table_options.set_block_size(rocksdb_config.block_size as usize);
    let cache = Cache::new_lru_cache(rocksdb_config.block_cache_size as usize);
    table_options.set_block_cache(&cache);
    let mut cfds = Vec::with_capacity(cfs.len());
    for cf_name in cfs {
        let mut cf_opts = Options::default();
        cf_opts.set_compression_type(DBCompressionType::Lz4);
        cf_opts.set_block_based_table_factory(&table_options);
        if cf_name == EVENT_BY_TYPE_CF_NAME {
            let prefix_extractor =
                SliceTransform::create("event_by_type_extractor", event_by_type_extractor, None);
            cf_opts.set_prefix_extractor(prefix_extractor);
        }
        cfds.push(ColumnFamilyDescriptor::new((*cf_name).to_string(), cf_opts));
    }
    cfds
}

fn event_by_type_extractor(raw_key: &[u8]) -> &[u8] {
    &raw_key[..(raw_key.len() - size_of::<Version>() - size_of::<event_by_type::Index>())]
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
