// Copyright © Aptos Foundation

use crate::ParserConfig;
use aptos_indexer_grpc_server_framework::ServerArgs;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = <ServerArgs as clap::Parser>::parse();
    args.run::<ParserConfig>().await
}
