// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::schema::*;
use aptos_config::config::RocksdbConfig;
use aptos_schemadb::{
    BlockBasedOptions, Cache, ColumnFamilyDescriptor, ColumnFamilyName, DBCompressionType, Options,
    SliceTransform, DEFAULT_COLUMN_FAMILY_NAME,
};
use aptos_types::transaction::Version;

const VERSION_SIZE: usize = std::mem::size_of::<Version>();

pub(super) fn ledger_db_column_families() -> Vec<ColumnFamilyName> {
    vec![
        /* empty cf */ DEFAULT_COLUMN_FAMILY_NAME,
        BLOCK_BY_VERSION_CF_NAME,
        BLOCK_INFO_CF_NAME,
        EPOCH_BY_VERSION_CF_NAME,
        EVENT_ACCUMULATOR_CF_NAME,
        EVENT_BY_KEY_CF_NAME,
        EVENT_BY_VERSION_CF_NAME,
        EVENT_CF_NAME,
        LEDGER_INFO_CF_NAME,
        STALE_STATE_VALUE_INDEX_CF_NAME,
        STATE_VALUE_CF_NAME,
        TRANSACTION_CF_NAME,
        TRANSACTION_ACCUMULATOR_CF_NAME,
        TRANSACTION_ACCUMULATOR_HASH_CF_NAME,
        TRANSACTION_AUXILIARY_DATA_CF_NAME,
        ORDERED_TRANSACTION_BY_ACCOUNT_CF_NAME,
        TRANSACTION_SUMMARIES_BY_ACCOUNT_CF_NAME,
        TRANSACTION_BY_HASH_CF_NAME,
        TRANSACTION_INFO_CF_NAME,
        VERSION_DATA_CF_NAME,
        WRITE_SET_CF_NAME,
        DB_METADATA_CF_NAME,
    ]
}

pub(super) fn event_db_column_families() -> Vec<ColumnFamilyName> {
    vec![
        /* empty cf */ DEFAULT_COLUMN_FAMILY_NAME,
        DB_METADATA_CF_NAME,
        EVENT_ACCUMULATOR_CF_NAME,
        EVENT_BY_KEY_CF_NAME,
        EVENT_BY_VERSION_CF_NAME,
        EVENT_CF_NAME,
    ]
}

pub(super) fn transaction_accumulator_db_column_families() -> Vec<ColumnFamilyName> {
    vec![
        /* empty cf */ DEFAULT_COLUMN_FAMILY_NAME,
        DB_METADATA_CF_NAME,
        TRANSACTION_ACCUMULATOR_CF_NAME,
        TRANSACTION_ACCUMULATOR_HASH_CF_NAME,
    ]
}

pub(super) fn transaction_auxiliary_data_db_column_families() -> Vec<ColumnFamilyName> {
    vec![
        /* empty cf */ DEFAULT_COLUMN_FAMILY_NAME,
        DB_METADATA_CF_NAME,
        TRANSACTION_AUXILIARY_DATA_CF_NAME,
    ]
}

pub(super) fn transaction_db_column_families() -> Vec<ColumnFamilyName> {
    vec![
        /* empty cf */ DEFAULT_COLUMN_FAMILY_NAME,
        DB_METADATA_CF_NAME,
        TRANSACTION_CF_NAME,
        ORDERED_TRANSACTION_BY_ACCOUNT_CF_NAME,
        TRANSACTION_SUMMARIES_BY_ACCOUNT_CF_NAME,
        TRANSACTION_BY_HASH_CF_NAME,
    ]
}

pub(super) fn transaction_info_db_column_families() -> Vec<ColumnFamilyName> {
    vec![
        /* empty cf */ DEFAULT_COLUMN_FAMILY_NAME,
        DB_METADATA_CF_NAME,
        TRANSACTION_INFO_CF_NAME,
    ]
}

pub(super) fn write_set_db_column_families() -> Vec<ColumnFamilyName> {
    vec![
        /* empty cf */ DEFAULT_COLUMN_FAMILY_NAME,
        DB_METADATA_CF_NAME,
        WRITE_SET_CF_NAME,
    ]
}

pub(super) fn ledger_metadata_db_column_families() -> Vec<ColumnFamilyName> {
    vec![
        /* empty cf */ DEFAULT_COLUMN_FAMILY_NAME,
        BLOCK_BY_VERSION_CF_NAME,
        BLOCK_INFO_CF_NAME,
        DB_METADATA_CF_NAME,
        EPOCH_BY_VERSION_CF_NAME,
        LEDGER_INFO_CF_NAME,
        VERSION_DATA_CF_NAME,
    ]
}

pub(super) fn state_merkle_db_column_families() -> Vec<ColumnFamilyName> {
    vec![
        /* empty cf */ DEFAULT_COLUMN_FAMILY_NAME,
        DB_METADATA_CF_NAME,
        JELLYFISH_MERKLE_NODE_CF_NAME,
        STALE_NODE_INDEX_CF_NAME,
        STALE_NODE_INDEX_CROSS_EPOCH_CF_NAME,
    ]
}

