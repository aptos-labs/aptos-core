// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_indexer_grpc_parser::worker::Worker;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::runtime::Builder;
use warp::Filter;

fn main() {
    aptos_logger::Logger::new().init();
    aptos_crash_handler::setup_panic_handler();
    let runtime = Builder::new_multi_thread()
        .thread_name("Datastream Indexer")
        .disable_lifo_slot()
        .enable_all()
        .build()
        .expect("[indexer] failed to create runtime");
    // Start processing.
    runtime.spawn(async move {
        let worker = Worker::new().await;
        worker.run().await;
    });

    // Start liveness and readiness probes.
    runtime.spawn(async move {
        let readiness = warp::path("readiness")
            .map(move || warp::reply::with_status("ready", warp::http::StatusCode::OK));
        // TODO: fix the liveness probe port number to be configurable.
        warp::serve(readiness).run(([0, 0, 0, 0], 8080)).await;
    });

    let term = Arc::new(AtomicBool::new(false));
    while !term.load(Ordering::Acquire) {
        std::thread::park();
    }
}
