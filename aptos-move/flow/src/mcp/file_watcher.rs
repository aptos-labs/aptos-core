// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! File watcher for automatic package cache invalidation.
//!
//! Monitors directories containing Move source files and detects changes
//! (modifications, additions, deletions) that should trigger recompilation.
//!
//! The watcher callback resolves affected cache keys immediately and invokes
//! a caller-provided invalidation callback (typically removing the entry from
//! the package cache). There is no internal event queue, so no events can be
//! lost regardless of load.

use move_command_line_common::env::MOVE_HOME;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

/// Shared state accessed by both the watcher callback and the public API.
///
/// Lock ordering: always acquire `state` before any external lock (e.g. the
/// package cache) to avoid deadlocks. The watcher callback collects affected
/// keys under the `state` lock, drops it, then calls the invalidation callback.
struct WatchState {
    /// Maps a watched directory to the set of package cache keys that depend on it.
    dir_to_keys: HashMap<PathBuf, HashSet<String>>,
}

/// Watches source directories for file changes and maps them to package cache keys.
///
/// Uses `notify::RecommendedWatcher` (OS-native: FSEvents on macOS, inotify on Linux)
/// to receive file system events. The watcher callback resolves each event path to
/// affected cache keys immediately and invokes the `on_invalidate` callback for each.
#[derive(Clone)]
pub(crate) struct FileWatcher {
    inner: Arc<FileWatcherInner>,
}

struct FileWatcherInner {
    /// The OS file watcher. Kept alive so watches remain active.
    watcher: Mutex<RecommendedWatcher>,
    /// Shared state: directory→keys mapping.
    state: Arc<Mutex<WatchState>>,
}

impl FileWatcher {
    /// Create a new file watcher.
    ///
    /// `on_invalidate` is called (from the OS watcher thread) for each cache key
    /// whose source files have changed. Typically this removes the entry from the
    /// package cache.
    pub(crate) fn new(on_invalidate: Arc<dyn Fn(&str) + Send + Sync>) -> notify::Result<Self> {
        let state = Arc::new(Mutex::new(WatchState {
            dir_to_keys: HashMap::new(),
        }));
        let cb_state = Arc::clone(&state);
        let watcher = RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    // Collect affected keys under the state lock, then drop it
                    // before calling on_invalidate (which may acquire other locks).
                    let keys: HashSet<String> = {
                        let st = cb_state.lock().expect("watch_state lock poisoned");
                        let mut keys = HashSet::new();
                        for path in &event.paths {
                            let path = canonicalize_or_keep(path);
                            if let Some(k) = st.dir_to_keys.get(&path) {
                                keys.extend(k.iter().cloned());
                            }
                            if let Some(parent) = path.parent() {
                                if let Some(k) = st.dir_to_keys.get(parent) {
                                    keys.extend(k.iter().cloned());
                                }
                            }
                        }
                        keys
                    };
                    for key in &keys {
                        log::debug!("file change detected for `{}`", key);
                        on_invalidate(key);
                    }
                }
            },
            notify::Config::default(),
        )?;
        Ok(Self {
            inner: Arc::new(FileWatcherInner {
                watcher: Mutex::new(watcher),
                state,
            }),
        })
    }

    /// Register directory watches for a package.
    ///
    /// Watches the `package_root` directory (for `Move.toml` changes) and the parent
    /// directories of all `source_files` (for modifications and new file additions).
    /// All paths are canonicalized to ensure consistent matching with OS-reported events
    /// (e.g. macOS resolves `/var` → `/private/var`).
    pub(crate) fn watch_package(
        &self,
        cache_key: &str,
        package_root: &Path,
        source_files: &[String],
    ) -> usize {
        let mut dirs: HashSet<PathBuf> = HashSet::new();
        dirs.insert(canonicalize_or_keep(package_root));
        for file in source_files {
            if let Some(parent) = Path::new(file).parent() {
                dirs.insert(canonicalize_or_keep(parent));
            }
        }
        let move_home = Path::new(MOVE_HOME.as_str());
        dirs.retain(|dir| {
            if dir.starts_with(move_home) {
                log::debug!("skipping downloaded dependency dir `{}`", dir.display());
                false
            } else {
                true
            }
        });
        let mut watcher = self
            .inner
            .watcher
            .lock()
            .expect("file_watcher lock poisoned");
        let mut state = self.inner.state.lock().expect("watch_state lock poisoned");
        for dir in &dirs {
            if !state.dir_to_keys.contains_key(dir) {
                log::debug!("watching directory `{}`", dir.display());
                let _ = watcher.watch(dir, RecursiveMode::NonRecursive);
            }
            state
                .dir_to_keys
                .entry(dir.clone())
                .or_default()
                .insert(cache_key.to_string());
        }
        dirs.len()
    }

    /// Remove all watches associated with a cache key.
    ///
    /// Directories that no longer have any associated keys are unwatched.
    pub(crate) fn unwatch_package(&self, cache_key: &str) {
        let mut state = self.inner.state.lock().expect("watch_state lock poisoned");
        let mut watcher = self
            .inner
            .watcher
            .lock()
            .expect("file_watcher lock poisoned");
        state.dir_to_keys.retain(|dir, keys| {
            keys.remove(cache_key);
            if keys.is_empty() {
                let _ = watcher.unwatch(dir);
                false
            } else {
                true
            }
        });
    }
}

