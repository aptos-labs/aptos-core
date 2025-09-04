// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::{GAUGE, TIMER},
    IN_ANY_DROP_POOL,
};
use velor_infallible::Mutex;
use velor_metrics_core::{IntGaugeVecHelper, TimerHelper};
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
            num_tasks_tracker: Arc::new(NumTasksTracker::new(name, max_tasks)),
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

    pub fn max_tasks(&self) -> usize {
        self.num_tasks_tracker.max_tasks
    }

    pub fn num_threads(&self) -> usize {
        self.thread_pool.max_count()
    }

    pub fn wait_for_backlog_drop(&self, no_more_than: usize) {
        let _timer = TIMER.timer_with(&[self.name, "wait_for_backlog_drop"]);
        self.num_tasks_tracker.wait_for_backlog_drop(no_more_than);
    }

    fn schedule_drop_impl<V: Send + 'static>(&self, v: V, notif_sender_opt: Option<Sender<()>>) {
        if IN_ANY_DROP_POOL.get() {
            Self::do_drop(v, notif_sender_opt);
            return;
        }

        let _timer = TIMER.timer_with(&[self.name, "enqueue_drop"]);
        self.num_tasks_tracker.inc();

        let name = self.name;
        let num_tasks_tracker = self.num_tasks_tracker.clone();

        self.thread_pool.execute(move || {
            let _timer = TIMER.timer_with(&[name, "real_drop"]);

            IN_ANY_DROP_POOL.with(|flag| {
                flag.set(true);
            });

            Self::do_drop(v, notif_sender_opt);

            num_tasks_tracker.dec();
        })
    }

    fn do_drop<V: Send + 'static>(v: V, notif_sender_opt: Option<Sender<()>>) {
        drop(v);

        if let Some(sender) = notif_sender_opt {
            sender.send(()).ok();
        }
    }
}

struct NumTasksTracker {
    name: &'static str,
    lock: Mutex<usize>,
    cvar: Condvar,
    max_tasks: usize,
}

impl NumTasksTracker {
    fn new(name: &'static str, max_tasks: usize) -> Self {
        Self {
            name,
            lock: Mutex::new(0),
            cvar: Condvar::new(),
            max_tasks,
        }
    }

    fn inc(&self) {
        let mut num_tasks = self.lock.lock();
        while *num_tasks >= self.max_tasks {
            num_tasks = self.cvar.wait(num_tasks).expect("lock poisoned.");
        }
        *num_tasks += 1;
        GAUGE.set_with(&[self.name, "num_tasks"], *num_tasks as i64);
    }

    fn dec(&self) {
        let mut num_tasks = self.lock.lock();
        *num_tasks -= 1;
        GAUGE.set_with(&[self.name, "num_tasks"], *num_tasks as i64);
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
    use crate::{AsyncConcurrentDropper, DropHelper, DEFAULT_DROPPER};
    use rayon::prelude::*;
    use std::{sync::Arc, thread::sleep, time::Duration};
    use threadpool::ThreadPool;

    #[derive(Clone, Default)]
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

    #[test]
    fn test_nested_drops() {
        #[derive(Clone, Default)]
        struct Nested {
            _inner: DropHelper<SlowDropper>,
        }

        // pump 2 x max_tasks to the drop queue
        let num_items = DEFAULT_DROPPER.max_tasks() * 2;
        let items = vec![DropHelper::new(Nested::default()); num_items];
        let drop_thread = std::thread::spawn(move || {
            items.into_par_iter().for_each(drop);
        });

        // expect no deadlock and the whole thing to be dropped in full concurrency (with some leeway)
        sleep(Duration::from_millis(
            200 + 200 * num_items as u64 / DEFAULT_DROPPER.num_threads() as u64,
        ));
        assert!(drop_thread.is_finished(), "Drop queue deadlocked.");
    }
}
