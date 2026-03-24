// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Move package testing and coverage tools.

use super::super::{
    common::{format_error_chain, mcp_err, mcp_err_chain, resolve_function, tool_error},
    package_data::PackageData,
    session::FlowSession,
    McpArgs,
};
use aptos_framework::extended_checks;
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_types::on_chain_config::{
    aptos_test_feature_flags_genesis, Features, TimedFeaturesBuilder,
};
use aptos_vm::natives;
use move_command_line_common::files::MOVE_COVERAGE_MAP_EXTENSION;
use move_coverage::{coverage_map::CoverageMap, source_coverage::SourceCoverageBuilder};
use move_model::model::GlobalEnv;
use move_package::{BuildConfig, CompilerConfig};
use move_package_cache::file_lock::FileLock;
use move_unit_test::{
    package_test::{run_move_unit_tests, UnitTestResult},
    test_validation, UnitTestingConfig,
};
use move_vm_runtime::native_functions::NativeFunctionTable;
use rmcp::{
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content},
    schemars, tool, tool_router,
};
use std::{
    collections::{hash_map::DefaultHasher, BTreeMap, BTreeSet},
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::Once,
    time::Duration,
};

// ========== MCP Tool types ==========

/// Parameters for `move_package_test` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct MovePackageTestParams {
    /// Path to the Move package directory.
    package_path: String,
    /// If true, save current coverage as baseline for future comparisons.
    /// Use this in the first test run before generating new tests.
    #[serde(default)]
    establish_baseline: bool,
}

/// Response for `move_package_test` tool.
#[derive(Debug, serde::Serialize)]
struct TestResponse {
    /// Whether all tests passed.
    success: bool,
    /// Whether baseline was established (only in baseline mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    baseline_established: Option<bool>,
    /// Lines that became covered since baseline (only in normal mode).
    /// Maps source file path to list of newly covered line numbers.
    #[serde(skip_serializing_if = "Option::is_none")]
    newly_covered: Option<BTreeMap<String, BTreeSet<u32>>>,
    /// Test output (only on failure, or hint when baseline is missing).
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<String>,
}

/// Parameters for `move_package_coverage` tool.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct MovePackageCoverageParams {
    /// Path to the Move package directory.
    package_path: String,
    /// Optional function to scope coverage to, in the format "module_name::function_name".
    /// If omitted, returns uncovered lines for all functions.
    function: Option<String>,
}

/// Response for `move_package_coverage` tool.
#[derive(Debug, serde::Serialize)]
struct CoverageResponse {
    /// Uncovered lines per source file.
    /// Maps source file path to list of uncovered line numbers.
    uncovered: BTreeMap<String, BTreeSet<u32>>,
}

// ========== MCP Tools ==========

#[tool_router(router = package_test_router, vis = "pub(crate)")]
impl FlowSession {
    /// Run tests, then either establish a baseline or report newly covered lines.
    #[tool(
        description = "Run Move unit tests for a package. Set establish_baseline=true to save \
                          current coverage as baseline (use before generating new tests). Without \
                          establish_baseline, returns lines newly covered since baseline. Returns \
                          newly_covered=null if no baseline exists — call with establish_baseline first.",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    async fn move_package_test(
        &self,
        Parameters(params): Parameters<MovePackageTestParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        Ok(self
            .move_package_test_impl(params)
            .await
            .unwrap_or_else(tool_error))
    }

    #[tool(
        description = "Get uncovered source lines for a package, optionally scoped to a function. \
                          Uses existing coverage map if available, otherwise runs tests first. \
                          Use this to identify which code paths need test coverage.",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    async fn move_package_coverage(
        &self,
        Parameters(params): Parameters<MovePackageCoverageParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        Ok(self
            .move_package_coverage_impl(params)
            .await
            .unwrap_or_else(tool_error))
    }
}

