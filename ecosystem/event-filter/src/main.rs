use aptos_event_filter::worker::EventFilterConfig;
use aptos_indexer_grpc_server_framework::ServerArgs;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = <ServerArgs as clap::Parser>::parse();
    args.run::<EventFilterConfig>().await
}
