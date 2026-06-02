// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use crate::{
    common::{run_batch_committer_loop, CommitMessage, MerkleBatch},
    metrics::OTHER_TIMERS_SECONDS,
    position_merkle_db::PositionMerkleDb,
};
use aptos_crypto::HashValue;
use aptos_logger::{info, trace};
use aptos_metrics_core::TimerHelper;
use aptos_types::transaction::Version;
use std::sync::{mpsc::Receiver, Arc};

pub(crate) struct PositionMerkleCommit {
    pub version: Version,
    pub root_hash: HashValue,
    pub batch: MerkleBatch,
}

pub(crate) struct PositionMerkleBatchCommitter {
    merkle_db: Arc<PositionMerkleDb>,
    receiver: Receiver<CommitMessage<PositionMerkleCommit>>,
}

impl PositionMerkleBatchCommitter {
    pub fn new(
        merkle_db: Arc<PositionMerkleDb>,
        receiver: Receiver<CommitMessage<PositionMerkleCommit>>,
    ) -> Self {
        Self {
            merkle_db,
            receiver,
        }
    }

    pub fn run(self) {
        let Self {
            merkle_db,
            receiver,
        } = self;
        run_batch_committer_loop(receiver, |commit| {
            let _timer = OTHER_TIMERS_SECONDS.timer_with(&["position_batch_committer_work"]);
            let PositionMerkleCommit {
                version,
                root_hash,
                batch,
            } = commit;
            merkle_db
                .commit(version, batch.top_levels_batch, batch.batches_for_shards)
                .expect("Position merkle commit failed.");
            info!(
                version = version,
                root_hash = %root_hash,
                "Position merkle snapshot committed."
            );
        });
        trace!("Position merkle batch committing thread exit.");
    }
}
