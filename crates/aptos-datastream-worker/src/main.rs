// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_datastream_worker::{constants::APTOS_DATASTREAM_WORKER_THREAD_NAME, worker::Worker};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::runtime::Builder;
use warp::Filter;

fn main() {
    aptos_logger::Logger::new().init();
    let runtime = Builder::new_multi_thread()
        .thread_name(APTOS_DATASTREAM_WORKER_THREAD_NAME)
        .disable_lifo_slot()
        .enable_all()
        .build()
        .expect("[indexer] failed to create runtime");

    let worker_status = Arc::new(AtomicBool::new(false));
    let worker_status_readiness = worker_status.clone();
    let worker_status_liveness = worker_status.clone();
    // Start processing.
    runtime.spawn(async move {
        let worker = Worker::new(worker_status).await;
        worker.run().await;
    });

    // TODO: add version check for cold storage.
    runtime.spawn(async move {
        let readiness = warp::path("readiness").map(move || {
            match worker_status_readiness.load(Ordering::SeqCst) {
                true => warp::reply::with_status("ready", warp::http::StatusCode::OK),
                false => warp::reply::with_status(
                    "not ready",
                    warp::http::StatusCode::SERVICE_UNAVAILABLE,
                ),
            }
        });
        let liveness = warp::path("liveness").map(move || {
            match worker_status_liveness.load(Ordering::SeqCst) {
                true => warp::reply::with_status("alive", warp::http::StatusCode::OK),
                false => warp::reply::with_status(
                    "not alive",
                    warp::http::StatusCode::SERVICE_UNAVAILABLE,
                ),
            }
        });
        warp::serve(readiness.or(liveness))
            .run(([0, 0, 0, 0], 8080))
            .await;
    });
    std::thread::park();
}
