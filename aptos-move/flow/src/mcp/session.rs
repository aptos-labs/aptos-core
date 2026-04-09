// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{
    common::format_error_chain, file_watcher::FileWatcher, package_data::PackageData, McpArgs,
};
use crate::GlobalOpts;
use rmcp::{
    handler::server::router::tool::ToolRouter,
    model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo},
    tool_handler, ServerHandler,
};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

/// MCP session holding a package data cache.
#[derive(Clone)]
pub(crate) struct FlowSession {
    #[allow(dead_code)]
    global: GlobalOpts,
    args: McpArgs,
    /// Cache of compiled packages. `Mutex<PackageData>` is needed because `GlobalEnv`
    /// is `!Sync` (it uses `RefCell` internally).
    package_cache: Arc<Mutex<BTreeMap<String, Arc<Mutex<PackageData>>>>>,
    file_watcher: FileWatcher,
    tool_router: ToolRouter<Self>,
    /// Session-scoped temp directory, automatically deleted on drop.
    temp_dir: Arc<tempfile::TempDir>,
}

impl FlowSession {
    /// Combined router for all registered MCP tools.
    ///
    /// Add new tool routers here — this is the single source of truth used by
    /// both `new()` and `tool_names()`.
    fn all_tool_routers() -> ToolRouter<Self> {
        Self::package_manifest_router()
            + Self::package_query_router()
            + Self::package_spec_infer_router()
            + Self::package_status_router()
            + Self::package_test_router()
            + Self::package_verify_router()
    }

    /// Returns the names of all registered MCP tools.
    /// Used by the plugin renderer to validate tool references in templates.
    pub(crate) fn tool_names() -> Vec<String> {
        Self::all_tool_routers()
            .list_all()
            .into_iter()
            .map(|t| t.name.to_string())
            .collect()
    }

    /// Returns (name, description) pairs for all registered MCP tools.
    /// Used by the plugin renderer to generate the README.
    pub(crate) fn tool_descriptions() -> Vec<(String, String)> {
        Self::all_tool_routers()
            .list_all()
            .into_iter()
            .map(|t| {
                (
                    t.name.to_string(),
                    t.description.as_deref().unwrap_or("").to_string(),
                )
            })
            .collect()
    }

    pub(crate) fn args(&self) -> &McpArgs {
        &self.args
    }

