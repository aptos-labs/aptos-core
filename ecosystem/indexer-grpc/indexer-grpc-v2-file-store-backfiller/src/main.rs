// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
