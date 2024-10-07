// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{GAUGE, TIMER};
use aptos_infallible::Mutex;
use aptos_metrics_core::{IntGaugeHelper, TimerHelper};
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc, Condvar,
};
use threadpool::ThreadPool;

/// A helper to send things to a thread pool for asynchronous dropping.
///
/// Be aware that there is a bounded number of concurrent drops, as a result:
///   1. when it's "out of capacity", `schedule_drop` will block until a slot to be available.
///   2. if the `Drop` implementation tries to lock things, there can be a potential deadlock due
///      to another thing being waiting for a slot to be available.
pub struct AsyncConcurrentDropper {
    name: &'static str,
    num_tasks_tracker: Arc<NumTasksTracker>,
    /// use dedicated thread pool to minimize the possibility of deadlock
    thread_pool: ThreadPool,
}

impl AsyncConcurrentDropper {
    pub fn new(name: &'static str, max_tasks: usize, num_threads: usize) -> Self {
        Self {
            name,
            num_tasks_tracker: Arc::new(NumTasksTracker::new(max_tasks)),
            thread_pool: ThreadPool::with_name(format!("{}_conc_dropper", name), num_threads),
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

    pub fn wait_for_backlog_drop(&self, no_more_than: usize) {
        let _timer = TIMER.timer_with(&[self.name, "wait_for_backlog_drop"]);
        self.num_tasks_tracker.wait_for_backlog_drop(no_more_than);
    }

    fn schedule_drop_impl<V: Send + 'static>(&self, v: V, notif_sender_opt: Option<Sender<()>>) {
        let _timer = TIMER.timer_with(&[self.name, "enqueue_drop"]);
        let num_tasks = self.num_tasks_tracker.inc();
        GAUGE.set_with(&[self.name, "num_tasks"], num_tasks as i64);

        let name = self.name;
        let num_tasks_tracker = self.num_tasks_tracker.clone();

        self.thread_pool.execute(move || {
            let _timer = TIMER.timer_with(&[name, "real_drop"]);

            drop(v);

            if let Some(sender) = notif_sender_opt {
                sender.send(()).ok();
            }

            num_tasks_tracker.dec();
        })
    }
}

struct NumTasksTracker {
    lock: Mutex<usize>,
    cvar: Condvar,
    max_tasks: usize,
}

impl NumTasksTracker {
    fn new(max_tasks: usize) -> Self {
        Self {
            lock: Mutex::new(0),
            cvar: Condvar::new(),
            max_tasks,
        }
    }

    fn inc(&self) -> usize {
        let mut num_tasks = self.lock.lock();
        while *num_tasks >= self.max_tasks {
            num_tasks = self.cvar.wait(num_tasks).expect("lock poisoned.");
        }
        *num_tasks += 1;
        *num_tasks
    }

    fn dec(&self) {
        let mut num_tasks = self.lock.lock();
        *num_tasks -= 1;
        self.cvar.notify_all();
    }

    fn wait_for_backlog_drop(&self, no_more_than: usize) {
        let mut num_tasks = self.lock.lock();
        while *num_tasks > no_more_than {
            num_tasks = self.cvar.wait(num_tasks).expect("lock poisoned.");
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::AsyncConcurrentDropper;
    use std::{sync::Arc, thread::sleep, time::Duration};
    use threadpool::ThreadPool;

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

    fn async_wait(
        thread_pool: &ThreadPool,
        dropper: &Arc<AsyncConcurrentDropper>,
        no_more_than: usize,
    ) {
        let dropper = Arc::clone(dropper);
        thread_pool.execute(move || dropper.wait_for_backlog_drop(no_more_than));
    }

    #[test]
    fn test_wait_for_backlog_drop() {
        let s = Arc::new(AsyncConcurrentDropper::new("test", 8, 4));
        let t = ThreadPool::new(4);
        let now = std::time::Instant::now();
        for _ in 0..8 {
            s.schedule_drop(SlowDropper);
        }
        assert!(now.elapsed() < Duration::from_millis(200));
        s.wait_for_backlog_drop(8);
        assert!(now.elapsed() < Duration::from_millis(200));
        async_wait(&t, &s, 8);
        async_wait(&t, &s, 8);
        async_wait(&t, &s, 7);
        async_wait(&t, &s, 4);
        t.join();
        assert!(now.elapsed() > Duration::from_millis(200));
        assert!(now.elapsed() < Duration::from_millis(400));
        s.wait_for_backlog_drop(4);
        assert!(now.elapsed() < Duration::from_millis(400));
        async_wait(&t, &s, 3);
        async_wait(&t, &s, 2);
        async_wait(&t, &s, 1);
        t.join();
        assert!(now.elapsed() > Duration::from_millis(400));
        assert!(now.elapsed() < Duration::from_millis(600));
        s.wait_for_backlog_drop(0);
        assert!(now.elapsed() < Duration::from_millis(600));
    }
}
