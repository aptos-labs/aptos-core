// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_indexer_grpc_server_framework::ServerArgs;
use aptos_nft_metadata_crawler::config::NFTMetadataCrawlerConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = <ServerArgs as clap::Parser>::parse();
    args.run::<NFTMetadataCrawlerConfig>().await
}
