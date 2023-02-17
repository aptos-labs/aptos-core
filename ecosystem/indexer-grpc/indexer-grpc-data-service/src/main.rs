// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_indexer_grpc_data_service::{get_grpc_address, service::DatastreamServer};
use aptos_indexer_grpc_utils::get_health_check_port;
use aptos_protos::datastream::v1::indexer_stream_server::IndexerStreamServer;
use std::{
    net::ToSocketAddrs,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tonic::transport::Server;
use warp::Filter;

fn main() {
    aptos_logger::Logger::new().init();
    aptos_crash_handler::setup_panic_handler();

    let runtime = aptos_runtimes::spawn_named_runtime("indexerdata".to_string(), None);
    // Start serving.
    runtime.spawn(async move {
        let server = DatastreamServer::new();
        Server::builder()
            .add_service(IndexerStreamServer::new(server))
            .serve(
                get_grpc_address()
                    .to_socket_addrs()
                    .unwrap()
                    .next()
                    .unwrap(),
            )
            .await
            .unwrap();
    });

    // Start liveness and readiness probes.
    runtime.spawn(async move {
        let readiness = warp::path("readiness")
            .map(move || warp::reply::with_status("ready", warp::http::StatusCode::OK));
        warp::serve(readiness)
            .run(([0, 0, 0, 0], get_health_check_port()))
            .await;
    });

    let term = Arc::new(AtomicBool::new(false));
    while !term.load(Ordering::Acquire) {
        std::thread::park();
    }
}
