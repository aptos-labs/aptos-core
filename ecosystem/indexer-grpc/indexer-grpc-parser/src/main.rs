// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_indexer_grpc_parser::worker::Worker;
use aptos_indexer_grpc_utils::register_probes_and_metrics_handler;
use clap::Parser;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

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
        register_probes_and_metrics_handler(health_port).await;
    });

    let term = Arc::new(AtomicBool::new(false));
    while !term.load(Ordering::Acquire) {
        std::thread::park();
    }
}
