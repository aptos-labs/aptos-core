// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_indexer_grpc_file_store::{get_redis_address, processor::Processor};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use warp::Filter;

fn main() {
    aptos_logger::Logger::new().init();
    aptos_crash_handler::setup_panic_handler();
    let runtime = aptos_runtimes::spawn_named_runtime("indexerfile".to_string(), None);

    let redis_address = get_redis_address();
    runtime.spawn(async move {
        let mut processor = Processor::new(redis_address);
        processor.run().await;
    });

    // Start liveness and readiness probes.
    runtime.spawn(async move {
        let readiness = warp::path("readiness")
            .map(move || warp::reply::with_status("ready", warp::http::StatusCode::OK));
        warp::serve(readiness).run(([0, 0, 0, 0], 8080)).await;
    });

    let term = Arc::new(AtomicBool::new(false));
    while !term.load(Ordering::Acquire) {
        std::thread::park();
    }
}
