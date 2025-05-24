// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_indexer_grpc_server_framework::ServerArgs;
use aptos_indexer_grpc_v2_file_store_backfiller::IndexerGrpcV2FileStoreBackfillerConfig;
use clap::Parser;

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.run::<IndexerGrpcV2FileStoreBackfillerConfig>()
        .await
        .expect("Failed to run server");
    Ok(())
}
