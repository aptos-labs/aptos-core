// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

//! This crate provides [`AptosDB`] which represents physical storage of the core Aptos data
//! structures.
//!
//! It relays read/write operations on the physical storage via `schemadb` to the underlying
//! Key-Value storage system, and implements aptos data structures on top of it.

pub use crate::db::AptosDB;

// Used in this and other crates for testing.

pub mod backup;
pub mod common;
pub mod db;
pub mod get_restore_handler;
pub mod metrics;
pub(crate) mod rocksdb_property_reporter;
pub mod schema;
pub mod state_restore;
pub mod utils;

#[cfg(feature = "db-debugger")]
pub mod db_debugger;
pub mod fast_sync_storage_wrapper;

mod db_options;
mod event_store;
mod ledger_db;
mod lru_node_cache;
pub mod native_state_committer;
pub mod position_buffered_state;
pub mod position_db;
pub(crate) mod position_merkle_batch_committer;
pub mod position_merkle_db;
pub mod position_metrics;
pub(crate) mod position_pruner;
pub(crate) mod position_snapshot_committer;
pub mod position_state_store;
pub mod position_state_sync;
mod pruner;
mod sharded_jmt_merkle_db;
mod sharded_kv_db;
mod trading_native;

#[cfg(test)]
mod native_storage_tests;

pub use native_state_committer::{MerkleLeafUpdate, NativeMerkleLeafUpdates, NativeStateCommitter};
mod state_kv_db;
mod state_merkle_db;
mod state_store;
mod state_value_chunk;
mod transaction_store;
mod versioned_node_cache;
