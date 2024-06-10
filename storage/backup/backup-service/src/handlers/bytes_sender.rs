// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_storage_interface::{AptosDbError, Result as DbResult};
use bytes::{Bytes, BytesMut};

type BoxError = Box<dyn std::error::Error + Send + Sync>;
type BytesResult = Result<Bytes, BoxError>;

pub(super) struct BytesSender {
    buffer: BytesMut,
    bytes_tx: tokio::sync::mpsc::Sender<BytesResult>,
}

impl BytesSender {
    const MAX_BATCHES: usize = 100;
    #[cfg(not(test))]
    const TARGET_BATCH_SIZE: usize = 1024 * 1024;
    #[cfg(test)]
    const TARGET_BATCH_SIZE: usize = 100;

    pub fn new() -> (Self, tokio_stream::wrappers::ReceiverStream<BytesResult>) {
        let (bytes_tx, bytes_rx) = tokio::sync::mpsc::channel(Self::MAX_BATCHES);

        let myself = Self {
            buffer: BytesMut::new(),
            bytes_tx,
        };

        let stream = tokio_stream::wrappers::ReceiverStream::new(bytes_rx);

        (myself, stream)
    }

    pub fn send_bytes(&mut self, bytes: Bytes) -> DbResult<()> {
        self.buffer.extend(bytes);

        if self.buffer.len() >= Self::TARGET_BATCH_SIZE {
            self.flush_buffer()?
        }

        Ok(())
    }

    pub fn flush_buffer(&mut self) -> DbResult<()> {
        let bytes = self.buffer.split().freeze();
        self.send_res(Ok(bytes))
    }

    pub fn finish(mut self) -> DbResult<()> {
        self.flush_buffer()
    }

    pub fn abort<E: std::error::Error + Send + Sync + 'static>(self, err: E) -> DbResult<()> {
        self.send_res(Err(Box::new(err)))
    }

    pub fn send_res(&self, item: BytesResult) -> DbResult<()> {
        self.bytes_tx
            .blocking_send(item)
            .map_err(|e| AptosDbError::Other(format!("Failed to send to response stream. {e}")))
    }
}
