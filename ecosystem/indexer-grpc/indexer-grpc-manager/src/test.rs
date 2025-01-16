// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::config::{IndexerGrpcManagerConfig, ServiceConfig};
use aptos_config::utils::get_available_port;
use aptos_indexer_grpc_server_framework::RunnableConfig;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 16)]
async fn test_run() {
    let port = get_available_port();
    let listen_address = format!("127.0.0.1:{port}").parse().unwrap();
    let config = IndexerGrpcManagerConfig {
        chain_id: 0,
        service_config: ServiceConfig { listen_address },
    };

    let task = tokio::spawn(async move {
        config.run().await.unwrap();
    });

    tokio::time::sleep(Duration::from_secs(10)).await;

    assert!(!task.is_finished());
}