pub(super) fn skip_reporting_cf(cf_name: &str) -> bool {
    cf_name == DEFAULT_COLUMN_FAMILY_NAME || cf_name == DB_METADATA_CF_NAME
}

pub(super) fn state_kv_db_column_families() -> Vec<ColumnFamilyName> {
    vec![
        /* empty cf */ DEFAULT_COLUMN_FAMILY_NAME,
        DB_METADATA_CF_NAME,
        STALE_STATE_VALUE_INDEX_CF_NAME,
        STATE_VALUE_CF_NAME,
        STATE_VALUE_INDEX_CF_NAME,
    ]
}

pub(super) fn state_kv_db_new_key_column_families() -> Vec<ColumnFamilyName> {
    vec![
        /* empty cf */ DEFAULT_COLUMN_FAMILY_NAME,
        DB_METADATA_CF_NAME,
        STALE_STATE_VALUE_INDEX_BY_KEY_HASH_CF_NAME,
        STATE_VALUE_BY_KEY_HASH_CF_NAME,
        STATE_VALUE_INDEX_CF_NAME, // we still need this cf before deleting all the write callsites
    ]
}

fn gen_cfds<F>(
    rocksdb_config: &RocksdbConfig,
    cfs: Vec<ColumnFamilyName>,
    cf_opts_post_processor: F,
) -> Vec<ColumnFamilyDescriptor>
where
    F: Fn(ColumnFamilyName, &mut Options),
{
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
        cf_opts.add_compact_on_deletion_collector_factory(0, 0, 0.4);
        cf_opts_post_processor(cf_name, &mut cf_opts);
        cfds.push(ColumnFamilyDescriptor::new((*cf_name).to_string(), cf_opts));
    }
    cfds
}

fn with_state_key_extractor_processor(cf_name: ColumnFamilyName, cf_opts: &mut Options) {
    if cf_name == STATE_VALUE_CF_NAME || cf_name == STATE_VALUE_BY_KEY_HASH_CF_NAME {
        let prefix_extractor =
            SliceTransform::create("state_key_extractor", state_key_extractor, None);
        cf_opts.set_prefix_extractor(prefix_extractor);
    }
}

pub(super) fn gen_event_cfds(rocksdb_config: &RocksdbConfig) -> Vec<ColumnFamilyDescriptor> {
    let cfs = event_db_column_families();
    gen_cfds(rocksdb_config, cfs, |_, _| {})
}

pub(super) fn gen_transaction_accumulator_cfds(
    rocksdb_config: &RocksdbConfig,
) -> Vec<ColumnFamilyDescriptor> {
    let cfs = transaction_accumulator_db_column_families();
    gen_cfds(rocksdb_config, cfs, |_, _| {})
}

pub(super) fn gen_transaction_auxiliary_data_cfds(
    rocksdb_config: &RocksdbConfig,
) -> Vec<ColumnFamilyDescriptor> {
    let cfs = transaction_auxiliary_data_db_column_families();
    gen_cfds(rocksdb_config, cfs, |_, _| {})
}
pub(super) fn gen_transaction_cfds(rocksdb_config: &RocksdbConfig) -> Vec<ColumnFamilyDescriptor> {
    let cfs = transaction_db_column_families();
    gen_cfds(rocksdb_config, cfs, |_, _| {})
}

pub(super) fn gen_transaction_info_cfds(
    rocksdb_config: &RocksdbConfig,
) -> Vec<ColumnFamilyDescriptor> {
    let cfs = transaction_info_db_column_families();
    gen_cfds(rocksdb_config, cfs, |_, _| {})
}

pub(super) fn gen_write_set_cfds(rocksdb_config: &RocksdbConfig) -> Vec<ColumnFamilyDescriptor> {
    let cfs = write_set_db_column_families();
    gen_cfds(rocksdb_config, cfs, |_, _| {})
}

pub(super) fn gen_ledger_metadata_cfds(
    rocksdb_config: &RocksdbConfig,
) -> Vec<ColumnFamilyDescriptor> {
    let cfs = ledger_metadata_db_column_families();
    gen_cfds(rocksdb_config, cfs, |_, _| {})
}

pub(super) fn gen_ledger_cfds(rocksdb_config: &RocksdbConfig) -> Vec<ColumnFamilyDescriptor> {
    let cfs = ledger_db_column_families();
    gen_cfds(rocksdb_config, cfs, with_state_key_extractor_processor)
}

pub(super) fn gen_state_merkle_cfds(rocksdb_config: &RocksdbConfig) -> Vec<ColumnFamilyDescriptor> {
    let cfs = state_merkle_db_column_families();
    gen_cfds(rocksdb_config, cfs, |_, _| {})
}

pub(super) fn gen_state_kv_shard_cfds(
    rocksdb_config: &RocksdbConfig,
) -> Vec<ColumnFamilyDescriptor> {
    let cfs = state_kv_db_new_key_column_families();
    gen_cfds(rocksdb_config, cfs, with_state_key_extractor_processor)
}

fn state_key_extractor(state_value_raw_key: &[u8]) -> &[u8] {
    &state_value_raw_key[..(state_value_raw_key.len() - VERSION_SIZE)]
}
