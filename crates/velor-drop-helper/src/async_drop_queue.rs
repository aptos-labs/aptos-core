// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::TIMER;
use velor_infallible::Mutex;
use velor_metrics_core::TimerHelper;
use std::sync::mpsc::{channel, Receiver, Sender};
use threadpool::ThreadPool;

#[derive(Debug)]
pub struct AsyncDropQueue {
    name: &'static str,
    token_tx: Sender<()>,
    token_rx: Mutex<Receiver<()>>,
    thread: ThreadPool,
}

impl AsyncDropQueue {
    pub fn new(name: &'static str, max_pending_drops: usize) -> Self {
        let (token_tx, token_rx) = channel();
        for _ in 0..max_pending_drops {
            token_tx
                .send(())
                .expect("AsyncDropQueue: Failed to buffer initial tokens.");
        }
        // single threaded threadpool
        let thread = ThreadPool::new(1);
        Self {
            name,
            token_tx,
            token_rx: Mutex::new(token_rx),
            thread,
        }
    }

    pub fn enqueue_drop<V: Send + 'static>(&self, v: V) {
        let _timer = TIMER.timer_with(&[self.name, "enqueue_drop"]);

        self.token_rx.lock().recv().unwrap();

        let token_tx = self.token_tx.clone();
        let name = self.name;
        self.thread.execute(move || {
            let _timer = TIMER.timer_with(&[name, "real_drop"]);

            drop(v);

            token_tx.send(()).ok();
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::async_drop_queue::AsyncDropQueue;
    use std::{
        sync::mpsc::{channel, Sender},
        thread::sleep,
        time::Duration,
    };

    struct SendBackOnDrop {
        id: usize,
        tx: Sender<usize>,
    }

    impl Drop for SendBackOnDrop {
        fn drop(&mut self) {
            sleep(Duration::from_millis(200));
            self.tx.send(self.id).unwrap()
        }
    }

    #[test]
    fn test_queue() {
        let q = AsyncDropQueue::new("test", 4);
        let now = std::time::Instant::now();
        let (tx, rx) = channel();
        for id in 0..4 {
            q.enqueue_drop(SendBackOnDrop { id, tx: tx.clone() });
        }
        assert!(now.elapsed() < Duration::from_millis(200));
        q.enqueue_drop(SendBackOnDrop {
            id: 4,
            tx: tx.clone(),
        });
        assert!(now.elapsed() > Duration::from_millis(200));
        drop(tx);
        assert_eq!(rx.iter().collect::<Vec<_>>(), (0..5).collect::<Vec<_>>(),);
        assert!(now.elapsed() > Duration::from_secs(1));
    }
}
