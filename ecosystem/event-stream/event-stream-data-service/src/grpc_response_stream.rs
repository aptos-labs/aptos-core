// Copyright © Aptos Foundation

use crate::response_dispatcher::{GrpcResponseDispatcher, ResponseDispatcher};
use aptos_indexer_grpc_data_access::StorageClient;
use aptos_protos::indexer::v1::TransactionsResponse;
use futures::Stream;
use tokio::sync::mpsc::channel;
use tonic::Status;

/// GrpcResponseStream is a struct that provides a stream of responses to the gRPC server.
/// The response stream is backed by a channel that is filled by GrpcResponseGenerator in another thread.
/// TODO: Add generic support for other types of responses or server-side transformations.
pub struct GrpcResponseStream {
    /// The channel for receiving responses from upstream clients.
    inner: tokio_stream::wrappers::ReceiverStream<Result<TransactionsResponse, Status>>,
}

impl GrpcResponseStream {
    #[allow(dead_code)]
    pub fn new(
        starting_version: u64,
        transaction_count: Option<u64>,
        buffer_size: Option<usize>,
        storages: &[StorageClient],
    ) -> anyhow::Result<Self> {
        let (channel_sender, channel_receiver) = channel(buffer_size.unwrap_or(12));
        let response_stream = Self {
            inner: tokio_stream::wrappers::ReceiverStream::new(channel_receiver),
        };
        let storages = storages.to_vec();
        // Start a separate thread to generate the response for the stream.
        tokio::spawn(async move {
            let mut response_dispatcher = GrpcResponseDispatcher::new(
                starting_version,
                transaction_count,
                channel_sender,
                storages.as_slice(),
            );
            match response_dispatcher.run().await {
                Ok(_) => {
                    tracing::info!("Response dispatcher finished successfully.");
                },
                Err(e) => {
                    tracing::error!("Response dispatcher failed: {}", e);
                },
            }
        });
        Ok(response_stream)
    }
}

impl Stream for GrpcResponseStream {
    type Item = Result<TransactionsResponse, Status>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.get_mut();
        std::pin::Pin::new(&mut this.inner).poll_next(cx)
    }
}
