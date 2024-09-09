// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

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

pub mod db_options;
pub mod event_store;
pub mod ledger_db;
pub mod lru_node_cache;
pub mod pruner;
pub mod state_kv_db;
pub mod state_merkle_db;
pub mod state_store;
pub mod transaction_store;
pub mod versioned_node_cache;
