// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use fs2::FileExt;
use futures::FutureExt;
use std::{
    fs::{self, File},
    mem,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::{pin, select, task};

/// A file-based lock to ensure exclusive access to certain resources.
///
/// This is used by the package cache to ensure only one process can mutate a cached repo, checkout,
/// or on-chain package at a time.
pub struct FileLock {
    file: Option<File>,
    path: PathBuf,
}

impl FileLock {
    /// Attempts to acquire an exclusive `FileLock`, with an optional alert callback.
    ///
    /// If the lock cannot be acquired within `alert_timeout`, the `alert_on_wait` callback
    /// is executed to notify the caller.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let lock = FileLock::lock_with_alert_on_wait(
    ///     "/tmp/my-lock",
    ///     Duration::from_secs(1),
    ///     || println!("Waiting for lock to be released...")
    /// ).await?;
    /// ```
    pub async fn lock_with_alert_on_wait<P, F>(
        lock_path: P,
        alert_timeout: Duration,
        alert_on_wait: F,
    ) -> Result<Self>
    where
        P: AsRef<Path>,
        F: FnOnce(),
    {
        let lock_path = lock_path.as_ref().to_owned();

        let lock_fut = {
            let lock_path = lock_path.clone();

            task::spawn_blocking(move || -> Result<File> {
                let lock_file = File::create(&lock_path)?;
                lock_file.lock_exclusive()?;
                Ok(lock_file)
            })
        };

        let timeout = tokio::time::sleep(alert_timeout).fuse();

        pin!(lock_fut, timeout);

        let lock_file = select! {
            _ = &mut timeout => {
                alert_on_wait();
                lock_fut.await??
            },
            res = &mut lock_fut => res??,
        };

        Ok(Self {
            file: Some(lock_file),
            path: lock_path,
        })
    }
}

impl Drop for FileLock {
    /// Automatically releases the lock and removes the lock file when dropped.
    /// This makes the lock easy to use -- exclusive access is guaranteed as long as the lock is alive.
    fn drop(&mut self) {
        let file = self.file.take().expect("this should always succeed");
        mem::drop(file);
        _ = fs::remove_file(&self.path); // Best effort
    }
}
