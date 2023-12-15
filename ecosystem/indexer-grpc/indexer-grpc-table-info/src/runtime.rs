// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::table_info_service::TableInfoService;
use aptos_api::context::Context;
use aptos_config::config::NodeConfig;
use aptos_mempool::MempoolClientSender;
use aptos_storage_interface::DbReaderWriter;
use aptos_types::chain_id::ChainId;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Creates a runtime which creates a thread pool which sets up fullnode indexer table info service
/// Returns corresponding Tokio runtime
pub fn bootstrap(
    config: &NodeConfig,
    chain_id: ChainId,
    db: DbReaderWriter,
    mp_sender: MempoolClientSender,
) -> Option<Runtime> {
    if !config.indexer_table_info.enabled {
        return None;
    }

    let runtime = aptos_runtimes::spawn_named_runtime("table-info".to_string(), None);

    let node_config = config.clone();
    let parser_task_count = node_config.indexer_table_info.parser_task_count;
    let parser_batch_size = node_config.indexer_table_info.parser_batch_size;
    let enable_expensive_logging = node_config.indexer_table_info.enable_expensive_logging;
    let next_version = db.reader.get_indexer_async_v2_next_version().unwrap();

    // Spawn the runtime for table info parsing
    runtime.spawn(async move {
        let context = Arc::new(Context::new(
            chain_id,
            db.reader.clone(),
            mp_sender,
            node_config,
        ));
        let mut parser = TableInfoService::new(
            context,
            next_version,
            parser_task_count,
            parser_batch_size,
            enable_expensive_logging,
        );
        parser.run(db.clone()).await
    });
    Some(runtime)
}
