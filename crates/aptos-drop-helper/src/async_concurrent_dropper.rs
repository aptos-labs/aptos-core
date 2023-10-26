// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::TIMER;
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_infallible::Mutex;
use aptos_metrics_core::TimerHelper;
use std::sync::mpsc::{channel, Receiver, Sender};

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
}

impl AsyncConcurrentDropper {
    pub fn new(name: &'static str, max_concurrent_drops: usize) -> Self {
        let (token_tx, token_rx) = channel();
        for _ in 0..max_concurrent_drops {
            token_tx
                .send(())
                .expect("DropHelper: Failed to buffer initial tokens.");
        }
        Self {
            name,
            token_tx,
            token_rx: Mutex::new(token_rx),
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
        THREAD_MANAGER.get_non_exe_cpu_pool().spawn(move || {
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
            sleep(Duration::from_secs(1));
        }
    }

    #[test]
    fn test_concurrency_limit_hit() {
        let s = AsyncConcurrentDropper::new("test", 8);
        let now = std::time::Instant::now();
        let rx = s.schedule_drop_with_waiter(SlowDropper);
        for _ in 1..8 {
            s.schedule_drop(SlowDropper);
        }
        assert!(now.elapsed() < Duration::from_millis(500));
        rx.recv().unwrap();
        assert!(now.elapsed() > Duration::from_secs(1));
    }

    #[test]
    fn test_within_concurrency_limit() {
        let s = AsyncConcurrentDropper::new("test", 8);
        let now = std::time::Instant::now();
        for _ in 0..8 {
            s.schedule_drop(SlowDropper);
        }
        assert!(now.elapsed() < Duration::from_millis(500));
        s.schedule_drop(SlowDropper);
        assert!(now.elapsed() > Duration::from_secs(1));
    }
}