impl FlowSession {
    async fn move_package_test_impl(
        &self,
        params: MovePackageTestParams,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        log::info!(
            "move_package_test: path=`{}` establish_baseline={}",
            params.package_path,
            params.establish_baseline
        );

        let pkg_path = PathBuf::from(&params.package_path);
        if !pkg_path.exists() {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "package path does not exist: {}",
                params.package_path
            ))]));
        }

        // File lock serializes concurrent test runs on the same package.
        let _lock = self.coverage_lock(&pkg_path).await?;

        let (success, output) = self.run_tests_async(&pkg_path).await?;

        // Baseline mode: save current coverage as reference point for future comparisons.
        // Only save on success to ensure baseline reflects a valid test run.
        if params.establish_baseline {
            if success {
                save_baseline_coverage_map(self.temp_dir(), &pkg_path)
                    .map_err(|e| mcp_err_chain("failed to save baseline", &e))?;
            }
            log::info!(
                "move_package_test: tests {}, baseline {}",
                if success { "passed" } else { "failed" },
                if success { "saved" } else { "not saved" }
            );
            return test_response_to_result(TestResponse {
                success,
                baseline_established: Some(success),
                newly_covered: None,
                output: if success { None } else { Some(output) },
            });
        }

        // Normal mode: compare current coverage against baseline to find newly covered lines.
        // Skip coverage analysis on test failure (coverage map may reflect partial execution).
        let has_baseline = has_baseline(self.temp_dir(), &pkg_path);
        let newly_covered = if success && has_baseline {
            let (pkg_data, _) = self.resolve_package(&params.package_path).await?;
            let mut pkg = pkg_data
                .lock()
                .map_err(|_| mcp_err("package lock poisoned"))?;
            compute_newly_covered(self.temp_dir(), &pkg_path, &mut pkg)
                .map_err(|e| mcp_err_chain("failed to compute coverage", &e))?
        } else {
            None
        };

        let newly_covered_count: usize = newly_covered
            .as_ref()
            .map(|nc| nc.values().map(|lines| lines.len()).sum())
            .unwrap_or(0);
        log::info!(
            "move_package_test: tests {}, {} newly covered lines",
            if success { "passed" } else { "failed" },
            newly_covered_count
        );

        let output = if !success {
            Some(output)
        } else if !has_baseline {
            Some("No baseline established. Call with establish_baseline=true first.".to_string())
        } else {
            None
        };

        test_response_to_result(TestResponse {
            success,
            baseline_established: None,
            newly_covered,
            output,
        })
    }

    async fn move_package_coverage_impl(
        &self,
        params: MovePackageCoverageParams,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        log::info!(
            "move_package_coverage: path=`{}`{}",
            params.package_path,
            params
                .function
                .as_deref()
                .map_or(String::new(), |f| format!(", function=`{}`", f))
        );

        let pkg_path = PathBuf::from(&params.package_path);
        if !pkg_path.exists() {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "package path does not exist: {}",
                params.package_path
            ))]));
        }

        // File lock serializes concurrent test runs on the same package.
        let _lock = self.coverage_lock(&pkg_path).await?;

        // If source changed since last build, the coverage map on disk is stale.
        let (pkg_data, rebuilt) = self.resolve_package(&params.package_path).await?;
        if rebuilt {
            delete_coverage_map(&pkg_path);
        }

        // Run tests if coverage map is missing, or if no baseline exists
        // (coverage map without baseline is likely stale from a previous session).
        let need_baseline = !has_baseline(self.temp_dir(), &pkg_path);
        if need_baseline || !has_coverage_map(&pkg_path) {
            log::info!("move_package_coverage: running tests to generate fresh coverage");
            let (success, output) = self.run_tests_async(&pkg_path).await?;
            if !success {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "tests failed. Fix tests before checking coverage.\n\n{}",
                    output
                ))]));
            }
            if need_baseline {
                if let Err(e) = save_baseline_coverage_map(self.temp_dir(), &pkg_path) {
                    log::warn!("move_package_coverage: no baseline exists; attempt to create one failed: {}", e);
                }
            }
        }

        let mut pkg = pkg_data
            .lock()
            .map_err(|_| mcp_err("package lock poisoned"))?;
        let line_filter = make_function_line_filter(params.function.as_deref(), &mut pkg)?;
        let uncovered = compute_uncovered(&pkg_path, &mut pkg, line_filter.as_deref())
            .map_err(|e| mcp_err_chain("failed to compute coverage", &e))?;

        log::info!(
            "move_package_coverage: {} uncovered lines",
            uncovered.values().map(|l| l.len()).sum::<usize>()
        );

        let json = serde_json::to_string_pretty(&CoverageResponse { uncovered })
            .map_err(|e| mcp_err(format!("json serialization failed: {}", e)))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Acquire per-package file lock to serialize concurrent coverage operations.
    async fn coverage_lock(&self, pkg_path: &Path) -> Result<FileLock, rmcp::ErrorData> {
        FileLock::lock_with_alert_on_wait(
            pkg_path.join(".coverage_map.lock"),
            Duration::from_millis(1000),
            || log::info!("waiting for coverage map lock on `{}`", pkg_path.display()),
        )
        .await
        .map_err(|e| mcp_err(format!("failed to acquire coverage lock: {}", e)))
    }

    /// Run tests on a blocking thread.
    /// The caller is responsible for holding the per-package file lock
    /// for the duration of the call.
    async fn run_tests_async(&self, pkg_path: &Path) -> Result<(bool, String), rmcp::ErrorData> {
        let args = self.args().clone();
        let path = pkg_path.to_path_buf();
        let result = tokio::task::spawn_blocking(move || run_tests(&path, &args)).await;
        result
            // spawn_blocking task panicked or was cancelled (bug)
            .map_err(|e| mcp_err(format!("test task error: {}", e)))?
            // compilation or test runner infrastructure failure
            .map_err(|e| mcp_err(format!("test execution failed: {}", format_error_chain(&e))))
    }
}