/// Canonicalize a path, falling back to the original if canonicalization fails
/// (e.g. because the path doesn't exist yet).
fn canonicalize_or_keep(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, time::Duration};
    use tempfile::TempDir;

    /// Small delay to let OS file events propagate.
    async fn settle() {
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    /// Create a `FileWatcher` whose callback records invalidated keys.
    fn make_watcher() -> (FileWatcher, Arc<Mutex<HashSet<String>>>) {
        let invalidated = Arc::new(Mutex::new(HashSet::<String>::new()));
        let inv = Arc::clone(&invalidated);
        let watcher = FileWatcher::new(Arc::new(move |key: &str| {
            inv.lock().unwrap().insert(key.to_string());
        }))
        .unwrap();
        (watcher, invalidated)
    }

    #[tokio::test]
    async fn detects_file_modification() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.move");
        fs::write(&file, "original").unwrap();

        let (watcher, invalidated) = make_watcher();
        watcher.watch_package("pkg1", dir.path(), &[file.to_string_lossy().into_owned()]);

        fs::write(&file, "modified").unwrap();
        settle().await;

        let inv = invalidated.lock().unwrap();
        assert!(inv.contains("pkg1"), "expected pkg1 in {:?}", *inv);
    }

    #[tokio::test]
    async fn detects_new_file_in_directory() {
        let dir = TempDir::new().unwrap();
        let existing = dir.path().join("existing.move");
        fs::write(&existing, "content").unwrap();

        let (watcher, invalidated) = make_watcher();
        watcher.watch_package("pkg1", dir.path(), &[existing
            .to_string_lossy()
            .into_owned()]);

        // Add a brand-new file to the watched directory.
        fs::write(dir.path().join("new_module.move"), "module 0x1::m {}").unwrap();
        settle().await;

        let inv = invalidated.lock().unwrap();
        assert!(inv.contains("pkg1"), "expected pkg1 in {:?}", *inv);
    }

    #[tokio::test]
    async fn no_cross_package_invalidation() {
        let dir1 = TempDir::new().unwrap();
        let dir2 = TempDir::new().unwrap();
        let file1 = dir1.path().join("a.move");
        let file2 = dir2.path().join("b.move");
        fs::write(&file1, "a").unwrap();
        fs::write(&file2, "b").unwrap();

        let (watcher, invalidated) = make_watcher();
        watcher.watch_package("pkg1", dir1.path(), &[file1.to_string_lossy().into_owned()]);
        watcher.watch_package("pkg2", dir2.path(), &[file2.to_string_lossy().into_owned()]);

        // Only modify file2.
        fs::write(&file2, "modified").unwrap();
        settle().await;

        let inv = invalidated.lock().unwrap();
        assert!(
            !inv.contains("pkg1"),
            "pkg1 should not be invalidated: {:?}",
            *inv
        );
        assert!(inv.contains("pkg2"), "expected pkg2 in {:?}", *inv);
    }

    #[tokio::test]
    async fn unwatch_stops_tracking() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.move");
        fs::write(&file, "original").unwrap();

        let (watcher, invalidated) = make_watcher();
        watcher.watch_package("pkg1", dir.path(), &[file.to_string_lossy().into_owned()]);
        watcher.unwatch_package("pkg1");

        fs::write(&file, "modified").unwrap();
        settle().await;

        let inv = invalidated.lock().unwrap();
        assert!(
            inv.is_empty(),
            "expected empty after unwatch, got {:?}",
            *inv
        );
    }
}
