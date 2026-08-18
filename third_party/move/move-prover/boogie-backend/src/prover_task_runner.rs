// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Bounded execution of prover tasks, including racing multiple solver seeds.

use crate::options::BoogieOptions;
use async_trait::async_trait;
use futures::{stream::FuturesUnordered, StreamExt};
use log::debug;
use regex::Regex;
use std::{
    collections::BTreeMap,
    process::Output,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    process::Command,
    sync::{Notify, Semaphore},
    time::{timeout, timeout_at},
};

const MAX_PERMITS: usize = usize::MAX >> 4;

#[async_trait]
pub trait ProverTask {
    type TaskResult: Send + 'static;
    type TaskId: Send + Copy + 'static;

    /// Initialize the task runner given the number of instances.
    fn init(&mut self, num_instances: usize) -> Vec<Self::TaskId>;

    /// Run the task with `task_id` under the shared child-process limit.
    async fn run(&mut self, task_id: Self::TaskId, sem: Arc<Semaphore>) -> Self::TaskResult;

    /// Returns whether the task result is considered successful.
    fn is_success(&self, task_result: &Self::TaskResult) -> bool;

    /// Wait for the primary result even if another instance finishes first.
    fn prefer_primary(&self) -> bool {
        false
    }

    /// Delay fallback instances until this duration has elapsed. `None` starts
    /// every instance immediately when execution is not sequential.
    fn retry_delay(&self) -> Option<Duration> {
        None
    }

    /// Wait before launching a fallback instance. The default applies the
    /// configured delay from the time the task is scheduled.
    async fn wait_for_retry(&mut self, index: usize) {
        if index > 0 {
            if let Some(delay) = self.retry_delay() {
                tokio::time::sleep(delay).await;
            }
        }
    }

    /// Returns a task result used for representing a hard timeout.
    fn make_timeout(&self) -> (Self::TaskId, Self::TaskResult);
}

pub struct ProverTaskRunner();

impl ProverTaskRunner {
    /// Run seed instances for one prover task and return the first successful
    /// result, or the last result if all instances fail.
    pub fn run_tasks<T>(
        task: T,
        num_instances: usize,
        sequential: bool,
        hard_timeout_secs: u64,
    ) -> (T::TaskId, T::TaskResult)
    where
        T: ProverTask + Clone + Send + 'static,
    {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(Self::run_tasks_async(
            task,
            num_instances,
            sequential,
            hard_timeout_secs,
            Arc::new(Semaphore::new(MAX_PERMITS)),
        ))
    }

    /// Produce and consume independent prover tasks under one global child-process limit.
    /// At most `process_limit` tasks and their metadata are retained at once.
    pub fn run_task_pipeline<T, M, E, P, C>(
        task_count: usize,
        num_instances: usize,
        sequential: bool,
        hard_timeout_secs: u64,
        process_limit: usize,
        mut produce: P,
        mut consume: C,
    ) -> Result<(), E>
    where
        T: ProverTask + Clone + Send + Sync + 'static,
        P: FnMut(usize) -> Result<(T, M), E>,
        C: FnMut(usize, M, T::TaskId, T::TaskResult, Duration) -> Result<(), E>,
    {
        // Process futures run on a dedicated runtime worker so synchronous BPL
        // generation on this thread cannot starve their deadlines.
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let process_limit = process_limit.max(1);
        let process_sem = Arc::new(Semaphore::new(process_limit));
        rt.block_on(async {
            let mut next_index = 0;
            let mut pending: FuturesUnordered<
                tokio::task::JoinHandle<(usize, T::TaskId, T::TaskResult, Duration)>,
            > = FuturesUnordered::new();
            let mut metadata_by_index = BTreeMap::new();
            loop {
                while next_index < task_count && pending.len() < process_limit {
                    let index = next_index;
                    let (task, metadata) = match produce(index) {
                        Ok(produced) => produced,
                        Err(err) => {
                            for handle in pending.iter() {
                                handle.abort();
                            }
                            return Err(err);
                        },
                    };
                    metadata_by_index.insert(index, metadata);
                    let process_sem = process_sem.clone();
                    pending.push(tokio::spawn(async move {
                        let start = Instant::now();
                        let (task_id, result) = Self::run_tasks_async(
                            task,
                            num_instances,
                            sequential,
                            hard_timeout_secs,
                            process_sem,
                        )
                        .await;
                        (index, task_id, result, start.elapsed())
                    }));
                    next_index += 1;
                }
                let Some(completed) = pending.next().await else {
                    break;
                };
                let (index, task_id, result, duration) =
                    completed.expect("prover task execution panicked");
                let metadata = metadata_by_index
                    .remove(&index)
                    .expect("missing prover task metadata");
                if let Err(err) = consume(index, metadata, task_id, result, duration) {
                    for handle in pending.iter() {
                        handle.abort();
                    }
                    return Err(err);
                }
            }
            Ok(())
        })
    }

