// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::pruner::db_pruner::DBPruner;
use aptos_logger::{
    error,
    prelude::{sample, SampleRate},
};
use aptos_types::transaction::Version;
use std::{
    sync::{
        mpsc::{self, Receiver, SyncSender, TryRecvError},
        Arc,
    },
    thread::{sleep, JoinHandle},
    time::Duration,
};

/// Maintains the pruner and periodically calls the db_pruner's prune method to prune the DB.
/// This also exposes API to report the progress to the parent thread.
pub struct PrunerWorker {
    // The name of the worker.
    worker_name: String,
    /// The thread to run pruner.
    worker_thread: Option<JoinHandle<()>>,
    /// The pruner.
    pruner: Arc<dyn DBPruner>,
    /// Sending `()` wakes the worker. Dropping the sender signals quit.
    wake_sender: Option<SyncSender<()>>,
}

impl PrunerWorker {
    pub(crate) fn new(pruner: Arc<dyn DBPruner>, batch_size: usize, name: &str) -> Self {
        let (wake_sender, wake_receiver) = mpsc::sync_channel(1);
        let pruner_clone = Arc::clone(&pruner);

        let worker_thread = std::thread::Builder::new()
            .name(format!("{name}_pruner"))
            .spawn(move || Self::work(pruner_clone, batch_size, wake_receiver))
            .expect("Creating pruner thread should succeed.");

        Self {
            worker_name: name.into(),
            worker_thread: Some(worker_thread),
            pruner,
            wake_sender: Some(wake_sender),
        }
    }

    pub fn set_target_db_version(&self, target_db_version: Version) {
        if target_db_version > self.pruner.target_version() {
            self.pruner.set_target_version(target_db_version);
            // Wake the worker. If the channel buffer is full, the worker is already
            // active and will see the new target on its next `is_pruning_pending` check.
            if let Some(ref sender) = self.wake_sender {
                let _ = sender.try_send(());
            }
        }
    }

    #[allow(dead_code)]
    pub fn is_pruning_pending(&self) -> bool {
        self.pruner.is_pruning_pending()
    }

    fn work(pruner: Arc<dyn DBPruner>, batch_size: usize, wake_receiver: Receiver<()>) {
        loop {
            // Check for quit (channel closed) before doing more work.
            if matches!(wake_receiver.try_recv(), Err(TryRecvError::Disconnected)) {
                break;
            }
            if let Err(err) = pruner.prune(batch_size) {
                sample!(
                    SampleRate::Duration(Duration::from_secs(1)),
                    error!(error = ?err, "Pruner has error.")
                );
                sleep(Duration::from_millis(100));
                continue;
            }
            if !pruner.is_pruning_pending() {
                // Block until notified of new work or channel is closed (quit).
                if wake_receiver.recv().is_err() {
                    break;
                }
            }
        }
    }
}

impl Drop for PrunerWorker {
    fn drop(&mut self) {
        // Close the channel to signal the worker to quit.
        self.wake_sender.take();
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