// ========== Helpers ==========

/// Serialize a TestResponse into a tool result (success or error based on outcome).
fn test_response_to_result(response: TestResponse) -> Result<CallToolResult, rmcp::ErrorData> {
    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| mcp_err(format!("json serialization failed: {}", e)))?;
    Ok(if response.success {
        CallToolResult::success(vec![Content::text(json)])
    } else {
        CallToolResult::error(vec![Content::text(json)])
    })
}

// --------- Test runner ---------------------------------------------------------------

/// Gas limit for unit test execution. Prevents infinite loops from hanging tests.
const TEST_GAS_LIMIT: u64 = 100_000;

/// Guards one-time initialization of test configuration (validation hooks,
/// native function mode) so that concurrent calls are safe.
static TEST_INIT: Once = Once::new();

/// Run Move unit tests for a package with coverage enabled.
/// Returns (success, output) where output is the test output string.
fn run_tests(pkg_path: &Path, args: &McpArgs) -> anyhow::Result<(bool, String)> {
    log::info!("running tests for `{}`", pkg_path.display());

    let build_config = make_test_build_config(pkg_path, args);
    let unit_test_config = UnitTestingConfig::default();

    let mut output = Vec::new();
    let result = run_move_unit_tests(
        pkg_path,
        build_config,
        unit_test_config,
        aptos_test_natives(),
        aptos_test_feature_flags_genesis(),
        Some(TEST_GAS_LIMIT),
        None, // cost_table: use default
        true, // compute_coverage: needed for coverage tracking
        &mut output,
        true, // enable_enum_types
    );

    let output_str = String::from_utf8_lossy(&output).into_owned();
    match result {
        Ok(UnitTestResult::Success) => {
            log::info!("tests passed");
            Ok((true, output_str))
        },
        Ok(UnitTestResult::Failure) => {
            log::info!("tests failed");
            Ok((false, output_str))
        },
        Err(e) => {
            log::error!("test execution error: {}", e);
            log::error!(
                "captured output ({} bytes): {}",
                output_str.len(),
                output_str
            );
            Err(e.context(output_str))
        },
    }
}

/// Initialize test globals (idempotent) and return Aptos native functions.
fn aptos_test_natives() -> NativeFunctionTable {
    TEST_INIT.call_once(|| {
        natives::configure_for_unit_test();
        test_validation::set_validation_hook(Box::new(|env: &GlobalEnv| {
            extended_checks::run_extended_checks(env);
        }));
    });

    natives::aptos_natives(
        LATEST_GAS_FEATURE_VERSION,
        NativeGasParameters::zeros(),
        MiscGasParameters::zeros(),
        TimedFeaturesBuilder::enable_all().build(),
        Features::default(),
    )
}

