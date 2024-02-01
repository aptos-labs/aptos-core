// Copyright © Aptos Foundation

use aptos_indexer_grpc_data_access::StorageClient;
use aptos_protos::indexer::v1::TransactionsResponse;
use tokio::sync::mpsc::Sender;
use tonic::Status;

pub mod grpc_response_dispatcher;
pub use grpc_response_dispatcher::*;

/// ResponseDispatcher is a trait that defines the interface for dispatching responses into channel via provided sender.
#[async_trait::async_trait]
pub trait ResponseDispatcher {
    fn new(
        starting_version: u64,
        transaction_count: Option<u64>,
        sender: Sender<Result<TransactionsResponse, Status>>,
        // Dispatcher is expected to fetch responses from these storages in order;
        // if it fails to fetch from the first storage, it will try the second one, etc.
        // StorageClient is expected to be *cheap to clone*.
        storage_clients: &[StorageClient],
    ) -> Self;
    // Dispatch a single response to the channel.
    async fn dispatch(
        &mut self,
        response: Result<TransactionsResponse, Status>,
    ) -> anyhow::Result<()>;

    // Fetch responses that need to be dispatched. TransactionsResponse might get chunked into multiple responses.
    async fn fetch_with_retries(&mut self) -> anyhow::Result<Vec<TransactionsResponse>, Status>;

    // Run the dispatcher in a loop: fetch -> dispatch.
    async fn run(&mut self) -> anyhow::Result<()>;
}
