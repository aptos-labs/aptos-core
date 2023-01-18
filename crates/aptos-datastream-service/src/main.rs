// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_datastream_service::service::DatastreamServer;
use aptos_protos::datastream::v1::indexer_stream_server::IndexerStreamServer;
use deadpool_redis::{Config, Runtime};
use std::{net::ToSocketAddrs, sync::Arc};
use tonic::transport::Server;

pub fn get_redis_address() -> String {
    std::env::var("REDIS_ADDRESS").expect("REDIS_ADDRESS is required.")
}

pub fn get_redis_port() -> String {
    std::env::var("REDIS_PORT").expect("REDIS_PORT is required.")
}

pub fn get_server_port() -> String {
    std::env::var("SERVER_PORT").expect("SERVER_PORT is required.")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    aptos_logger::Logger::new().init();
    let redis_address = get_redis_address();
    let redis_port = get_redis_port();
    let cfg = Config::from_url(format!("redis://{}:{}", redis_address, redis_port));
    let redis_pool = Arc::new(cfg.create_pool(Some(Runtime::Tokio1)).unwrap());

    let server = DatastreamServer { redis_pool };

    Server::builder()
        .initial_stream_window_size(65535)
        .add_service(IndexerStreamServer::new(server))
        .serve(
            format!("0.0.0.0:{}", get_server_port())
                .to_string()
                .to_socket_addrs()
                .unwrap()
                .next()
                .unwrap(),
        )
        .await
        .unwrap();

    Ok(())
}
