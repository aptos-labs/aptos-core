// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::block_info::Round;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct QuorumStoreConfig {
    pub channel_size: usize,
    pub proof_timeout_ms: usize,
    pub batch_request_num_peers: usize,
    pub mempool_pulling_interval: usize,
    pub end_batch_ms: u64,
    pub max_batch_counts: usize,
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
    pub mempool_txn_pull_max_count: u64,
    pub mempool_txn_pull_max_bytes: u64,
    pub back_pressure_local_batch_num: usize,
    pub num_workers_for_remote_fragments: usize,
}

impl Default for QuorumStoreConfig {
    fn default() -> QuorumStoreConfig {
        QuorumStoreConfig {
            channel_size: 1000,
            proof_timeout_ms: 10000,
            batch_request_num_peers: 2,
            mempool_pulling_interval: 100,
            end_batch_ms: 500,
            max_batch_counts: 300,
            max_batch_bytes: 1000000,
            batch_request_timeout_ms: 10000,
            batch_expiry_round_gap_when_init: 100,
            batch_expiry_round_gap_behind_latest_certified: 500,
            batch_expiry_round_gap_beyond_latest_certified: 500,
            batch_expiry_grace_rounds: 5,
            memory_quota: 100000000,
            db_quota: 10000000000,
            mempool_txn_pull_max_count: 300,
            mempool_txn_pull_max_bytes: 1000000,
            // QS will be backpressured if the remaining local batches is more than this number
            back_pressure_local_batch_num: 10,
            // number of batch coordinators to handle QS Fragment messages, should be >= 1
            num_workers_for_remote_fragments: 2,
        }
    }
}
