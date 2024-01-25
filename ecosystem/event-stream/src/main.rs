// Copyright © Aptos Foundation

use aptos_indexer_grpc_server_framework::ServerArgs;
use aptos_event_stream::worker::EventStreamConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = <ServerArgs as clap::Parser>::parse();
    args.run::<EventStreamConfig>().await
}
