// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_datastream_worker::{constants::APTOS_DATASTREAM_WORKER_THREAD_NAME, worker::Worker};
use tokio::runtime::Builder;

fn main() {
    aptos_logger::Logger::new().init();
    let runtime = Builder::new_multi_thread()
        .thread_name(APTOS_DATASTREAM_WORKER_THREAD_NAME)
        .disable_lifo_slot()
        .enable_all()
        .build()
        .expect("[indexer] failed to create runtime");
    // Start processing.
    runtime.spawn(async move {
        let worker = Worker::new().await;
        worker.run().await;
    });

    std::thread::park();
}
