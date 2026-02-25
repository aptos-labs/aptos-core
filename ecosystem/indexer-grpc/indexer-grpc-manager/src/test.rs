// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::config::{CacheConfig, IndexerGrpcManagerConfig, ServiceConfig};
use aptos_config::utils::get_available_port;
use aptos_indexer_grpc_server_framework::RunnableConfig;
use aptos_indexer_grpc_utils::{
    config::IndexerGrpcFileStoreConfig,
    file_store_operator_v2::common::{FileStoreMetadata, METADATA_FILE_NAME},
};
use std::{net::SocketAddr, path::PathBuf, time::Duration};

#[tokio::test(flavor = "multi_thread", worker_threads = 16)]
async fn test_run() {
    let port = get_available_port();
    let listen_address: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let self_address = listen_address.to_string();
    let file_store_config = IndexerGrpcFileStoreConfig::default();
    let metadata = FileStoreMetadata {
        chain_id: 0,
        num_transactions_per_folder: 100000,
        version: 0,
    };
    let raw_data = serde_json::to_vec(&metadata).unwrap();
    let file_store = file_store_config.clone().create_filestore().await;
    file_store
        .save_raw_file(PathBuf::from(METADATA_FILE_NAME), raw_data)
        .await
        .unwrap();

    let config = IndexerGrpcManagerConfig {
        chain_id: 0,
        service_config: ServiceConfig { listen_address },
        cache_config: CacheConfig {
            max_cache_size: 5 * (1 << 30),
            target_cache_size: 4 * (1 << 30),
        },
        file_store_config,
        self_advertised_address: self_address.clone(),
        grpc_manager_addresses: vec![self_address],
        fullnode_addresses: vec![],
        is_master: true,
        allow_fn_fallback: false,
        http2_ping_interval_secs: 60,
        http2_ping_timeout_secs: 10,
        service_staleness_threshold_secs: 60,
    };

    let task = tokio::spawn(async move {
        config.run().await.unwrap();
    });

    tokio::time::sleep(Duration::from_secs(10)).await;

    assert!(!task.is_finished());
}
