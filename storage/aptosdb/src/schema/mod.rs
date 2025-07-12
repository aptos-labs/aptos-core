// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines representation of Aptos core data structures at physical level via schemas
//! that implement [`aptos_schemadb::schema::Schema`].
//!
//! All schemas are `pub(crate)` so not shown in rustdoc, refer to the source code to see details.

pub(crate) mod block_by_version;
pub(crate) mod block_info;
pub(crate) mod db_metadata;
pub(crate) mod epoch_by_version;
pub(crate) mod event;
pub(crate) mod event_accumulator;
pub(crate) mod hot_state_value_by_key_hash;
pub(crate) mod jellyfish_merkle_node;
pub(crate) mod ledger_info;
pub(crate) mod persisted_auxiliary_info;
pub(crate) mod stale_node_index;
pub(crate) mod stale_node_index_cross_epoch;
pub(crate) mod stale_state_value_index;
pub(crate) mod stale_state_value_index_by_key_hash;
pub(crate) mod state_value;
pub(crate) mod state_value_by_key_hash;
pub(crate) mod transaction;
pub(crate) mod transaction_accumulator;
pub(crate) mod transaction_accumulator_root_hash;
pub(crate) mod transaction_auxiliary_data;
pub(crate) mod transaction_by_hash;
pub(crate) mod transaction_info;
pub(crate) mod transaction_summaries_by_account;
pub(crate) mod version_data;
pub(crate) mod write_set;

use anyhow::{ensure, Result};
use aptos_schemadb::ColumnFamilyName;

pub const BLOCK_BY_VERSION_CF_NAME: ColumnFamilyName = "block_by_version";
pub const BLOCK_INFO_CF_NAME: ColumnFamilyName = "block_info";
pub const DB_METADATA_CF_NAME: ColumnFamilyName = "db_metadata";
pub const EPOCH_BY_VERSION_CF_NAME: ColumnFamilyName = "epoch_by_version";
pub const EVENT_ACCUMULATOR_CF_NAME: ColumnFamilyName = "event_accumulator";
pub const EVENT_BY_KEY_CF_NAME: ColumnFamilyName = "event_by_key";
pub const EVENT_BY_VERSION_CF_NAME: ColumnFamilyName = "event_by_version";
pub const EVENT_CF_NAME: ColumnFamilyName = "event";
pub const HOT_STATE_VALUE_BY_KEY_HASH_CF_NAME: ColumnFamilyName = "hot_state_value_by_key_hash";
pub const JELLYFISH_MERKLE_NODE_CF_NAME: ColumnFamilyName = "jellyfish_merkle_node";
pub const LEDGER_INFO_CF_NAME: ColumnFamilyName = "ledger_info";
pub const PERSISTED_AUXILIARY_INFO_CF_NAME: ColumnFamilyName = "persisted_auxiliary_info";
pub const STALE_NODE_INDEX_CF_NAME: ColumnFamilyName = "stale_node_index";
pub const STALE_NODE_INDEX_CROSS_EPOCH_CF_NAME: ColumnFamilyName = "stale_node_index_cross_epoch";
pub const STALE_STATE_VALUE_INDEX_CF_NAME: ColumnFamilyName = "stale_state_value_index";
pub const STALE_STATE_VALUE_INDEX_BY_KEY_HASH_CF_NAME: ColumnFamilyName =
    "stale_state_value_index_by_key_hash";
pub const STATE_VALUE_CF_NAME: ColumnFamilyName = "state_value";
pub const STATE_VALUE_BY_KEY_HASH_CF_NAME: ColumnFamilyName = "state_value_by_key_hash";
pub const STATE_VALUE_INDEX_CF_NAME: ColumnFamilyName = "state_value_index";
pub const TRANSACTION_CF_NAME: ColumnFamilyName = "transaction";
pub const TRANSACTION_ACCUMULATOR_CF_NAME: ColumnFamilyName = "transaction_accumulator";
pub const TRANSACTION_ACCUMULATOR_HASH_CF_NAME: ColumnFamilyName =
    "transaction_accumulator_root_hash";
