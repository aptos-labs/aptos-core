// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::table_info_parser::TableInfoParser;
use aptos_db_indexer_async_v2::counters::{
    IndexerTableInfoStep, DURATION_IN_SECS, LATEST_PROCESSED_VERSION, NUM_TRANSACTIONS_COUNT, SERVICE_TYPE
};
use aptos_api::context::Context;
use aptos_config::config::NodeConfig;
use aptos_logger::{error, info};
use aptos_mempool::MempoolClientSender;
use aptos_moving_average::MovingAverage;
use aptos_storage_interface::DbReaderWriter;
use aptos_types::chain_id::ChainId;
use std::{env, sync::Arc};
use tokio::runtime::Runtime;

pub fn bootstrap(
    config: &NodeConfig,
    chain_id: ChainId,
    db: DbReaderWriter,
    mp_sender: MempoolClientSender,
) -> Option<Runtime> {
    if !config.storage.enable_indexer_async_v2 {
        return None;
    }

    let runtime = aptos_runtimes::spawn_named_runtime("table-info".to_string(), None);

    let node_config = config.clone();
    let parser_task_count = node_config.indexer_grpc.parser_task_count;
    let parser_batch_size = node_config.indexer_grpc.parser_batch_size;
    // to ensure that we start on the safe version without gap
    let next_version = db.reader.get_indexer_async_v2_next_version().unwrap();
    let subtraction_value = parser_batch_size as u64 * parser_task_count as u64;
    let start_version = if next_version > subtraction_value {
        next_version - subtraction_value
    } else {
        0
    };

    runtime.spawn(async move {
        let context =
            Arc::new(Context::new(chain_id, db.reader.clone(), mp_sender, node_config));
        let mut ma = MovingAverage::new(10_000);
        let mut base: u64 = 0;
        let db_writer = db.writer.clone();
        let mut parser =
            TableInfoParser::new(
                context,
                start_version,
                parser_task_count,
                parser_batch_size,
            );
        loop {
            let start_time = std::time::Instant::now();
            let results = parser.process_next_batch(db_writer.clone()).await;
            let max_version =
            match TableInfoParser::get_max_batch_version(results) {
                Ok(max_version) => max_version,
                Err(e) => {
                    error!(
                        "[table-info] Error getting the max batch version processed: {}",
                        e
                    );
                    break;
                },
            };
            LATEST_PROCESSED_VERSION
                .with_label_values(&[
                    SERVICE_TYPE,
                    IndexerTableInfoStep::TableInfoProcessed.get_step(),
                    IndexerTableInfoStep::TableInfoProcessed.get_label(),
                ])
                .set(max_version as i64);
            let processed_versions = max_version - parser.current_version + 1;
            NUM_TRANSACTIONS_COUNT
                .with_label_values(&[
                    SERVICE_TYPE,
                    IndexerTableInfoStep::TableInfoProcessed.get_step(),
                    IndexerTableInfoStep::TableInfoProcessed.get_label(),
                ])
                .set(processed_versions as i64);
            let duration = start_time.elapsed();
            DURATION_IN_SECS
                .with_label_values(&[
                    SERVICE_TYPE,
                    IndexerTableInfoStep::TableInfoProcessed.get_step(),
                    IndexerTableInfoStep::TableInfoProcessed.get_label(),
                ])
                .set(duration.as_secs_f64());

            let parse_millis = duration.as_millis();
            let new_base: u64 = ma.sum() / (1000_u64);
            
            ma.tick_now(processed_versions);
            if base != new_base {
                base = new_base;

                info!(
                    parse_millis_per_loop = parse_millis,
                    batch_start_version = parser.current_version,
                    batch_end_version = max_version,
                    versions_processed = ma.sum(),
                    tps = (ma.avg() * 1000.0) as u64,
                    "[table-info] Table info processed successfully"
                );
            }
            parser.current_version = max_version + 1;
        }
    });
    Some(runtime)
}
