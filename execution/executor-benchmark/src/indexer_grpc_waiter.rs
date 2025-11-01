// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_db_indexer::db_v2::IndexerAsyncV2;
use aptos_logger::info;
use aptos_types::transaction::Version;
use std::{sync::Arc, time::{Duration, Instant}};

const INDEXER_GRPC_POLL_INTERVAL_MS: u64 = 100;
const STATUS_LOG_INTERVAL_SECS: u64 = 5;

pub struct IndexerGrpcWaiter {
    indexer_async_v2: Arc<IndexerAsyncV2>,
}

impl IndexerGrpcWaiter {
    pub fn new(indexer_async_v2: Arc<IndexerAsyncV2>) -> Self {
        Self { indexer_async_v2 }
    }

    pub async fn wait_for_version(&self, target_version: Version) {
        info!(
            "Waiting for indexer_grpc to reach target version: {}",
            target_version
        );

        let start_time = Instant::now();
        let mut last_log_time = Instant::now();

        loop {
            let current_version = self.indexer_async_v2.next_version();
            if current_version > target_version {
                info!(
                    "Indexer_grpc reached target version. Current: {}, Target: {}, elapsed: {:.2}s",
                    current_version, target_version, start_time.elapsed().as_secs_f64()
                );
                break;
            }

            // Log status every 5 seconds
            if last_log_time.elapsed().as_secs() >= STATUS_LOG_INTERVAL_SECS {
                let versions_behind = target_version.saturating_sub(current_version);
                let elapsed_secs = start_time.elapsed().as_secs_f64();
                info!(
                    "Indexer_grpc progress: current={}, target={}, behind={}, elapsed={:.2}s",
                    current_version, target_version, versions_behind, elapsed_secs
                );
                last_log_time = Instant::now();
            }

            tokio::time::sleep(Duration::from_millis(INDEXER_GRPC_POLL_INTERVAL_MS)).await;
        }
    }
}