/// Create a build config suitable for testing, using MCP args.
fn make_test_build_config(pkg_path: &Path, args: &McpArgs) -> BuildConfig {
    let compiler_config = CompilerConfig {
        known_attributes: extended_checks::get_all_attribute_names().clone(),
        bytecode_version: args.bytecode_version,
        language_version: args.language_version,
        experiments: args.experiments.clone(),
        ..Default::default()
    };

    let additional_named_addresses = args
        .named_addresses
        .iter()
        .map(|(name, addr)| (name.clone(), addr.into_inner()))
        .collect();

    BuildConfig {
        test_mode: true,
        dev_mode: args.dev_mode,
        install_dir: Some(pkg_path.join("build")),
        compiler_config,
        full_model_generation: true,
        additional_named_addresses,
        ..Default::default()
    }
}

// --------- Coverage helpers ----------------------------------------------------------

/// Path to the baseline coverage map file in the session temp directory.
/// Uses a hash of the canonicalized package path to avoid collisions.
fn baseline_coverage_map_path(base_dir: &Path, pkg_path: &Path) -> PathBuf {
    // Canonicalize to ensure ./pkg and /abs/pkg hash the same
    let canonical = pkg_path
        .canonicalize()
        .unwrap_or_else(|_| pkg_path.to_path_buf());

    let mut hasher = DefaultHasher::new();
    canonical.hash(&mut hasher);
    let hash = hasher.finish();

    base_dir.join(format!("baseline_coverage_{:x}.mvcov", hash))
}

/// Path to the coverage map file generated by the test runner.
fn coverage_map_path(pkg_path: &Path) -> PathBuf {
    pkg_path
        .join(".coverage_map")
        .with_extension(MOVE_COVERAGE_MAP_EXTENSION)
}

/// Save the current coverage map as baseline. Creates an empty baseline if no tests exist.
fn save_baseline_coverage_map(base_dir: &Path, pkg_path: &Path) -> anyhow::Result<()> {
    let src = coverage_map_path(pkg_path);
    let dst = baseline_coverage_map_path(base_dir, pkg_path);

    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }

    if src.exists() {
        std::fs::copy(&src, &dst)?;
        log::info!("saved baseline coverage to {}", dst.display());
    } else {
        // No tests exist yet - create empty baseline
        let empty = CoverageMap::default();
        move_coverage::coverage_map::output_map_to_file(&dst, &empty)?;
        log::info!("created empty baseline coverage at {}", dst.display());
    }
    Ok(())
}

fn has_coverage_map(pkg_path: &Path) -> bool {
    coverage_map_path(pkg_path).exists()
}

fn has_baseline(base_dir: &Path, pkg_path: &Path) -> bool {
    baseline_coverage_map_path(base_dir, pkg_path).exists()
}

/// Delete the coverage map (preserves baseline).
fn delete_coverage_map(pkg_path: &Path) {
    let _ = std::fs::remove_file(coverage_map_path(pkg_path));
}

/// Load coverage map from disk, falling back to an empty map when no file exists
/// (e.g. zero tests). The empty map causes every executable line to be reported
/// as uncovered.
fn load_coverage_map(path: &Path) -> anyhow::Result<CoverageMap> {
    if path.exists() {
        CoverageMap::from_binary_file(&path)
    } else {
        Ok(CoverageMap::default())
    }
}

/// Ensure bytecode is available and return the model environment.
fn ensure_env(pkg_data: &mut PackageData) -> anyhow::Result<&GlobalEnv> {
    if !pkg_data.has_bytecode() {
        pkg_data.rebuild_with_bytecode()?;
    }
    Ok(pkg_data.env())
}

/// Convert uncovered locations from a SourceCoverageBuilder to line numbers.
fn uncovered_lines(builder: &SourceCoverageBuilder, env: &GlobalEnv) -> BTreeSet<u32> {
    builder
        .uncovered_locations
        .iter()
        .filter_map(|loc| env.get_location(&env.to_loc(loc)))
        .map(|location| location.line.0 + 1)
        .collect()
}

/// Optional filter: given a source path and line number, returns whether the line
/// should be included in coverage results.
type LineFilter = dyn Fn(&str, u32) -> bool;

