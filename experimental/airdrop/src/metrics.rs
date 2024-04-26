// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_logger::info;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize},
        Arc,
    },
    time::{Duration, Instant},
};

pub(crate) trait PrintProgress {
    fn print_progress(&self, elapsed: Duration);
}

pub(crate) fn spawn_async_tracking<T: PrintProgress + Send + Sync + 'static>(tracking: Arc<T>, interval: Duration) -> Arc<AtomicBool> {
    let done = Arc::new(AtomicBool::new(false));
    let done_clone = done.clone();
    tokio::spawn(async move {
        let mut previous = Instant::now();
        while !done_clone.load(std::sync::atomic::Ordering::Relaxed) {
            tokio::time::sleep(interval).await;
            let current = Instant::now();
            tracking.print_progress(current - previous);
            previous = current;
        }
    });
    done
}

impl PrintProgress for AtomicUsize {
    fn print_progress(&self, _elapsed: Duration) {
        info!("Progress: {}", self.load(std::sync::atomic::Ordering::Relaxed));
    }
}

const LATENCY_PRECISION: f64 = 0.1;

pub(crate) struct Tracking {
    submitted: AtomicUsize,
    done: AtomicUsize,
    sum_latency: AtomicU64,

    last_printed_submitted: AtomicUsize,
    last_printed_done: AtomicUsize,
    last_printed_sum_latency: AtomicU64,

    last_printed_latency: AtomicU64,
}

impl Tracking {
    pub fn new() -> Self {
        Self {
            submitted: AtomicUsize::new(0),
            done: AtomicUsize::new(0),
            sum_latency: AtomicU64::new(0),
            last_printed_submitted: AtomicUsize::new(0),
            last_printed_done: AtomicUsize::new(0),
            last_printed_sum_latency: AtomicU64::new(0),
            last_printed_latency: AtomicU64::new(0),
        }
    }

    pub fn submitted(&self, num: usize) -> Instant {
        self.submitted
            .fetch_add(num, std::sync::atomic::Ordering::Relaxed);
        Instant::now()
    }

    pub fn committed_succesfully(&self, num: usize, submitted_time: Instant) {
        self.done
            .fetch_add(num, std::sync::atomic::Ordering::Relaxed);
        self.sum_latency.fetch_add(
            (submitted_time.elapsed().as_secs_f64() / LATENCY_PRECISION) as u64 * num as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    pub fn print_stats(&self, elapsed: f64) {
        let submitted = self.submitted.load(std::sync::atomic::Ordering::Relaxed);
        let done = self.done.load(std::sync::atomic::Ordering::Relaxed);
        let sum_latency = self.sum_latency.load(std::sync::atomic::Ordering::Relaxed);
        info!(
            "Submitted: {}, Done: {}, Avg latency: {}, Avg TPS: {} (including warm up and checking for committed transactions)",
            submitted,
            done,
            sum_latency as f64 / done as f64 * LATENCY_PRECISION,
            done as f64 / elapsed
        );
    }

    pub fn get_last_latency(&self) -> f64 {
        let last_done = self.last_printed_done.load(std::sync::atomic::Ordering::Relaxed);
        let cur_done = self.done.load(std::sync::atomic::Ordering::Relaxed);

        let last_sum_latency = self.last_printed_sum_latency.load(std::sync::atomic::Ordering::Relaxed);
        let cur_sum_latency = self.sum_latency.load(std::sync::atomic::Ordering::Relaxed);

        let committed = cur_done - last_done;

        let last_latency = self.last_printed_latency.load(std::sync::atomic::Ordering::Relaxed) as f64 * LATENCY_PRECISION;

        if committed > 0 {
            last_latency.min(((cur_sum_latency - last_sum_latency) as f64 / committed as f64) * LATENCY_PRECISION)
        } else {
            last_latency
        }
    }
}

impl PrintProgress for Tracking {
    fn print_progress(&self, elapsed: Duration) {
        let cur_submitted = self.submitted.load(std::sync::atomic::Ordering::Relaxed);
        let last_submitted = self.last_printed_submitted.swap(cur_submitted, std::sync::atomic::Ordering::Relaxed);

        let cur_done = self.done.load(std::sync::atomic::Ordering::Relaxed);
        let last_done = self.last_printed_done.swap(cur_done, std::sync::atomic::Ordering::Relaxed);

        let cur_sum_latency = self.sum_latency.load(std::sync::atomic::Ordering::Relaxed);
        let last_sum_latency = self.last_printed_sum_latency.swap(cur_sum_latency, std::sync::atomic::Ordering::Relaxed);

        let committed = cur_done - last_done;

        let latency = if committed > 0 {
            ((cur_sum_latency - last_sum_latency) as f64 / committed as f64) as u64
        } else {
            0
        };
        self.last_printed_latency.store(latency, std::sync::atomic::Ordering::Relaxed);
        info!(
            "Blockchain: progress: {}, committed TPS: {}, submitted TPS {}, latency {}",
            cur_done,
            committed as f32 / elapsed.as_secs_f32(),
            (cur_submitted - last_submitted) as f32 / elapsed.as_secs_f32(),
            latency as f64 * LATENCY_PRECISION,
        );
    }
}
