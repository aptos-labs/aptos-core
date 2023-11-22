// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::table_info_parser::TableInfoParser;
use aptos_api::context::Context;
use aptos_config::config::NodeConfig;
use aptos_db_indexer_async_v2::{
    backup_restore_operator::gcs::GcsBackupRestoreOperator, db::INDEX_ASYNC_V2_DB_NAME,
};
use aptos_logger::{error, info};
use aptos_mempool::MempoolClientSender;
use aptos_moving_average::MovingAverage;
use aptos_storage_interface::DbReaderWriter;
use aptos_types::chain_id::{ChainId, NamedChain};
use std::{env, sync::Arc};
use tokio::runtime::Runtime;

pub fn bootstrap(
    config: &NodeConfig,
    chain_id: ChainId,
    db: DbReaderWriter,
    mp_sender: MempoolClientSender,
) -> Option<Runtime> {
    let runtime = aptos_runtimes::spawn_named_runtime("table-info".to_string(), None);

    let node_config = config.clone();
    let parser_task_count = node_config.indexer_grpc.parser_task_count;
    let parser_batch_size = node_config.indexer_grpc.parser_batch_size;
    let named_chain =
        match NamedChain::from_chain_id(&chain_id) {
            Ok(named_chain) => format!("{}", named_chain).to_lowercase(),
            Err(_err) => {
                info!("Getting chain name from not named chains");
                chain_id.id().to_string()
            },
        };
    let backup_restore_operator = Arc::new(GcsBackupRestoreOperator::new(
        node_config.indexer_grpc.backup_restore_bucket_name.clone() + "-" + &named_chain,
    ));

    let mut epoch: u64 = 0;
    let mut start_version: u64 = 0;

    runtime.block_on(async {
        backup_restore_operator
            .verify_storage_bucket_existence()
            .await;
        let db_path = node_config
            .storage
            .get_dir_paths()
            .default_root_path()
            .join(INDEX_ASYNC_V2_DB_NAME);
        backup_restore_operator
            .restore_snapshot(chain_id.id() as u64, db_path.clone())
            .await
            .expect("Failed to restore snapshot");
        let metadata = backup_restore_operator
            .create_default_metadata_if_absent(chain_id.id().into())
            .await
            .unwrap();
        epoch = metadata.epoch;
        start_version = metadata.version;
    });

    runtime.spawn(async move {
        let context =
            Arc::new(Context::new(chain_id, db.reader.clone(), mp_sender, node_config));

        let mut ma = MovingAverage::new(10_000);

        let mut base: u64 = 0;

        let db_writer = db.writer.clone();
        let mut parser =
            TableInfoParser::new(context, start_version, parser_task_count, parser_batch_size);
        loop {
            let start_millis = chrono::Utc::now().naive_utc();
            let results = parser
                .process_next_batch(db_writer.clone(), backup_restore_operator.clone())
                .await;
            let parse_millis = (chrono::Utc::now().naive_utc() - start_millis).num_milliseconds();
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
            let new_base: u64 = ma.sum() / (1000_u64);
            ma.tick_now(max_version - parser.current_version + 1);
            if base != new_base {
                base = new_base;

                info!(
                    parse_millis = parse_millis,
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