    /// Configured tool timeout as a `Duration`.
    pub(crate) fn tool_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.args.tool_timeout)
    }

    pub(crate) fn temp_dir(&self) -> &Path {
        self.temp_dir.path()
    }

    pub(crate) fn new(args: McpArgs, global: GlobalOpts) -> Self {
        let package_cache = Arc::new(Mutex::new(BTreeMap::new()));
        let cache_ref = Arc::clone(&package_cache);
        let file_watcher = FileWatcher::new(Arc::new(move |key: &str| {
            if cache_ref
                .lock()
                .expect("package_cache lock poisoned")
                .remove(key)
                .is_some()
            {
                log::info!("invalidating cache for `{}`", key);
            }
        }))
        .expect("failed to create file watcher");
        let temp_dir =
            Arc::new(tempfile::TempDir::new().expect("failed to create session temp directory"));
        Self {
            global,
            args,
            package_cache,
            file_watcher,
            tool_router: Self::all_tool_routers(),
            temp_dir,
        }
    }

    /// Invalidate the cache entry for a package.
    ///
    /// Called after a tool timeout to ensure the next call gets a fresh
    /// `PackageData` with its own mutex, rather than deadlocking on the
    /// mutex still held by the timed-out `spawn_blocking` task.
    pub(crate) fn invalidate_package(&self, package_path: &str) {
        let key = self.resolve_package_path(package_path);
        if self
            .package_cache
            .lock()
            .expect("package_cache lock poisoned")
            .remove(&key)
            .is_some()
        {
            log::info!("invalidating cache for `{}` after timeout", key);
        }
    }

    /// Resolve and canonicalize the given package path, returning a string key.
    pub(crate) fn resolve_package_path(&self, package_path: &str) -> String {
        let path = PathBuf::from(package_path);
        path.canonicalize()
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned()
    }

    /// Resolve a package, returning `(package_data, rebuilt)`.
    ///
    /// Returns cached `PackageData` (`rebuilt = false`) or builds it on cache
    /// miss (`rebuilt = true`). Building is offloaded to `spawn_blocking` since
    /// compilation is a heavy synchronous operation that must not block the
    /// async executor.
    ///
    /// Cache entries are removed directly by the file-watcher callback,
    /// so a cache miss here means either first access or an invalidation.
    pub(crate) async fn resolve_package(
        &self,
        package_path: &str,
    ) -> Result<(Arc<Mutex<PackageData>>, bool), rmcp::ErrorData> {
        let key = self.resolve_package_path(package_path);
        {
            let cache = self
                .package_cache
                .lock()
                .expect("package_cache lock poisoned");
            if let Some(data) = cache.get(&key) {
                log::info!("cache hit for `{}`", key);
                return Ok((Arc::clone(data), false));
            }
        }

        // Cache miss — rebuild. Keep existing watches active during compilation
        // so that edits are not missed; the watcher callback will call
        // `cache.remove(key)` which is a no-op while there is no cache entry.
        let build_start = std::time::SystemTime::now();

        log::info!("building package `{}`", key);
        let args = self.args.clone();
        let key_clone = key.clone();
        let data =
            tokio::task::spawn_blocking(move || PackageData::init(key_clone.as_ref(), &args))
                .await
                .map_err(|e| {
                    rmcp::ErrorData::internal_error(format!("build task failed: {}", e), None)
                })?
                .map_err(|e| {
                    let msg = format_error_chain(&e);
                    log::info!("build failed for `{}`: {}", key, msg);
                    rmcp::ErrorData::internal_error(
                        format!("failed to build package `{}`: {}", key, msg),
                        None,
                    )
                })?;

        // Swap old watches for the new source file set.
        self.file_watcher.unwatch_package(&key);
        let source_files = data.env().get_source_file_names();
        let num_dirs = self
            .file_watcher
            .watch_package(&key, Path::new(&key), &source_files);
        log::info!("built package `{}`, watching {} dirs", key, num_dirs);

        // If any source file was modified during the build, skip caching.
        // The next call will see a cache miss and rebuild with fresh sources.
        let stale = source_files.iter().any(|f| {
            std::fs::metadata(f)
                .and_then(|m| m.modified())
                .is_ok_and(|mtime| mtime >= build_start)
        });

        let data = Arc::new(Mutex::new(data));
        if stale {
            log::info!("source changed during build of `{}`, skipping cache", key);
            return Ok((data, true));
        }

        let mut cache = self
            .package_cache
            .lock()
            .expect("package_cache lock poisoned");
        cache.insert(key, Arc::clone(&data));
        Ok((data, true))
    }
}

#[tool_handler]
impl ServerHandler for FlowSession {
    fn get_info(&self) -> ServerInfo {
        let mut instructions =
            "MCP server for Move smart contract development on Aptos.".to_string();
        if self.args.dev_mode {
            instructions.push_str(
                " Packages are compiled in dev mode \
                 (dev-addresses and dev-dependencies are active).",
            );
        }
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "move-flow".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                ..Default::default()
            },
            instructions: Some(instructions),
        }
    }
}

// --------- Helpers ---------------------------------------------------------------

/// Helper to convert any Serialize type into a CallToolResult with JSON text content
pub(crate) fn into_call_tool_result<T: Serialize>(value: &T) -> CallToolResult {
    let json = serde_json::to_string_pretty(value).expect("serde_json serialization failed");
    CallToolResult::success(vec![Content::text(json)])
}