pub const TRANSACTION_AUXILIARY_DATA_CF_NAME: ColumnFamilyName = "transaction_auxiliary_data";
pub const ORDERED_TRANSACTION_BY_ACCOUNT_CF_NAME: ColumnFamilyName = "transaction_by_account";
pub const TRANSACTION_SUMMARIES_BY_ACCOUNT_CF_NAME: ColumnFamilyName =
    "transaction_summaries_by_account";
pub const TRANSACTION_BY_HASH_CF_NAME: ColumnFamilyName = "transaction_by_hash";
pub const TRANSACTION_INFO_CF_NAME: ColumnFamilyName = "transaction_info";
pub const VERSION_DATA_CF_NAME: ColumnFamilyName = "version_data";
pub const WRITE_SET_CF_NAME: ColumnFamilyName = "write_set";

fn ensure_slice_len_eq(data: &[u8], len: usize) -> Result<()> {
    ensure!(
        data.len() == len,
        "Unexpected data len {}, expected {}.",
        data.len(),
        len,
    );
    Ok(())
}

fn ensure_slice_len_gt(data: &[u8], len: usize) -> Result<()> {
    ensure!(
        data.len() > len,
        "Unexpected data len {}, expected to be greater than {}.",
        data.len(),
        len,
    );
    Ok(())
}

#[cfg(feature = "fuzzing")]
pub mod fuzzing {
    use aptos_schemadb::schema::fuzzing::assert_no_panic_decoding;

    pub fn fuzz_decode(data: &[u8]) {
        #[allow(unused_must_use)]
        {
            assert_no_panic_decoding::<super::block_by_version::BlockByVersionSchema>(data);
            assert_no_panic_decoding::<super::block_info::BlockInfoSchema>(data);
            assert_no_panic_decoding::<super::epoch_by_version::EpochByVersionSchema>(data);
            assert_no_panic_decoding::<super::event::EventSchema>(data);
            assert_no_panic_decoding::<super::event_accumulator::EventAccumulatorSchema>(data);
            assert_no_panic_decoding::<super::jellyfish_merkle_node::JellyfishMerkleNodeSchema>(
                data,
            );
            assert_no_panic_decoding::<super::ledger_info::LedgerInfoSchema>(data);
            assert_no_panic_decoding::<super::db_metadata::DbMetadataSchema>(data);
            assert_no_panic_decoding::<super::persisted_auxiliary_info::PersistedAuxiliaryInfoSchema>(
                data,
            );
            assert_no_panic_decoding::<super::stale_node_index::StaleNodeIndexSchema>(data);
            assert_no_panic_decoding::<
                super::stale_node_index_cross_epoch::StaleNodeIndexCrossEpochSchema,
            >(data);
            assert_no_panic_decoding::<
                super::stale_state_value_index_by_key_hash::StaleStateValueIndexByKeyHashSchema,
            >(data);
            assert_no_panic_decoding::<super::stale_state_value_index::StaleStateValueIndexSchema>(
                data,
            );
            assert_no_panic_decoding::<super::state_value::StateValueSchema>(data);
            assert_no_panic_decoding::<super::state_value_by_key_hash::StateValueByKeyHashSchema>(
                data,
            );
            assert_no_panic_decoding::<super::transaction::TransactionSchema>(data);
            assert_no_panic_decoding::<super::transaction_accumulator::TransactionAccumulatorSchema>(
                data,
            );
            assert_no_panic_decoding::<
                super::transaction_accumulator_root_hash::TransactionAccumulatorRootHashSchema,
            >(data);
            assert_no_panic_decoding::<
                super::transaction_auxiliary_data::TransactionAuxiliaryDataSchema,
            >(data);
            assert_no_panic_decoding::<super::transaction_by_hash::TransactionByHashSchema>(data);
            assert_no_panic_decoding::<super::transaction_info::TransactionInfoSchema>(data);
            assert_no_panic_decoding::<super::version_data::VersionDataSchema>(data);
            assert_no_panic_decoding::<super::write_set::WriteSetSchema>(data);
        }
    }
}
