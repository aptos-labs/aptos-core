// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_datastream_service::service::DatastreamServer;
use aptos_datastream_worker::redis_pool_client::RedisClient;
use aptos_protos::datastream::v1::indexer_stream_server::IndexerStreamServer;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let redis_client = Arc::new(RedisClient::new("localhost:6379".to_string()));

    let server = DatastreamServer { redis_client };

    Server::builder()
        .initial_stream_window_size(65535)
        .add_service(IndexerStreamServer::new(server))
        .serve("[::1]:50051".to_socket_addrs().unwrap().next().unwrap())
        .await
        .unwrap();

    Ok(())
}
