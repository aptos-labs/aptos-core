// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_indexer_grpc_cache_worker::worker::Worker;
use aptos_indexer_grpc_utils::register_probes_and_metrics_handler;
use clap::Parser;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
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
    let config = aptos_indexer_grpc_utils::config::IndexerGrpcConfig::load(
        std::path::PathBuf::from(args.config_path),
    )
    .unwrap();

    let health_port = config.health_check_port;

    let runtime = aptos_runtimes::spawn_named_runtime("indexercache".to_string(), None);

    // Start processing.
    runtime.spawn(async move {
        let mut worker = Worker::new(config).await;
        worker.run().await;
    });

    // Start liveness and readiness probes.
    runtime.spawn(async move {
        register_probes_and_metrics_handler(health_port).await;
    });

    let term = Arc::new(AtomicBool::new(false));
    while !term.load(Ordering::Acquire) {
        thread::park();
    }
}
