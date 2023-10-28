// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::TIMER;
use aptos_infallible::Mutex;
use aptos_metrics_core::TimerHelper;
use std::sync::mpsc::{channel, Receiver, Sender};
use threadpool::ThreadPool;

/// A helper to send things to a thread pool for asynchronous dropping.
///
/// Be aware that there is a bounded number of concurrent drops, as a result:
///   1. when it's "out of capacity", `schedule_drop` will block until a slot to be available.
///   2. if the `Drop` implementation tries to lock things, there can be a potential dead lock due
///      to another thing being waiting for a slot to be available.
pub struct AsyncConcurrentDropper {
    name: &'static str,
    token_tx: Sender<()>,
    token_rx: Mutex<Receiver<()>>,
    /// use dedicated threadpool to minimize the possibility of dead lock
    thread_pool: ThreadPool,
}

impl AsyncConcurrentDropper {
    pub fn new(name: &'static str, max_async_drops: usize, num_threads: usize) -> Self {
        let (token_tx, token_rx) = channel();
        for _ in 0..max_async_drops {
            token_tx
                .send(())
                .expect("DropHelper: Failed to buffer initial tokens.");
        }
        let thread_pool = ThreadPool::new(num_threads);
        Self {
            name,
            token_tx,
            token_rx: Mutex::new(token_rx),
            thread_pool,
        }
    }

    pub fn schedule_drop<V: Send + 'static>(&self, v: V) {
        self.schedule_drop_impl(v, None)
    }

    pub fn schedule_drop_with_waiter<V: Send + 'static>(&self, v: V) -> Receiver<()> {
        let (tx, rx) = channel();
        self.schedule_drop_impl(v, Some(tx));
        rx
    }

    fn schedule_drop_impl<V: Send + 'static>(&self, v: V, notif_sender_opt: Option<Sender<()>>) {
        let _timer = TIMER.timer_with(&[self.name, "enqueue_drop"]);

        self.token_rx.lock().recv().unwrap();

        let token_tx = self.token_tx.clone();
        let name = self.name;
        self.thread_pool.execute(move || {
            let _timer = TIMER.timer_with(&[name, "real_drop"]);

            drop(v);

            if let Some(sender) = notif_sender_opt {
                sender.send(()).ok();
            }

            token_tx.send(()).ok();
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::AsyncConcurrentDropper;
    use std::{thread::sleep, time::Duration};

    struct SlowDropper;

    impl Drop for SlowDropper {
        fn drop(&mut self) {
            sleep(Duration::from_millis(200));
        }
    }

    #[test]
    fn test_within_concurrency_limit() {
        let s = AsyncConcurrentDropper::new("test", 8, 4);
        let now = std::time::Instant::now();
        let rx1 = s.schedule_drop_with_waiter(SlowDropper); // first round
        for _ in 0..3 {
            s.schedule_drop(SlowDropper);
        }
        let rx2 = s.schedule_drop_with_waiter(SlowDropper); // second round
        for _ in 0..3 {
            s.schedule_drop(SlowDropper);
        }
        assert!(now.elapsed() < Duration::from_millis(200));
        rx1.recv().unwrap();
        assert!(now.elapsed() > Duration::from_millis(200));
        assert!(now.elapsed() < Duration::from_millis(400));
        rx2.recv().unwrap();
        assert!(now.elapsed() > Duration::from_millis(400));
        assert!(now.elapsed() < Duration::from_millis(600));
    }

    #[test]
    fn test_concurrency_limit_hit() {
        let s = AsyncConcurrentDropper::new("test", 8, 4);
        let now = std::time::Instant::now();
        for _ in 0..8 {
            s.schedule_drop(SlowDropper);
        }
        assert!(now.elapsed() < Duration::from_millis(200));
        s.schedule_drop(SlowDropper);
        assert!(now.elapsed() > Duration::from_millis(200));
        assert!(now.elapsed() < Duration::from_millis(400));
        s.schedule_drop(SlowDropper);
        assert!(now.elapsed() < Duration::from_millis(400));
    }
}
