// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_indexer_grpc_parser::worker::Worker;
use clap::Parser;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use warp::Filter;

#[derive(Parser)]
pub struct Args {
    #[clap(short, long)]
    pub config_path: String,
}

fn main() {
    aptos_logger::Logger::new().init();
    aptos_crash_handler::setup_panic_handler();

    // Load config.
    let args = Args::parse();
    let config = aptos_indexer_grpc_utils::config::IndexerGrpcProcessorConfig::load(
        std::path::PathBuf::from(args.config_path),
    )
    .unwrap();

    let runtime = aptos_runtimes::spawn_named_runtime("indexerproc".to_string(), None);

    let health_port = config.health_check_port;
    runtime.spawn(async move {
        let worker = Worker::new(config).await;
        worker.run().await;
    });

    // Start liveness and readiness probes.
    runtime.spawn(async move {
        let readiness = warp::path("readiness")
            .map(move || warp::reply::with_status("ready", warp::http::StatusCode::OK));
        // TODO: fix the liveness probe port number to be configurable.
        warp::serve(readiness)
            .run(([0, 0, 0, 0], health_port))
            .await;
    });

    let term = Arc::new(AtomicBool::new(false));
    while !term.load(Ordering::Acquire) {
        std::thread::park();
    }
}
