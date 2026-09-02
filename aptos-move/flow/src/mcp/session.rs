// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{
    common::format_error_chain, file_watcher::FileWatcher, package_data::PackageData,
    telemetry::Telemetry, McpArgs,
};
use crate::{
    conditions::{ConditionDelta, ConditionStatus},
    evaluation::EvaluationConfig,
    GlobalOpts,
};
use rmcp::{
    handler::server::{router::tool::ToolRouter, tool::ToolCallContext},
    model::{
        CallToolRequestParams, CallToolResult, Content, Implementation, ListToolsResult,
        PaginatedRequestParams, ProtocolVersion, ServerCapabilities, ServerInfo, Tool,
    },
    service::RequestContext,
    RoleServer, ServerHandler,
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
    evaluation: EvaluationConfig,
    telemetry: Telemetry,
    /// Cache of compiled packages. `Mutex<PackageData>` is needed because `GlobalEnv`
    /// is `!Sync` (it uses `RefCell` internally).
    package_cache: Arc<Mutex<BTreeMap<String, Arc<Mutex<PackageData>>>>>,
    file_watcher: FileWatcher,
    tool_router: ToolRouter<Self>,
    /// Session-scoped temp directory, automatically deleted on drop.
    temp_dir: Arc<tempfile::TempDir>,
    /// Condition statuses from the previous candidate check of each package.
    ///
    /// A progress delta compares two attempts, so one of them has to be
    /// remembered. Keyed by package; a session checking several keeps a
    /// history per package rather than interleaving them.
    previous_conditions: Arc<Mutex<BTreeMap<String, Vec<ConditionStatus>>>>,
}

impl FlowSession {
    /// Combined router for all registered MCP tools.
    ///
    /// Add new tool routers here — this is the single source of truth used by
    /// both `new()` and `tool_names()`.
    fn all_tool_routers(evaluation: EvaluationConfig) -> ToolRouter<Self> {
        let mut router = Self::package_manifest_router()
            + Self::package_query_router()
            + Self::package_status_router()
            + Self::package_test_router()
            + Self::package_verify_router();
        if evaluation.replay_tool_enabled() {
            router += Self::replay_transaction_router();
        }
        if evaluation.wp_tool_enabled() {
            router += Self::package_spec_infer_router();
        }
        router += Self::spec_check_router();
        router
    }

    /// Returns the names of all registered MCP tools.
    /// Used by the plugin renderer to validate tool references in templates.
    pub(crate) fn tool_names(evaluation: EvaluationConfig) -> Vec<String> {
        Self::all_tool_routers(evaluation)
            .list_all()
            .into_iter()
            .map(|t| t.name.to_string())
            .collect()
    }

