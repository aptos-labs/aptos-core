// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_indexer_grpc_data_service::IndexerGrpcDataServiceConfig;
use aptos_indexer_grpc_server_framework::ServerArgs;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.run::<IndexerGrpcDataServiceConfig>().await
}
