// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    SinkExt, StreamExt,
};

#[async_trait]
pub trait StatelessPipeline: Send + Sync {
    type Request;
    type Response;
    async fn process(&self, req: Self::Request) -> Self::Response;
}

pub struct PipelinePhase<T: StatelessPipeline> {
    rx: UnboundedReceiver<T::Request>,
    tx: UnboundedSender<T::Response>,
    processor: Box<T>,
}

impl<T: StatelessPipeline> PipelinePhase<T> {
    pub fn new(
        rx: UnboundedReceiver<T::Request>,
        tx: UnboundedSender<T::Response>,
        processor: Box<T>,
    ) -> Self {
        Self { rx, tx, processor }
    }

    pub async fn start(mut self) {
        // main loop
        while let Some(req) = self.rx.next().await {
            let resp = self.processor.process(req).await;
            if self.tx.send(resp).await.is_err() {
                break;
            }
        }
    }
}
