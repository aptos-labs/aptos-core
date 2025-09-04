// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use velor_indexer_grpc_file_store_backfiller::IndexerGrpcFileStoreBackfillerConfig;
use velor_indexer_grpc_server_framework::ServerArgs;
use clap::Parser;

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.run::<IndexerGrpcFileStoreBackfillerConfig>()
        .await
        .expect("Failed to run server");
    Ok(())
}
