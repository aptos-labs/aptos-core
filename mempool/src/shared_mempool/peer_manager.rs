// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use network::transport::ConnectionMetadata;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::SystemTime,
};

/// State of last sync with peer:
/// `timeline_id` is position in log of ready transactions
/// `is_alive` - is connection healthy
#[derive(Clone, Debug)]
pub(crate) struct PeerSyncState {
    pub timeline_id: u64,
    pub is_alive: bool,
    pub broadcast_info: BroadcastInfo,
    pub metadata: ConnectionMetadata,
}

impl PeerSyncState {
    pub fn new(metadata: ConnectionMetadata) -> Self {
        PeerSyncState {
            timeline_id: 0,
            is_alive: true,
            broadcast_info: BroadcastInfo::new(),
            metadata,
        }
    }
}

/// Identifier for a broadcasted batch of txns.
/// For BatchId(`start_id`, `end_id`), (`start_id`, `end_id`) is the range of timeline IDs read from
/// the core mempool timeline index that produced the txns in this batch.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct BatchId(pub u64, pub u64);

impl PartialOrd for BatchId {
    fn partial_cmp(&self, other: &BatchId) -> Option<std::cmp::Ordering> {
        Some((other.0, other.1).cmp(&(self.0, self.1)))
    }
}

impl Ord for BatchId {
    fn cmp(&self, other: &BatchId) -> std::cmp::Ordering {
        (other.0, other.1).cmp(&(self.0, self.1))
    }
}

/// Txn broadcast-related info for a given remote peer.
#[derive(Clone, Debug)]
pub struct BroadcastInfo {
    // Sent broadcasts that have not yet received an ack.
    pub sent_batches: BTreeMap<BatchId, SystemTime>,
    // Broadcasts that have received a retry ack and are pending a resend.
    pub retry_batches: BTreeSet<BatchId>,
    // Whether broadcasting to this peer is in backoff mode, e.g. broadcasting at longer intervals.
    pub backoff_mode: bool,
}

impl BroadcastInfo {
    fn new() -> Self {
        Self {
            sent_batches: BTreeMap::new(),
            retry_batches: BTreeSet::new(),
            backoff_mode: false,
        }
    }
}
