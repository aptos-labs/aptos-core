// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::experimental::buffer_manager::{Receiver, Sender};
use aptos_logger::debug;
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

struct TaskGuard {
    counter: Arc<AtomicU64>,
}

impl TaskGuard {
    fn new(counter: Arc<AtomicU64>) -> Self {
        counter.fetch_add(1, Ordering::SeqCst);
        Self { counter }
    }
}

impl Drop for TaskGuard {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::SeqCst);
    }
}

pub struct CountedRequest<Request> {
    req: Request,
    guard: TaskGuard,
}

impl<Request> CountedRequest<Request> {
    pub fn new(req: Request, counter: Arc<AtomicU64>) -> Self {
        let guard = TaskGuard::new(counter);
        Self { req, guard }
    }
}

pub struct PipelinePhase<T: StatelessPipeline> {
    rx: Receiver<CountedRequest<T::Request>>,
    maybe_tx: Option<Sender<T::Response>>,
    processor: Box<T>,
}

impl<T: StatelessPipeline> PipelinePhase<T> {
    pub fn new(
        rx: Receiver<CountedRequest<T::Request>>,
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
        while let Some(counted_req) = self.rx.next().await {
            let CountedRequest { req, guard: _guard } = counted_req;
            let response = self.processor.process(req).await;
            if let Some(tx) = &mut self.maybe_tx {
                if tx.send(response).await.is_err() {
                    debug!("Failed to send response, buffer manager probably dropped");
                    break;
                }
            }
        }
    }
}