    /// Returns (name, description) pairs for all registered MCP tools.
    /// Used by the plugin renderer to generate the README.
    pub(crate) fn tool_descriptions(evaluation: EvaluationConfig) -> Vec<(String, String)> {
        Self::all_tool_routers(evaluation)
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

    pub(crate) fn evaluation(&self) -> EvaluationConfig {
        self.evaluation
    }

    pub(crate) fn telemetry(&self) -> &Telemetry {
        &self.telemetry
    }

    /// Configured tool timeout as a `Duration`.
    pub(crate) fn tool_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.args.tool_timeout)
    }

    /// In an evaluation session, refuse a package whose manifest names a
    /// dependency that building it would fetch over the network.
    ///
    /// Every tool that builds a package calls this before building, so the
    /// refusal does not depend on which tool the agent reaches for first:
    /// `resolve_package` for the cached model, and the candidate check, which
    /// builds its own. The verify tool's loop-invariant evidence also rebuilds
    /// from disk, but only after `resolve_package` succeeded for the package in
    /// the same call, and a manifest edit invalidates the cache.
    pub(crate) fn refuse_remote_dependencies(&self, package: &Path) -> Result<(), rmcp::ErrorData> {
        if !self.evaluation().evaluation_mode {
            return Ok(());
        }
        crate::experiment::reject_remote_dependencies(package)
            .map_err(|message| rmcp::ErrorData::invalid_params(message, None))
    }

    pub(crate) fn temp_dir(&self) -> &Path {
        self.temp_dir.path()
    }

    pub(crate) fn new(
        args: McpArgs,
        global: GlobalOpts,
        evaluation: EvaluationConfig,
        telemetry: Telemetry,
    ) -> Self {
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
            evaluation,
            telemetry,
            package_cache,
            file_watcher,
            tool_router: Self::all_tool_routers(evaluation),
            temp_dir,
            previous_conditions: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Progress since the previous candidate check of this package.
    ///
    /// Returns nothing for the first check: there is no earlier attempt to
    /// compare against, and a delta against an empty report would read as if
    /// every obligation had just been added.
    pub(crate) fn condition_progress(
        &self,
        package: &str,
        conditions: &[ConditionStatus],
    ) -> Option<String> {
        let mut history = self
            .previous_conditions
            .lock()
            .expect("previous_conditions lock poisoned");
        let previous = history.insert(package.to_string(), conditions.to_vec());
        previous.map(|previous| ConditionDelta::between(&previous, conditions).render())
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

    /// Record the outcome of one `resolve_package` call.
    ///
    /// Every exit from `resolve_package` reports the same shape, so the JSONL
    /// consumers can parse the four outcomes uniformly.
    fn emit_package_resolve(
        &self,
        key: &str,
        cache_hit: bool,
        started: std::time::Instant,
        outcome: &str,
        extra: Option<(&str, serde_json::Value)>,
    ) {
        let mut fields = serde_json::json!({
            "package_id": key,
            "cache_hit": cache_hit,
            "duration_us": started.elapsed().as_micros() as u64,
            "outcome": outcome,
        });
        if let Some((name, value)) = extra {
            fields[name] = value;
        }
        self.telemetry.emit("package_resolve", fields);
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
        let resolve_start = std::time::Instant::now();
        let key = self.resolve_package_path(package_path);
        // Before the cache can answer: a cached package whose manifest has
        // since gained a remote dependency would otherwise be served, and a
        // later filtered rebuild reads that manifest from disk. Watcher
        // invalidation is asynchronous, so it cannot be relied on here.
        if let Err(error) = self.refuse_remote_dependencies(Path::new(&key)) {
            self.emit_package_resolve(&key, false, resolve_start, "error", None);
            return Err(error);
        }
        {
            let cache = self
                .package_cache
                .lock()
                .expect("package_cache lock poisoned");
            if let Some(data) = cache.get(&key) {
                log::info!("cache hit for `{}`", key);
                self.emit_package_resolve(&key, true, resolve_start, "success", None);
                return Ok((Arc::clone(data), false));
            }
        }

        // Cache miss — rebuild. Keep existing watches active during compilation
        // so that edits are not missed; the watcher callback will call
        // `cache.remove(key)` which is a no-op while there is no cache entry.
        let build_start = std::time::SystemTime::now();

        log::info!("building package `{}`", key);
        // A measured session has no network egress channel by design: `Bash`
        // and the web tools are denied and the replay tool is not served.
        // Resolving a `git` or on-chain dependency would fetch it, and the
        // package under evaluation vendors everything it needs, so a remote
        // dependency here is refused rather than fetched.
        let args = self.args.clone();
        let key_clone = key.clone();
        let data =
            tokio::task::spawn_blocking(move || PackageData::init(key_clone.as_ref(), &args))
                .await
                .map_err(|e| {
                    self.emit_package_resolve(&key, false, resolve_start, "panic", None);
                    rmcp::ErrorData::internal_error(format!("build task failed: {}", e), None)
                })?
                .map_err(|e| {
                    let msg = format_error_chain(&e);
                    log::info!("build failed for `{}`: {}", key, msg);
                    self.emit_package_resolve(&key, false, resolve_start, "error", None);
                    rmcp::ErrorData::invalid_params(
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
        self.emit_package_resolve(
            &key,
            false,
            resolve_start,
            "success",
            Some(("watched_directories", num_dirs.into())),
        );

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

impl ServerHandler for FlowSession {
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let start = std::time::Instant::now();
        let tool_name = request.name.to_string();
        // Every field below costs something to produce -- `resolve_package_path`
        // canonicalizes, and the response is serialized in full just to be
        // measured. None of it is worth paying for on an unrecorded server.
        let recording = self.telemetry.is_enabled();
        let package_id = recording
            .then(|| {
                request
                    .arguments
                    .as_ref()
                    .and_then(|args| args.get("package_path"))
                    .and_then(|value| value.as_str())
                    .map(|path| self.resolve_package_path(path))
            })
            .flatten();
        let filter = recording
            .then(|| {
                request
                    .arguments
                    .as_ref()
                    .and_then(|args| args.get("filter"))
                    .cloned()
            })
            .flatten();
        let call_id = if recording {
            serde_json::to_value(&context.id).unwrap_or(serde_json::Value::Null)
        } else {
            serde_json::Value::Null
        };
        self.telemetry.emit(
            "tool_start",
            serde_json::json!({
                "call_id": &call_id,
                "tool_name": &tool_name,
                "package_id": &package_id,
                "filter": &filter,
            }),
        );

        let call_context = ToolCallContext::new(self, request, context);
        let result = self.tool_router.call(call_context).await;
        let (outcome, response_bytes) = match &result {
            Ok(response) => (
                if response.is_error == Some(true) {
                    "tool_error"
                } else {
                    "success"
                },
                if recording {
                    serde_json::to_vec(response).map_or(0, |bytes| bytes.len())
                } else {
                    0
                },
            ),
            Err(error) => (
                if error.to_string().contains("timeout") {
                    "timeout"
                } else {
                    "rpc_error"
                },
                if recording {
                    serde_json::to_vec(error).map_or(0, |bytes| bytes.len())
                } else {
                    0
                },
            ),
        };
        self.telemetry.emit(
            "tool_end",
            serde_json::json!({
                "call_id": &call_id,
                "tool_name": &tool_name,
                "package_id": &package_id,
                "filter": &filter,
                "duration_us": start.elapsed().as_micros() as u64,
                "outcome": outcome,
                "response_bytes": response_bytes,
            }),
        );
        result
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, rmcp::ErrorData> {
        Ok(ListToolsResult {
            tools: self.tool_router.list_all(),
            meta: None,
            next_cursor: None,
        })
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.tool_router.get(name).cloned()
    }

    fn get_info(&self) -> ServerInfo {
        let mut instructions =
            "MCP server for Move smart contract development on Aptos.".to_string();
        instructions.push_str(&format!(
            " Inference tactic: {}. Evaluation mode: {}.",
            self.evaluation.inference_tactic, self.evaluation.evaluation_mode
        ));
        if self.args.dev_mode {
            instructions.push_str(
                " Packages are compiled in dev mode \
                 (dev-addresses and dev-dependencies are active).",
            );
        }
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
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
