// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_db_indexer::table_info_service::TableInfoService;
use aptos_logger::info;
use aptos_types::transaction::Version;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

const TABLE_INFO_POLL_INTERVAL_MS: u64 = 50;
const STATUS_LOG_INTERVAL_SECS: u64 = 1;

pub struct TableInfoWaiter {
    table_info_service: Arc<TableInfoService>,
}

impl TableInfoWaiter {
    pub fn new(table_info_service: Arc<TableInfoService>) -> Self {
        Self { table_info_service }
    }

    pub async fn wait_for_version(&self, target_version: Version, abort_on_finish: bool) {
        info!(
            "Waiting for table info service to reach target version: {}",
            target_version
        );

        let start_time = Instant::now();
        let mut last_log_time = Instant::now();

        loop {
            let table_info_version = self.table_info_service.next_version().saturating_sub(1);
            if table_info_version >= target_version {
                info!(
                    "Table info service reached target version. Current: {}, Target: {}, elapsed: {:.2}s",
                    table_info_version,
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
                let versions_behind = target_version.saturating_sub(table_info_version);
                let elapsed_secs = start_time.elapsed().as_secs_f64();
                info!(
                    "Table info progress: target={}, current={}, behind={}, elapsed={:.2}s",
                    target_version, table_info_version, versions_behind, elapsed_secs
                );
                last_log_time = Instant::now();
            }

            tokio::time::sleep(Duration::from_millis(TABLE_INFO_POLL_INTERVAL_MS)).await;
        }
    }
}
