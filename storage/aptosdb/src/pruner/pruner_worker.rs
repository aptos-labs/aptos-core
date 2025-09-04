// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::pruner::db_pruner::DBPruner;
use aptos_logger::{
    error,
    prelude::{SampleRate, sample},
};
use aptos_types::transaction::Version;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{JoinHandle, sleep},
    time::Duration,
};

/// Maintains the pruner and periodically calls the db_pruner's prune method to prune the DB.
/// This also exposes API to report the progress to the parent thread.
pub struct PrunerWorker {
    // The name of the worker.
    worker_name: String,
    /// The thread to run pruner.
    worker_thread: Option<JoinHandle<()>>,

    inner: Arc<PrunerWorkerInner>,
}

pub struct PrunerWorkerInner {
    /// The worker will sleep for this period of time after pruning each batch.
    pruning_time_interval_in_ms: u64,
    /// The pruner.
    pruner: Arc<dyn DBPruner>,
    /// A threshold to control how many items we prune for each batch.
    batch_size: usize,
    /// Indicates whether the pruning loop should be running. Will only be set to true on pruner
    /// destruction.
    quit_worker: AtomicBool,
}

impl PrunerWorkerInner {
    fn new(pruner: Arc<dyn DBPruner>, batch_size: usize) -> Arc<Self> {
        Arc::new(Self {
            pruning_time_interval_in_ms: if cfg!(test) { 100 } else { 1 },
            pruner,
            batch_size,
            quit_worker: AtomicBool::new(false),
        })
    }

    // Loop that does the real pruning job.
    fn work(&self) {
        while !self.quit_worker.load(Ordering::SeqCst) {
            let pruner_result = self.pruner.prune(self.batch_size);
            if pruner_result.is_err() {
                sample!(
                    SampleRate::Duration(Duration::from_secs(1)),
                    error!(error = ?pruner_result.err().unwrap(),
                        "Pruner has error.")
                );
                sleep(Duration::from_millis(self.pruning_time_interval_in_ms));
                continue;
            }
            if !self.pruner.is_pruning_pending() {
                sleep(Duration::from_millis(self.pruning_time_interval_in_ms));
            }
        }
    }

    fn stop_pruning(&self) {
        self.quit_worker.store(true, Ordering::SeqCst);
    }
}

impl PrunerWorker {
    pub(crate) fn new(pruner: Arc<dyn DBPruner>, batch_size: usize, name: &str) -> Self {
        let inner = PrunerWorkerInner::new(pruner, batch_size);
        let inner_cloned = Arc::clone(&inner);

        let worker_thread = std::thread::Builder::new()
            .name(format!("{name}_pruner"))
            .spawn(move || inner_cloned.work())
            .expect("Creating pruner thread should succeed.");

        Self {
            worker_name: name.into(),
            worker_thread: Some(worker_thread),
            inner,
        }
    }

    pub fn set_target_db_version(&self, target_db_version: Version) {
        if target_db_version > self.inner.pruner.target_version() {
            self.inner.pruner.set_target_version(target_db_version);
        }
    }

    pub fn is_pruning_pending(&self) -> bool {
        self.inner.pruner.is_pruning_pending()
    }
}

impl Drop for PrunerWorker {
    fn drop(&mut self) {
        self.inner.stop_pruning();
        self.worker_thread
            .take()
            .unwrap_or_else(|| panic!("Pruner worker ({}) thread must exist.", self.worker_name))
            .join()
            .unwrap_or_else(|e| {
                panic!(
                    "Pruner worker ({}) thread should join peacefully: {e:?}",
                    self.worker_name
                )
            });
    }
}
