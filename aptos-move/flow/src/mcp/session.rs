// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{file_watcher::FileWatcher, package_data::PackageData, McpArgs};
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
}

impl FlowSession {
    /// Returns the names of all registered MCP tools.
    /// Used by the plugin renderer to validate tool references in templates.
    pub(crate) fn tool_names() -> Vec<String> {
        let router = Self::package_manifest_router()
            + Self::package_status_router()
            + Self::package_verify_router();
        router
            .list_all()
            .into_iter()
            .map(|t| t.name.to_string())
            .collect()
    }

    pub(crate) fn new(args: McpArgs, global: GlobalOpts) -> Self {
        let package_cache = Arc::new(Mutex::new(BTreeMap::new()));
        let cache_ref = Arc::clone(&package_cache);
        let file_watcher = FileWatcher::new(Arc::new(move |key: &str| {
            log::info!("invalidating cache for `{}`", key);
            cache_ref.lock().unwrap().remove(key);
        }))
        .expect("failed to create file watcher");
        Self {
            global,
            args,
            package_cache,
            file_watcher,
            tool_router: Self::package_manifest_router()
                + Self::package_status_router()
                + Self::package_verify_router(),
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

    /// Resolve a package, returning cached `PackageData` or building it on cache miss.
    ///
    /// Building is offloaded to `spawn_blocking` since compilation is a heavy
    /// synchronous operation that must not block the async executor.
    ///
    /// Cache entries are removed directly by the file-watcher callback,
    /// so a cache miss here means either first access or an invalidation.
    pub(crate) async fn resolve_package(
        &self,
        package_path: &str,
    ) -> Result<Arc<Mutex<PackageData>>, rmcp::ErrorData> {
        let key = self.resolve_package_path(package_path);
        {
            let cache = self.package_cache.lock().unwrap();
            if let Some(data) = cache.get(&key) {
                log::info!("cache hit for `{}`", key);
                return Ok(Arc::clone(data));
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
                    log::info!("build failed for `{}`: {}", key, e);
                    rmcp::ErrorData::internal_error(
                        format!("failed to build package `{}`: {}", key, e),
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
            return Ok(data);
        }

        let mut cache = self.package_cache.lock().unwrap();
        cache.insert(key, Arc::clone(&data));
        Ok(data)
    }
}

#[tool_handler]
impl ServerHandler for FlowSession {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "move-flow".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                ..Default::default()
            },
            instructions: Some("MCP server for Move smart contract development on Aptos.".into()),
        }
    }
}

// --------- Helpers ---------------------------------------------------------------

/// Helper to convert any Serialize type into a CallToolResult with JSON text content
pub(crate) fn into_call_tool_result<T: Serialize>(value: &T) -> CallToolResult {
    let json = serde_json::to_string_pretty(value).unwrap();
    CallToolResult::success(vec![Content::text(json)])
}
