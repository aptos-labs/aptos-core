// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_indexer_grpc_table_info::table_info_service::TableInfoService;
use aptos_logger::info;
use aptos_types::transaction::Version;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

const INDEXER_GRPC_POLL_INTERVAL_MS: u64 = 50;
const STATUS_LOG_INTERVAL_SECS: u64 = 1;

pub struct IndexerGrpcWaiter {
    table_info_service: Arc<TableInfoService>,
    stream_version: Arc<AtomicU64>,
}

impl Drop for IndexerGrpcWaiter {
    fn drop(&mut self) {
        println!("**** Dropping IndexerGrpcWaiter. ****");
    }
}

impl IndexerGrpcWaiter {
    pub fn new(table_info_service: Arc<TableInfoService>, stream_version: Arc<AtomicU64>) -> Self {
        Self {
            table_info_service,
            stream_version,
        }
    }

    pub async fn wait_for_version(&self, target_version: Version, abort_on_finish: bool) {
        info!(
            "Waiting for indexer_grpc to reach target version: {}",
            target_version
        );

        let start_time = Instant::now();
        let mut last_log_time = Instant::now();

        loop {
            let table_info_version = self.table_info_service.next_version().saturating_sub(1);
            let stream_version = self.stream_version.load(Ordering::SeqCst);
            if stream_version >= target_version {
                info!(
                    "Indexer stream reached target version. Current: {}, Target: {}, elapsed: {:.2}s",
                    stream_version,
                    target_version,
                    start_time.elapsed().as_secs_f64()
                );
                if abort_on_finish {
                    self.table_info_service.abort();
                }
                break;
            }

            // Log status every 1 second
            if last_log_time.elapsed().as_secs() >= STATUS_LOG_INTERVAL_SECS {
                let versions_behind = target_version.saturating_sub(stream_version);
                let elapsed_secs = start_time.elapsed().as_secs_f64();
                info!(
                    "Indexer_grpc progress: target={}, table_info_current={}, stream_version={}, behind={}, elapsed={:.2}s",
                    target_version, table_info_version, stream_version, versions_behind, elapsed_secs
                );
                last_log_time = Instant::now();
            }

            tokio::time::sleep(Duration::from_millis(INDEXER_GRPC_POLL_INTERVAL_MS)).await;
        }
    }
}
