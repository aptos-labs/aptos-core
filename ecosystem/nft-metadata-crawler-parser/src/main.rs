// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_indexer_grpc_server_framework::ServerArgs;
use aptos_nft_metadata_crawler_parser::config::NFTMetadataCrawlerConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = <ServerArgs as clap::Parser>::parse();
    args.run::<NFTMetadataCrawlerConfig>().await
}
