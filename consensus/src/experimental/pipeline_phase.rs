// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::experimental::buffer_manager::{Receiver, Sender};
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};

#[async_trait]
pub trait StatelessPipeline: Send + Sync {
    type Request;
    type Response;
    async fn process(&self, req: Self::Request) -> Self::Response;
}

pub struct PipelinePhase<T: StatelessPipeline> {
    rx: Receiver<T::Request>,
    maybe_tx: Option<Sender<T::Response>>,
    processor: Box<T>,
}

impl<T: StatelessPipeline> PipelinePhase<T> {
    pub fn new(
        rx: Receiver<T::Request>,
        maybe_tx: Option<Sender<T::Response>>,
        processor: Box<T>,
    ) -> Self {
        Self {
            rx,
            maybe_tx,
            processor,
        }
    }

    pub async fn start(mut self) {
        // main loop
        while let Some(req) = self.rx.next().await {
            let response = self.processor.process(req).await;
            if let Some(tx) = &mut self.maybe_tx {
                if tx.send(response).await.is_err() {
                    break;
                }
            }
        }
    }
}