/// Compute uncovered lines from the current coverage map.
/// Returns a map from source file path to uncovered line numbers.
fn compute_uncovered(
    pkg_path: &Path,
    pkg_data: &mut PackageData,
    line_filter: Option<&LineFilter>,
) -> anyhow::Result<BTreeMap<String, BTreeSet<u32>>> {
    let env = ensure_env(pkg_data)?;
    let cov_map = load_coverage_map(&coverage_map_path(pkg_path))?;

    let mut result: BTreeMap<String, BTreeSet<u32>> = BTreeMap::new();

    for module in env.get_modules() {
        if !module.is_primary_target() {
            continue;
        }
        let (compiled_module, source_map) =
            match (module.get_verified_module(), module.get_source_map()) {
                (Some(cm), Some(sm)) => (cm, sm),
                _ => continue,
            };
        let source_path = module.get_source_path().to_string_lossy().to_string();
        let builder =
            SourceCoverageBuilder::new(&cov_map, source_map, vec![(compiled_module, source_map)]);
        let mut lines = uncovered_lines(&builder, env);

        if let Some(filter) = line_filter {
            lines.retain(|&line| filter(&source_path, line));
        }

        if !lines.is_empty() {
            result.entry(source_path).or_default().extend(lines);
        }
    }

    Ok(result)
}

/// Compare baseline and current coverage to find newly covered lines.
/// Returns lines that were uncovered in baseline but are now covered.
fn compute_newly_covered(
    base_dir: &Path,
    pkg_path: &Path,
    pkg_data: &mut PackageData,
) -> anyhow::Result<Option<BTreeMap<String, BTreeSet<u32>>>> {
    if !has_coverage_map(pkg_path) {
        return Ok(None);
    }
    let env = ensure_env(pkg_data)?;
    let current_cov = CoverageMap::from_binary_file(&coverage_map_path(pkg_path))?;
    let baseline_cov =
        CoverageMap::from_binary_file(&baseline_coverage_map_path(base_dir, pkg_path))?;

    let mut result: BTreeMap<String, BTreeSet<u32>> = BTreeMap::new();

    for module in env.get_modules() {
        if !module.is_primary_target() {
            continue;
        }
        let (compiled_module, source_map) =
            match (module.get_verified_module(), module.get_source_map()) {
                (Some(cm), Some(sm)) => (cm, sm),
                _ => continue,
            };
        let source_path = module.get_source_path().to_string_lossy().to_string();
        let module_info = vec![(compiled_module, source_map)];

        let baseline_builder =
            SourceCoverageBuilder::new(&baseline_cov, source_map, module_info.clone());
        let current_builder = SourceCoverageBuilder::new(&current_cov, source_map, module_info);

        // Newly covered = was uncovered before, but is covered now
        let baseline_uncovered = uncovered_lines(&baseline_builder, env);
        let current_uncovered = uncovered_lines(&current_builder, env);
        let newly_covered: BTreeSet<u32> = baseline_uncovered
            .difference(&current_uncovered)
            .copied()
            .collect();

        if !newly_covered.is_empty() {
            result.entry(source_path).or_default().extend(newly_covered);
        }
    }

    Ok(Some(result))
}

/// Build a line filter for a specific function's span.
fn make_function_line_filter(
    function: Option<&str>,
    pkg_data: &mut PackageData,
) -> Result<Option<Box<LineFilter>>, rmcp::ErrorData> {
    let Some(function) = function else {
        return Ok(None);
    };
    let env = ensure_env(pkg_data).map_err(|e| mcp_err_chain("failed to load env", &e))?;
    let func = resolve_function(env, function)?;
    let loc = func.get_loc();
    let start = env
        .get_location(&loc)
        .map(|l| l.line.0 + 1)
        .ok_or_else(|| mcp_err("cannot resolve function location"))?;
    let end = env
        .get_location(&loc.at_end())
        .map(|l| l.line.0 + 1)
        .ok_or_else(|| mcp_err("cannot resolve function location"))?;
    let source_path = func
        .module_env
        .get_source_path()
        .to_string_lossy()
        .to_string();
    Ok(Some(Box::new(move |path: &str, line: u32| {
        path == source_path && line >= start && line <= end
    })))
}
