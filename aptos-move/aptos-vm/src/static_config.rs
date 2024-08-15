// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use num_cpus;
use once_cell::sync::OnceCell;
use std::cmp::{max, min};

static EXECUTION_CONCURRENCY_LEVEL: OnceCell<usize> = OnceCell::new();
static EXECUTION_COMMITTER_BACKUP_ENABLED: OnceCell<bool> = OnceCell::new();
static NUM_EXECUTION_SHARD: OnceCell<usize> = OnceCell::new();
static NUM_PROOF_READING_THREADS: OnceCell<usize> = OnceCell::new();
static DISCARD_FAILED_BLOCKS: OnceCell<bool> = OnceCell::new();
static PROCESSED_TRANSACTIONS_DETAILED_COUNTERS: OnceCell<bool> = OnceCell::new();

pub struct AptosVMStaticConfig {}

impl AptosVMStaticConfig {
    /// Sets execution concurrency level when invoked the first time.
    pub fn set_concurrency_level_once(mut concurrency_level: usize) {
        concurrency_level = min(concurrency_level, num_cpus::get());
        // Only the first call succeeds, due to OnceCell semantics.
        EXECUTION_CONCURRENCY_LEVEL.set(concurrency_level).ok();
    }

    /// Get the concurrency level if already set, otherwise return default 1
    /// (sequential execution).
    ///
    /// The concurrency level is fixed to 1 if gas profiling is enabled.
    pub fn get_concurrency_level() -> usize {
        match EXECUTION_CONCURRENCY_LEVEL.get() {
            Some(concurrency_level) => *concurrency_level,
            None => 1,
        }
    }

    pub fn set_committer_backup_enabled_once(enable_committer_backup: bool) {
        // Only the first call succeeds, due to OnceCell semantics.
        EXECUTION_COMMITTER_BACKUP_ENABLED
            .set(enable_committer_backup)
            .ok();
    }

    /// Returns whether committer backup behavior is enabled, true (enabled)
    /// if the flag has not been explicitly set.
    pub fn get_committer_backup_enabled() -> bool {
        match EXECUTION_COMMITTER_BACKUP_ENABLED.get() {
            Some(committer_backup_enabled) => *committer_backup_enabled,
            None => true,
        }
    }

    pub fn set_block_stm_profiling_enabled_once(enable_block_stm_profiling: bool) {
        // Only the first call succeeds, due to OnceCell semantics.
        EXECUTION_COMMITTER_BACKUP_ENABLED
            .set(enable_block_stm_profiling)
            .ok();
    }

    /// Returns whether block stm profiling is enabled, false (disabled)
    /// if the flag has not been explicitly set.
    pub fn get_block_stm_profiling_enabled() -> bool {
        match EXECUTION_COMMITTER_BACKUP_ENABLED.get() {
            Some(committer_backup_enabled) => *committer_backup_enabled,
            None => true,
        }
    }

    pub fn set_num_shards_once(mut num_shards: usize) {
        num_shards = max(num_shards, 1);
        // Only the first call succeeds, due to OnceCell semantics.
        NUM_EXECUTION_SHARD.set(num_shards).ok();
    }

    pub fn get_num_shards() -> usize {
        match NUM_EXECUTION_SHARD.get() {
            Some(num_shards) => *num_shards,
            None => 1,
        }
    }

    /// Sets runtime config when invoked the first time.
    pub fn set_discard_failed_blocks(enable: bool) {
        // Only the first call succeeds, due to OnceCell semantics.
        DISCARD_FAILED_BLOCKS.set(enable).ok();
    }

    /// Get the discard failed blocks flag if already set, otherwise return default (false)
    pub fn get_discard_failed_blocks() -> bool {
        match DISCARD_FAILED_BLOCKS.get() {
            Some(enable) => *enable,
            None => false,
        }
    }

    /// Sets the # of async proof reading threads.
    pub fn set_num_proof_reading_threads_once(mut num_threads: usize) {
        // TODO(grao): Do more analysis to tune this magic number.
        num_threads = min(num_threads, 256);
        // Only the first call succeeds, due to OnceCell semantics.
        NUM_PROOF_READING_THREADS.set(num_threads).ok();
    }

    /// Returns the # of async proof reading threads if already set, otherwise return default value
    /// (32).
    pub fn get_num_proof_reading_threads() -> usize {
        match NUM_PROOF_READING_THREADS.get() {
            Some(num_threads) => *num_threads,
            None => 32,
        }
    }

    /// Sets additional details in counters when invoked the first time.
    pub fn set_processed_transactions_detailed_counters() {
        // Only the first call succeeds, due to OnceCell semantics.
        PROCESSED_TRANSACTIONS_DETAILED_COUNTERS.set(true).ok();
    }

    /// Get whether we should capture additional details in counters
    pub fn get_processed_transactions_detailed_counters() -> bool {
        match PROCESSED_TRANSACTIONS_DETAILED_COUNTERS.get() {
            Some(value) => *value,
            None => false,
        }
    }
}
