// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::schema::*;
use aptos_config::config::RocksdbConfig;
use aptos_types::transaction::Version;
use schemadb::{
    BlockBasedOptions, Cache, ColumnFamilyDescriptor, ColumnFamilyName, DBCompressionType, Options,
    SliceTransform, DEFAULT_COLUMN_FAMILY_NAME,
};

const VERSION_SIZE: usize = std::mem::size_of::<Version>();

pub(super) fn ledger_db_column_families() -> Vec<ColumnFamilyName> {
    vec![
        /* empty cf */ DEFAULT_COLUMN_FAMILY_NAME,
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
        TRANSACTION_BY_ACCOUNT_CF_NAME,
        TRANSACTION_BY_HASH_CF_NAME,
        TRANSACTION_INFO_CF_NAME,
        VERSION_DATA_CF_NAME,
        WRITE_SET_CF_NAME,
        DB_METADATA_CF_NAME,
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

pub(super) fn gen_ledger_cfds(rocksdb_config: &RocksdbConfig) -> Vec<ColumnFamilyDescriptor> {
    let cfs = ledger_db_column_families();
    let mut cfds = Vec::with_capacity(cfs.len());
    let mut table_options = BlockBasedOptions::default();
    table_options.set_cache_index_and_filter_blocks(rocksdb_config.cache_index_and_filter_blocks);
    table_options.set_block_size(rocksdb_config.block_size as usize);
    let cache = Cache::new_lru_cache(rocksdb_config.block_cache_size as usize)
        .expect("Create Rocksdb block cache failed.");
    table_options.set_block_cache(&cache);
    for cf_name in cfs {
        let mut cf_opts = Options::default();
        cf_opts.set_compression_type(DBCompressionType::Lz4);
        cf_opts.set_block_based_table_factory(&table_options);
        // set cf options separately
        if cf_name == STATE_VALUE_CF_NAME {
            // TODO(lightmark): Use the defaults for bloom filter for now, will tune later.
            let prefix_extractor =
                SliceTransform::create("state_key_extractor", state_key_extractor, None);
            cf_opts.set_prefix_extractor(prefix_extractor);
        }
        cfds.push(ColumnFamilyDescriptor::new((*cf_name).to_string(), cf_opts));
    }
    cfds
}

pub(super) fn gen_state_merkle_cfds(rocksdb_config: &RocksdbConfig) -> Vec<ColumnFamilyDescriptor> {
    let cfs = state_merkle_db_column_families();
    let mut table_options = BlockBasedOptions::default();
    table_options.set_cache_index_and_filter_blocks(rocksdb_config.cache_index_and_filter_blocks);
    table_options.set_block_size(rocksdb_config.block_size as usize);
    let cache = Cache::new_lru_cache(rocksdb_config.block_cache_size as usize)
        .expect("Create Rocksdb block cache failed.");
    table_options.set_block_cache(&cache);
    let mut cfds = Vec::with_capacity(cfs.len());
    for cf_name in cfs {
        let mut cf_opts = Options::default();
        cf_opts.set_compression_type(DBCompressionType::Lz4);
        cf_opts.set_block_based_table_factory(&table_options);
        cfds.push(ColumnFamilyDescriptor::new((*cf_name).to_string(), cf_opts));
    }
    cfds
}

fn state_key_extractor(state_value_raw_key: &[u8]) -> &[u8] {
    &state_value_raw_key[..(state_value_raw_key.len() - VERSION_SIZE)]
}
