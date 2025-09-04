// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::BUFFER_MANAGER_PHASE_PROCESS_SECONDS,
    pipeline::buffer_manager::{Receiver, Sender},
};
use velor_logger::debug;
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};

#[async_trait]
pub trait StatelessPipeline: Send + Sync {
    type Request;
    type Response;

    const NAME: &'static str;

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

    fn spawn(&self) -> Self {
        Self::new(self.counter.clone())
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

    pub fn spawn<OtherRequest>(&self, other_req: OtherRequest) -> CountedRequest<OtherRequest> {
        CountedRequest {
            req: other_req,
            guard: self.guard.spawn(),
        }
    }
}

pub struct PipelinePhase<T: StatelessPipeline> {
    rx: Receiver<CountedRequest<T::Request>>,
    maybe_tx: Option<Sender<T::Response>>,
    processor: Box<T>,
    reset_flag: Arc<AtomicBool>,
}

impl<T: StatelessPipeline> PipelinePhase<T> {
    pub fn new(
        rx: Receiver<CountedRequest<T::Request>>,
        maybe_tx: Option<Sender<T::Response>>,
        processor: Box<T>,
        reset_flag: Arc<AtomicBool>,
    ) -> Self {
        Self {
            rx,
            maybe_tx,
            processor,
            reset_flag,
        }
    }

    pub async fn start(mut self) {
        // main loop
        while let Some(counted_req) = self.rx.next().await {
            let CountedRequest { req, guard: _guard } = counted_req;
            if self.reset_flag.load(Ordering::SeqCst) {
                continue;
            }
            let response = {
                let _timer = BUFFER_MANAGER_PHASE_PROCESS_SECONDS
                    .with_label_values(&[T::NAME])
                    .start_timer();
                self.processor.process(req).await
            };
            if let Some(tx) = &mut self.maybe_tx {
                if tx.send(response).await.is_err() {
                    debug!("Failed to send response, buffer manager probably dropped");
                    break;
                }
            }
        }
    }
}