    async fn run_tasks_async<T>(
        mut task: T,
        num_instances: usize,
        sequential: bool,
        hard_timeout_secs: u64,
        process_sem: Arc<Semaphore>,
    ) -> (T::TaskId, T::TaskResult)
    where
        T: ProverTask + Clone + Send + 'static,
    {
        let task_ids = task.init(num_instances);
        let prefer_primary = task.prefer_primary();
        let run = async {
            if sequential {
                let mut last_result = None;
                for task_id in task_ids {
                    let mut cloned_task = task.clone();
                    let result = cloned_task.run(task_id, process_sem.clone()).await;
                    if task.is_success(&result) {
                        return (task_id, result);
                    }
                    last_result = Some((task_id, result));
                    debug!("previous instance failed, trying the next worker...");
                }
                return last_result.expect("a prover task must initialize at least one instance");
            }

            let mut pending = FuturesUnordered::new();
            for (index, task_id) in task_ids.into_iter().enumerate() {
                let mut cloned_task = task.clone();
                let process_sem = process_sem.clone();
                pending.push(async move {
                    cloned_task.wait_for_retry(index).await;
                    let result = cloned_task.run(task_id, process_sem).await;
                    (index, task_id, result)
                });
            }
            let mut remaining = pending.len();
            let mut primary_failed = false;
            let mut fallback_success = None;
            while let Some((index, task_id, result)) = pending.next().await {
                remaining = remaining.saturating_sub(1);
                let success = task.is_success(&result);
                if !prefer_primary && (remaining == 0 || success) {
                    return (task_id, result);
                }
                if prefer_primary {
                    if index == 0 {
                        if success {
                            return (task_id, result);
                        }
                        primary_failed = true;
                        if let Some(fallback) = fallback_success {
                            return fallback;
                        }
                    } else if success {
                        if primary_failed {
                            return (task_id, result);
                        }
                        fallback_success.get_or_insert((task_id, result));
                        if remaining == 0 {
                            return fallback_success.expect("successful fallback");
                        }
                        continue;
                    }
                    if remaining == 0 {
                        return fallback_success.unwrap_or((task_id, result));
                    }
                }
                debug!("previous instance failed, waiting for another worker to report...");
            }
            unreachable!("a prover task must initialize at least one instance")
        };

        if hard_timeout_secs == 0 {
            run.await
        } else {
            match timeout(Duration::from_secs(hard_timeout_secs), run).await {
                Ok(result) => result,
                Err(_) => {
                    debug!(
                        "prover task exceeded hard timeout of {}s",
                        hard_timeout_secs
                    );
                    task.make_timeout()
                },
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct RunBoogieWithSeeds {
    pub options: BoogieOptions,
    pub boogie_file: String,
    pub prefer_primary: bool,
    /// Process deadline after acquiring the global process permit. Zero means
    /// no per-process deadline.
    pub process_timeout_secs: u64,
    /// Absolute deadline for all seeds of this job.
    pub process_deadline: Option<Instant>,
    /// Delay before starting a fallback seed for the implicit retry policy.
    pub seed_handoff_after: Option<Duration>,
    /// Signals that the primary process has acquired a process slot.
    pub primary_started: Arc<Notify>,
}

#[async_trait]
impl ProverTask for RunBoogieWithSeeds {
    type TaskId = usize;
    type TaskResult = std::io::Result<Output>;

    fn init(&mut self, num_instances: usize) -> Vec<Self::TaskId> {
        // If we are running only one Boogie instance, use the default random seed.
        if num_instances == 1 {
            return vec![self.options.random_seed];
        }
        // Keep retries reproducible across runs.
        (0..num_instances)
            .map(|offset| self.options.random_seed.wrapping_add(offset))
            .collect()
    }

    async fn run(&mut self, task_id: Self::TaskId, sem: Arc<Semaphore>) -> Self::TaskResult {
        let _guard = sem.acquire().await;
        if task_id == self.options.random_seed {
            // Wake delayed fallback seeds even when the package deadline has
            // already expired. They then observe the same deadline and finish
            // instead of waiting forever for a primary process that will not run.
            self.primary_started.notify_one();
        } else if let Some(delay) = self.seed_handoff_after {
            log::info!(
                "primary Boogie seed used {:.1}s; starting fallback seed {}",
                delay.as_secs_f64(),
                task_id
            );
        }
        if self
            .process_deadline
            .is_some_and(|deadline| deadline <= Instant::now())
        {
            return Err(std::io::Error::from(std::io::ErrorKind::TimedOut));
        }
        let args = self
            .get_boogie_command(task_id)
            .map_err(std::io::Error::other)?;
        debug!("running Boogie command with seed {}", task_id);
        let process = Command::new(&args[0])
            .args(&args[1..])
            .kill_on_drop(true)
            .output();
        let timeout_deadline = self
            .process_deadline
            .map(tokio::time::Instant::from_std)
            .into_iter()
            .chain((self.process_timeout_secs != 0).then(|| {
                tokio::time::Instant::now() + Duration::from_secs(self.process_timeout_secs)
            }))
            .min();
        if let Some(deadline) = timeout_deadline {
            timeout_at(deadline, process)
                .await
                .unwrap_or_else(|_| Err(std::io::Error::from(std::io::ErrorKind::TimedOut)))
        } else {
            process.await
        }
    }

    fn is_success(&self, task_result: &Self::TaskResult) -> bool {
        match task_result {
            Ok(res) => {
                if !res.status.success() {
                    return false;
                }
                let output = String::from_utf8_lossy(&res.stdout);
                self.contains_compilation_error(&output) || !self.contains_timeout(&output)
            },
            // Infrastructure failures terminate the race, but a timed-out seed
            // leaves another deterministic seed a chance to complete.
            Err(err) => err.kind() != std::io::ErrorKind::TimedOut,
        }
    }

    fn prefer_primary(&self) -> bool {
        self.prefer_primary
    }

    fn retry_delay(&self) -> Option<Duration> {
        self.seed_handoff_after
    }

    async fn wait_for_retry(&mut self, index: usize) {
        if index > 0 {
            if let Some(delay) = self.seed_handoff_after {
                self.primary_started.notified().await;
                tokio::time::sleep(delay).await;
            }
        }
    }

    fn make_timeout(&self) -> (Self::TaskId, Self::TaskResult) {
        (0, Err(std::io::Error::from(std::io::ErrorKind::TimedOut)))
    }
}

impl RunBoogieWithSeeds {
    /// Returns command line to call boogie.
    pub fn get_boogie_command(&mut self, seed: usize) -> anyhow::Result<Vec<String>> {
        self.options
            .boogie_flags
            .push(format!("-proverOpt:O:smt.random_seed={}", seed));
        self.options.get_boogie_command(&self.boogie_file)
    }

    /// Returns whether the output string contains any Boogie compilation errors.
    fn contains_compilation_error(&self, output: &str) -> bool {
        let regex =
            Regex::new(r"(?m)^.*\((?P<line>\d+),(?P<col>\d+)\).*(Error:|error:).*$").unwrap();
        regex.is_match(output)
    }

    /// Returns whether the output string contains any Boogie timeouts/inconclusiveness.
    fn contains_timeout(&self, output: &str) -> bool {
        let regex =
            Regex::new(r"(?m)^.*\((?P<line>\d+),(?P<col>\d+)\).*Verification.*(inconclusive|out of resource|timed out).*$")
                .unwrap();
        regex.is_match(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    };

    #[derive(Clone)]
    struct TestTask {
        job: usize,
        active: Arc<AtomicUsize>,
        max_active: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl ProverTask for TestTask {
        type TaskId = usize;
        type TaskResult = usize;

        fn init(&mut self, _num_instances: usize) -> Vec<Self::TaskId> {
            vec![0]
        }

        async fn run(&mut self, _task_id: Self::TaskId, sem: Arc<Semaphore>) -> Self::TaskResult {
            let _permit = sem.acquire().await.expect("semaphore closed");
            let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.max_active.fetch_max(active, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis((20 - self.job) as u64)).await;
            self.active.fetch_sub(1, Ordering::SeqCst);
            self.job
        }

        fn is_success(&self, _task_result: &Self::TaskResult) -> bool {
            true
        }

        fn make_timeout(&self) -> (Self::TaskId, Self::TaskResult) {
            (0, usize::MAX)
        }
    }

    #[test]
    fn pipeline_bounds_retained_metadata() {
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));
        let retained = Arc::new(AtomicUsize::new(0));
        let max_retained = Arc::new(AtomicUsize::new(0));
        let consumed = Arc::new(AtomicUsize::new(0));
        let result = ProverTaskRunner::run_task_pipeline(
            20,
            1,
            false,
            0,
            3,
            {
                let active = active.clone();
                let max_active = max_active.clone();
                let retained = retained.clone();
                let max_retained = max_retained.clone();
                move |job| {
                    let count = retained.fetch_add(1, Ordering::SeqCst) + 1;
                    max_retained.fetch_max(count, Ordering::SeqCst);
                    Ok::<_, ()>((
                        TestTask {
                            job,
                            active: active.clone(),
                            max_active: max_active.clone(),
                        },
                        job,
                    ))
                }
            },
            {
                let retained = retained.clone();
                let consumed = consumed.clone();
                move |index, metadata, _task_id, task_result, _duration| {
                    assert_eq!(index, metadata);
                    assert_eq!(metadata, task_result);
                    retained.fetch_sub(1, Ordering::SeqCst);
                    consumed.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, ()>(())
                }
            },
        );
        assert_eq!(result, Ok(()));
        assert_eq!(consumed.load(Ordering::SeqCst), 20);
        assert_eq!(retained.load(Ordering::SeqCst), 0);
        assert_eq!(max_retained.load(Ordering::SeqCst), 3);
        assert_eq!(max_active.load(Ordering::SeqCst), 3);
    }

    #[derive(Clone)]
    struct DeadlineTask {
        job: usize,
        started: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl ProverTask for DeadlineTask {
        type TaskId = usize;
        type TaskResult = usize;

        fn init(&mut self, _num_instances: usize) -> Vec<Self::TaskId> {
            vec![0]
        }

        async fn run(&mut self, _task_id: usize, sem: Arc<Semaphore>) -> usize {
            let _permit = sem.acquire().await.expect("semaphore closed");
            if self.job == 0 {
                self.started.store(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            self.job
        }

        fn is_success(&self, _task_result: &usize) -> bool {
            true
        }

        fn make_timeout(&self) -> (usize, usize) {
            (0, usize::MAX)
        }
    }

    #[test]
    fn pipeline_generation_does_not_starve_task_deadline() {
        let started = Arc::new(AtomicUsize::new(0));
        let results = Arc::new(Mutex::new(BTreeMap::new()));
        ProverTaskRunner::run_task_pipeline(
            2,
            1,
            false,
            1,
            2,
            {
                let started = started.clone();
                move |job| {
                    if job == 1 {
                        let wait_start = Instant::now();
                        while started.load(Ordering::SeqCst) == 0 {
                            assert!(wait_start.elapsed() < Duration::from_secs(1));
                            std::thread::yield_now();
                        }
                        std::thread::sleep(Duration::from_millis(1_200));
                    }
                    Ok::<_, ()>((
                        DeadlineTask {
                            job,
                            started: started.clone(),
                        },
                        (),
                    ))
                }
            },
            {
                let results = results.clone();
                move |index, (), _task_id, result, _duration| {
                    results.lock().unwrap().insert(index, result);
                    Ok::<_, ()>(())
                }
            },
        )
        .unwrap();
        assert_eq!(results.lock().unwrap().get(&0), Some(&usize::MAX));
    }

    #[test]
    fn seed_retries_are_deterministic() {
        let options = BoogieOptions {
            random_seed: 7,
            ..Default::default()
        };
        let mut task = RunBoogieWithSeeds {
            options,
            boogie_file: "test.bpl".to_string(),
            prefer_primary: false,
            process_timeout_secs: 0,
            process_deadline: None,
            seed_handoff_after: None,
            primary_started: Arc::new(Notify::new()),
        };
        assert_eq!(task.init(3), vec![7, 8, 9]);
    }

    #[test]
    fn delayed_seed_waits_for_primary_process_start() {
        let task = RunBoogieWithSeeds {
            options: BoogieOptions::default(),
            boogie_file: "test.bpl".to_string(),
            prefer_primary: false,
            process_timeout_secs: 0,
            process_deadline: None,
            seed_handoff_after: Some(Duration::from_millis(20)),
            primary_started: Arc::new(Notify::new()),
        };
        let started = task.primary_started.clone();
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(async move {
                let mut fallback = task;
                let wait = fallback.wait_for_retry(1);
                tokio::pin!(wait);
                assert!(tokio::time::timeout(Duration::from_millis(5), &mut wait)
                    .await
                    .is_err());

                let now = Instant::now();
                started.notify_one();
                wait.await;
                assert!(now.elapsed() >= Duration::from_millis(10));
            });
    }

    #[derive(Clone)]
    struct PreferenceTask(bool);

    #[async_trait]
    impl ProverTask for PreferenceTask {
        type TaskId = usize;
        type TaskResult = usize;

        fn init(&mut self, _num_instances: usize) -> Vec<Self::TaskId> {
            vec![0, 1]
        }

        async fn run(&mut self, task_id: usize, sem: Arc<Semaphore>) -> usize {
            let _permit = sem.acquire().await.expect("semaphore closed");
            tokio::time::sleep(Duration::from_millis(if task_id == 0 { 20 } else { 1 })).await;
            task_id
        }

        fn is_success(&self, _result: &usize) -> bool {
            true
        }

        fn prefer_primary(&self) -> bool {
            self.0
        }

        fn make_timeout(&self) -> (usize, usize) {
            (0, usize::MAX)
        }
    }

    #[test]
    fn primary_result_can_be_preferred() {
        assert_eq!(
            ProverTaskRunner::run_tasks(PreferenceTask(true), 2, false, 0).1,
            0
        );
        assert_eq!(
            ProverTaskRunner::run_tasks(PreferenceTask(false), 2, false, 0).1,
            1
        );
    }

    #[derive(Clone)]
    struct HedgedRetryTask(Arc<Mutex<Vec<usize>>>);

    #[async_trait]
    impl ProverTask for HedgedRetryTask {
        type TaskId = usize;
        type TaskResult = usize;

        fn init(&mut self, _num_instances: usize) -> Vec<Self::TaskId> {
            vec![0, 1]
        }

        async fn run(&mut self, task_id: usize, sem: Arc<Semaphore>) -> usize {
            let _permit = sem.acquire().await.expect("semaphore closed");
            self.0.lock().unwrap().push(task_id);
            tokio::time::sleep(Duration::from_millis(if task_id == 0 { 100 } else { 1 })).await;
            task_id
        }

        fn is_success(&self, _result: &usize) -> bool {
            true
        }

        fn retry_delay(&self) -> Option<Duration> {
            Some(Duration::from_millis(10))
        }

        fn make_timeout(&self) -> (usize, usize) {
            (0, usize::MAX)
        }
    }

    #[test]
    fn hedged_retry_starts_after_delay() {
        let attempts = Arc::new(Mutex::new(vec![]));
        let task = HedgedRetryTask(attempts.clone());
        assert_eq!(ProverTaskRunner::run_tasks(task, 2, false, 0).1, 1);
        assert_eq!(*attempts.lock().unwrap(), vec![0, 1]);
    }

    #[derive(Clone)]
    struct SequentialRetryTask {
        primary_succeeds: bool,
        attempts: Arc<Mutex<Vec<usize>>>,
    }

    #[async_trait]
    impl ProverTask for SequentialRetryTask {
        type TaskId = usize;
        type TaskResult = usize;

        fn init(&mut self, _num_instances: usize) -> Vec<Self::TaskId> {
            vec![0, 1]
        }

        async fn run(&mut self, task_id: usize, _sem: Arc<Semaphore>) -> usize {
            self.attempts.lock().unwrap().push(task_id);
            task_id
        }

        fn is_success(&self, result: &usize) -> bool {
            self.primary_succeeds || *result == 1
        }

        fn make_timeout(&self) -> (usize, usize) {
            (0, usize::MAX)
        }
    }

    #[test]
    fn sequential_retry_runs_only_after_primary_failure() {
        let attempts = Arc::new(Mutex::new(vec![]));
        let success = SequentialRetryTask {
            primary_succeeds: true,
            attempts: attempts.clone(),
        };
        assert_eq!(ProverTaskRunner::run_tasks(success, 2, true, 0).1, 0);
        assert_eq!(*attempts.lock().unwrap(), vec![0]);

        attempts.lock().unwrap().clear();
        let failure = SequentialRetryTask {
            primary_succeeds: false,
            attempts: attempts.clone(),
        };
        assert_eq!(ProverTaskRunner::run_tasks(failure, 2, true, 0).1, 1);
        assert_eq!(*attempts.lock().unwrap(), vec![0, 1]);
    }

    #[test]
    fn a_process_timeout_does_not_win_a_seed_race() {
        let task = RunBoogieWithSeeds {
            options: BoogieOptions::default(),
            boogie_file: "test.bpl".to_string(),
            prefer_primary: false,
            process_timeout_secs: 1,
            process_deadline: None,
            seed_handoff_after: None,
            primary_started: Arc::new(Notify::new()),
        };
        assert!(!task.is_success(&Err(std::io::Error::from(std::io::ErrorKind::TimedOut))));
        assert!(task.is_success(&Err(std::io::Error::other("cannot start Boogie"))));
    }

    #[test]
    fn package_deadline_prevents_late_seed() {
        let mut task = RunBoogieWithSeeds {
            options: BoogieOptions::default(),
            boogie_file: "test.bpl".to_string(),
            prefer_primary: false,
            process_timeout_secs: 60,
            process_deadline: Some(Instant::now() - Duration::from_secs(1)),
            seed_handoff_after: None,
            primary_started: Arc::new(Notify::new()),
        };
        let result = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(task.run(0, Arc::new(Semaphore::new(1))));
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::TimedOut);
    }

    #[test]
    fn expired_primary_wakes_delayed_fallback() {
        let task = RunBoogieWithSeeds {
            options: BoogieOptions::default(),
            boogie_file: "test.bpl".to_string(),
            prefer_primary: false,
            process_timeout_secs: 60,
            process_deadline: Some(Instant::now() - Duration::from_secs(1)),
            seed_handoff_after: Some(Duration::from_millis(1)),
            primary_started: Arc::new(Notify::new()),
        };
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(async move {
                let mut primary = task.clone();
                assert_eq!(
                    primary
                        .run(primary.options.random_seed, Arc::new(Semaphore::new(1)))
                        .await
                        .unwrap_err()
                        .kind(),
                    std::io::ErrorKind::TimedOut
                );
                let mut fallback = task;
                assert!(tokio::time::timeout(
                    Duration::from_millis(50),
                    fallback.wait_for_retry(1)
                )
                .await
                .is_ok());
            });
    }
}
