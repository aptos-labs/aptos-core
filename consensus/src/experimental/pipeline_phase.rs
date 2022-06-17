// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::experimental::buffer_manager::{Receiver, Sender};
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

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
    ongoing_tasks: Arc<AtomicU64>,
}

impl<T: StatelessPipeline> PipelinePhase<T> {
    pub fn new(
        rx: Receiver<T::Request>,
        maybe_tx: Option<Sender<T::Response>>,
        processor: Box<T>,
        ongoing_tasks: Arc<AtomicU64>,
    ) -> Self {
        Self {
            rx,
            maybe_tx,
            processor,
            ongoing_tasks,
        }
    }

    pub async fn start(mut self) {
        // main loop
        while let Some(req) = self.rx.next().await {
            self.ongoing_tasks.fetch_add(1, Ordering::SeqCst);
            let response = self.processor.process(req).await;
            self.ongoing_tasks.fetch_sub(1, Ordering::SeqCst);
            if let Some(tx) = &mut self.maybe_tx {
                if tx.send(response).await.is_err() {
                    break;
                }
            }
        }
    }
}
