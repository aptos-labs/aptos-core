// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_datastream_indexer::worker::Worker;
use tokio::runtime::Builder;

fn main() {
    aptos_logger::Logger::new().init();
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

    std::thread::park();
}
