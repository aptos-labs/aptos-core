// Copyright © Aptos Foundation

use crate::{
    get_anchor_shard_id,
    v2::{
        conflicting_txn_tracker::ConflictingTxnTracker, types::OriginalTxnIdx, PartitionState,
        PartitionerV2,
    },
};
use rayon::{iter::ParallelIterator, prelude::IntoParallelIterator};
use std::sync::RwLock;

impl PartitionerV2 {
    pub(crate) fn init(session: &mut PartitionState) {
        session.thread_pool.install(|| {
            (0..session.num_txns())
                .into_par_iter()
                .for_each(|txn_idx: OriginalTxnIdx| {
                    let txn_ref = session.txns[txn_idx].read().unwrap();
                    let txn = txn_ref.as_ref().unwrap();
                    let sender_idx = session.add_sender(txn.sender());
                    *session.senders[txn_idx].write().unwrap() = Some(sender_idx);

                    let reads = txn.read_hints.iter().map(|loc| (loc, false));
                    let writes = txn.write_hints.iter().map(|loc| (loc, true));
                    reads
                        .chain(writes)
                        .for_each(|(storage_location, is_write)| {
                            let key_idx = session.add_key(storage_location.state_key());
                            if is_write {
                                session.wsets[txn_idx].write().unwrap().insert(key_idx);
                            } else {
                                session.rsets[txn_idx].write().unwrap().insert(key_idx);
                            }
                            let tracker_ref =
                                session.trackers.entry(key_idx).or_insert_with(|| {
                                    let anchor_shard_id = get_anchor_shard_id(
                                        storage_location,
                                        session.num_executor_shards,
                                    );
                                    RwLock::new(ConflictingTxnTracker::new(
                                        storage_location.clone(),
                                        anchor_shard_id,
                                    ))
                                });
                            let mut tracker = tracker_ref.write().unwrap();
                            if is_write {
                                tracker.add_write_candidate(txn_idx);
                            } else {
                                tracker.add_read_candidate(txn_idx);
                            }
                        });
                });
        });
    }
}
