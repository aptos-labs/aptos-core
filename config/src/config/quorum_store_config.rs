// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::config::MAX_SENDING_BLOCK_TXNS_QUORUM_STORE_OVERRIDE;
use aptos_types::block_info::Round;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct QuorumStoreBackPressureConfig {
    pub backlog_txn_limit_count: u64,
    pub backlog_batch_limit_count: u64,
    pub decrease_duration_ms: u64,
    pub increase_duration_ms: u64,
    pub decrease_fraction: f64,
    pub dynamic_min_txn_per_s: u64,
    pub dynamic_max_txn_per_s: u64,
}

impl Default for QuorumStoreBackPressureConfig {
    fn default() -> QuorumStoreBackPressureConfig {
        QuorumStoreBackPressureConfig {
            // QS will be backpressured if the remaining total txns is more than this number
            backlog_txn_limit_count: MAX_SENDING_BLOCK_TXNS_QUORUM_STORE_OVERRIDE * 4,
            // QS will create batches immediately until this number is reached
            backlog_batch_limit_count: 80,
            decrease_duration_ms: 1000,
            increase_duration_ms: 1000,
            decrease_fraction: 0.5,
            dynamic_min_txn_per_s: 160,
            dynamic_max_txn_per_s: 2000,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct QuorumStoreConfig {
    pub channel_size: usize,
    pub proof_timeout_ms: usize,
    pub batch_request_num_peers: usize,
    pub batch_generation_poll_interval_ms: usize,
    pub batch_generation_max_interval_ms: usize,
    pub end_batch_ms: u64,
    pub max_batch_bytes: usize,
    pub batch_request_timeout_ms: usize,
    /// Used when setting up the expiration time for the batch initation.
    pub batch_expiry_round_gap_when_init: Round,
    /// Batches may have expiry set for batch_expiry_rounds_gap rounds after the
    /// latest committed round, and it will not be cleared from storage for another
    /// so other batch_expiry_grace_rounds rounds, so the peers on the network
    /// can still fetch the data they fall behind (later, they would have to state-sync).
    /// Used when checking the expiration time of the received batch against current logical time to prevent DDoS.
    pub batch_expiry_round_gap_behind_latest_certified: Round,
    pub batch_expiry_round_gap_beyond_latest_certified: Round,
    pub batch_expiry_grace_rounds: Round,
    pub memory_quota: usize,
    pub db_quota: usize,
    pub mempool_txn_pull_max_bytes: u64,
    pub back_pressure: QuorumStoreBackPressureConfig,
    pub num_workers_for_remote_fragments: usize,
}

impl Default for QuorumStoreConfig {
    fn default() -> QuorumStoreConfig {
        QuorumStoreConfig {
            channel_size: 1000,
            proof_timeout_ms: 10000,
            batch_request_num_peers: 2,
            batch_generation_poll_interval_ms: 25,
            batch_generation_max_interval_ms: 250,
            // TODO: This essentially turns fragments off, because there was performance degradation. Needs more investigation.
            end_batch_ms: 10,
            max_batch_bytes: 4 * 1024 * 1024,
            batch_request_timeout_ms: 10000,
            batch_expiry_round_gap_when_init: 100,
            batch_expiry_round_gap_behind_latest_certified: 500,
            batch_expiry_round_gap_beyond_latest_certified: 500,
            batch_expiry_grace_rounds: 5,
            memory_quota: 120_000_000,
            db_quota: 300_000_000,
            mempool_txn_pull_max_bytes: 4 * 1024 * 1024,
            back_pressure: QuorumStoreBackPressureConfig::default(),
            // number of batch coordinators to handle QS Fragment messages, should be >= 1
            num_workers_for_remote_fragments: 10,
        }
    }
}
