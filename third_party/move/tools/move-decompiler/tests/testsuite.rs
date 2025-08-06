// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Test framework for the Move decompiler, allowing different compilation, decompilation, and testing configurations.

use codespan_reporting::{diagnostic::Severity, term::termcolor::Buffer};
use libtest_mimic::{Arguments, Trial};
use move_compiler_v2::{logging, run_move_compiler_for_analysis, Experiment};
use move_decompiler::Decompiler;
use move_model::metadata::LanguageVersion;
use move_prover_test_utils::baseline_test;
use once_cell::unsync::Lazy;
use std::{
    collections::BTreeMap,
    fs::{canonicalize, read_dir, File},
    io::Write,
    path::{Path, PathBuf},
};
use tempfile::tempdir;
use walkdir::WalkDir;

/// Extension for expected output files
pub const EXP_EXT: &str = "exp";

/// Which level of decompilation testing to run.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Default)]
enum TestLevel {
    // Test if decompilation works
    #[default]
    Decompile,
    // Test if decompiled code can be recompiled
    Recompile,
    // TODO: add a Test level for running transactional tests on the recompiled code
}

/// Configuration for a set of tests.
#[derive(Clone)]
struct TestConfig {
    /// Name of this configuration
    name: &'static str,
    /// A static function pointer to the runner to be used for datatest. Since datatest
    /// does not support closures but only function pointers which can't capture
    /// context variables, we need to inject the config here. This MUST be set
    /// to `runner: |p| run_test(p, get_config_by_name("<name-of-this-config>"))`.
    /// See existing configurations before. NOTE: a common error is to use the
    /// wrong config name via copy & paste here, so watch out.
    runner: fn(&Path) -> anyhow::Result<()>,
    /// Path to sources of test cases.
    sources: &'static str,
    /// Path substring for tests to exclude.
    exclude: Vec<&'static str>,
    /// List of dependencies to be compiled.
    sources_deps: Vec<&'static str>,
    /// List of locations to look up already-compiled dependencies.
    dependencies: Vec<&'static str>,
    /// Options for compiling the test case
    compiler_options: move_compiler_v2::Options,
    /// Options for the decompiler
    decompiler_options: move_decompiler::Options,
    /// If set, a suffix for the baseline file used for these tests.
    /// If None, uses `exp`
    exp_suffix: Option<&'static str>,
    /// Which level of testing to run
    test_level: TestLevel,
}

impl TestConfig {
    fn new(
        compiler_opts: move_compiler_v2::Options,
        decompiler_opts: move_decompiler::Options,
    ) -> Self {
        Self {
            name: "",
            runner: |_| anyhow::bail!("no test runner"),
            sources: Default::default(),
            sources_deps: vec![],
            dependencies: vec![],
            compiler_options: compiler_opts,
            decompiler_options: decompiler_opts,
            exp_suffix: None,
            exclude: vec![],
            test_level: Default::default(),
        }
    }

    fn lang(self, v: LanguageVersion) -> Self {
        Self {
            compiler_options: self.compiler_options.clone().set_language_version(v),
            ..self
        }
    }

    /// Disable control flow optimizations
    fn no_cfg_opt(self, no_conditionals: bool) -> Self {
        Self {
            decompiler_options: self
                .decompiler_options
                .clone()
                .disable_conditional_transformation(no_conditionals),
            ..self
        }
    }

    /// Disable assignment optimizations
    fn no_assign_opt(self, no_expressions: bool) -> Self {
        Self {
            decompiler_options: self
                .decompiler_options
                .clone()
                .disable_assignment_transformation(no_expressions),
            ..self
        }
    }
}

/// Decompilation test configurations. A test configuration is selected by
/// matching the exclude path specifications with the test file's path.
/// One test file can be selected for multiple configurations, in which case multiple
/// tests are generated for it. Each test file must have at least one matching
/// configuration.
#[allow(clippy::declare_interior_mutable_const)]
const TEST_CONFIGS: Lazy<BTreeMap<&str, TestConfig>> = Lazy::new(|| {
    // Create default compilation options
    let mut compiler_opts = move_compiler_v2::Options::default()
        // Turn optimization on by default. Some configs below may turn it off.
        .set_experiment(Experiment::OPTIMIZE, true)
        .set_experiment(Experiment::OPTIMIZE_WAITING_FOR_COMPARE_TESTS, true);

    compiler_opts.testing = true;
    compiler_opts.named_address_mapping = vec![
        "std=0x1".to_string(),
        "aptos_std=0x1".to_string(),
        "aptos_framework=0x1".to_string(),
        "aptos_fungible_asset=0xA".to_string(),
        "aptos_token=0x3".to_string(),
        "core_resources=0xA550C18".to_string(),
        "vm_reserved=0x0".to_string(),
        "vm=0x0".to_string(),
        // Add more named addresses as needed
    ];

    // Create default decompilation options
    let decompiler_options = move_decompiler::Options::default();

    let config = || TestConfig::new(compiler_opts.clone(), decompiler_options.clone());
    let configs = vec![
        // --- Tests for compilation and decompilation of Move code ---
        TestConfig {
            name: "legacy-move-stdlib",
            runner: |p| run_test(p, get_config_by_name("legacy-move-stdlib")),
            sources: "./tests/legacy-move-stdlib",
            sources_deps: vec![],
            dependencies: vec!["./tests/legacy-move-stdlib"],
            exclude: vec![],
            test_level: TestLevel::Recompile,
            ..config()
                .lang(LanguageVersion::latest())
                .no_cfg_opt(false)
                .no_assign_opt(false)
        },
        TestConfig {
            name: "control-flow-recovery",
            runner: |p| run_test(p, get_config_by_name("control-flow-recovery")),
            sources: "./tests/control-flow-recovery",
            sources_deps: vec![],
            dependencies: vec!["./tests/legacy-move-stdlib"],
            exclude: vec![],
            test_level: TestLevel::Recompile,
            ..config()
                .lang(LanguageVersion::latest())
                .no_cfg_opt(false)
                .no_assign_opt(false)
        },
        TestConfig {
            name: "move-v2-features",
            runner: |p| run_test(p, get_config_by_name("move-v2-features")),
            sources: "./tests/move-v2-features",
            sources_deps: vec![],
            dependencies: vec!["./tests/legacy-move-stdlib"],
            exclude: vec![],
            test_level: TestLevel::Recompile,
            ..config()
                .lang(LanguageVersion::latest())
                .no_cfg_opt(false)
                .no_assign_opt(false)
        },
    ];
    configs.into_iter().map(|c| (c.name, c)).collect()
});

/// A function which gets a copy of a TestConfig by name.
#[allow(clippy::borrow_interior_mutable_const)]
fn get_config_by_name(name: &str) -> TestConfig {
    TEST_CONFIGS
        .get(name)
        .cloned()
        .unwrap_or_else(|| panic!("wrongly named test config `{}`", name))
}

/// Runs test at `path` with the given `config`.
fn run_test(path: &Path, config: TestConfig) -> anyhow::Result<()> {
    logging::setup_logging_for_testing(None);
    let test_output = run_compile_decompile_test_workflow(path, &config)?;
    generate_or_check_baseline(path, config, test_output)
}

fn run_compile_decompile_test_workflow(path: &Path, config: &TestConfig) -> anyhow::Result<String> {
    let mut output = String::new();

    // Step 1: compile the test case
    let path_str = path.display().to_string();

    // Update the compiler options based on the specific test case
    let mut compiler_options = config.compiler_options.clone();
    compiler_options.sources = vec![path_str.clone()];
    compiler_options.sources_deps = config
        .sources_deps
        .iter()
        .map(|s| path_from_crate_root(s))
        .collect();
    compiler_options.dependencies = config
        .dependencies
        .iter()
        .map(|s| path_from_crate_root(s))
        .collect();

    let mut error_writer = Buffer::no_color();
    let env = match run_move_compiler_for_analysis(&mut error_writer, compiler_options) {
        Ok(env) => env,
        Err(e) => {
            // Early exit if compilation fails
            output.push_str(&format!(
                "--- aborting with compilation errors:\n{:#}\n{}\n",
                e,
                String::from_utf8_lossy(&error_writer.into_inner())
            ));
            return Ok(output);
        },
    };

    // Step 2: decompile the compiled modules
    let mut decompiler = Decompiler::new(config.decompiler_options.clone());
    for module_env in env.get_modules() {
        if !module_env.is_primary_target() {
            continue;
        }
        if let Some(compiled_module) = module_env.get_verified_module() {
            let source_map = module_env.get_source_map().cloned().unwrap_or_else(|| {
                let mut bytes = vec![];
                compiled_module
                    .serialize(&mut bytes)
                    .expect("expected serialization success");
                decompiler.empty_source_map(&module_env.get_full_name_str(), &bytes)
            });
            output += &decompiler.decompile_module(compiled_module.clone(), source_map);
            output += "\n";
        }
    }
    if decompiler
        .env()
        .check_diag(&mut error_writer, Severity::Warning, "decompilation")
        .is_err()
    {
        // Early exit if decompilation fails
        output.push_str(&format!(
            "--- decompilation errors:\n{}\n",
            String::from_utf8_lossy(&error_writer.into_inner())
        ));
        return Ok(output);
    }

    if config.test_level == TestLevel::Decompile {
        // If we only want to test decompilation, return the output here.
        return Ok(output);
    }

    // Step 3: Recompile the decompiled code
    error_writer.clear();
    match recompile_decompiled_code(path, config, &output, &mut error_writer) {
        Ok(res) => output.push_str(&res),
        Err(e) => {
            output.push_str(&format!(
                "--- unable to recompile the decompiled code:\n{:#}\n{}\n",
                e,
                String::from_utf8_lossy(&error_writer.into_inner())
            ));
            return Ok(output);
        },
    }
    Ok(output)
}

/// Recompiles the decompiled code at `path` using the given `config`.
fn recompile_decompiled_code(
    path: &Path,
    config: &TestConfig,
    decompiled_code: &str,
    error_writer: &mut Buffer,
) -> anyhow::Result<String> {
    // Put the decompiled code into a temporary file with the same name as the original
    let temp_dir = tempdir()?;
    let temp_file_path = temp_dir
        .path()
        .join(path.file_name().expect("test case must have a file name"));
    let mut temp_file = File::create(&temp_file_path)?;
    temp_file.write_all(decompiled_code.as_bytes())?;

    // Reconfigure the compiler options to use the temporary file
    //   and exclude the original file from the dependencies.
    let mut compiler_options = config.compiler_options.clone();
    compiler_options.sources = vec![temp_file_path.to_string_lossy().to_string()];
    compiler_options.sources_deps = config
        .sources_deps
        .iter()
        .flat_map(|s| exclude_target_file(path, path_from_crate_root(s)))
        .collect();
    compiler_options.dependencies = config
        .dependencies
        .iter()
        .flat_map(|s| exclude_target_file(path, path_from_crate_root(s)))
        .collect();

    run_move_compiler_for_analysis(error_writer, compiler_options)?;
    Ok("\n============ recompilation succeeded ========\n".to_string())
}

/// Get a list of dependencies that exclude the target file itself
/// - If the dependency path is a file, return it
/// - If the dependency is a dir but does not contain the target file, return it
/// - Else, return all the contained `.move` files except for the target file
fn exclude_target_file(file: &Path, dir: String) -> Vec<String> {
    // get the canonical, absolute paths, avoiding issues with relative paths
    let canonical_file = canonicalize(file).expect("test case must have a valid path");
    let dir_path = Path::new(&dir);
    let canonical_dir = canonicalize(dir_path).expect("dependency must have a valid path");

    if canonical_file == canonical_dir {
        // If the dependency is the same as the file, return empty
        return vec![];
    }

    if canonical_dir.is_file()
        || canonical_file
            .parent()
            .expect("canonical file path must have a parent")
            != canonical_dir
    {
        // If the dependency is a file or not in the same directory as the target file,
        // return it as is.
        return vec![dir];
    }

    read_dir(dir)
        .expect("dependency must be a file or a directory")
        .filter_map(|entry| {
            let entry = entry.expect("dependency must be a valid file system entry");
            let path = canonicalize(entry.path()).expect("a valid path must be canonicalizable");
            if path.is_file() && path != canonical_file && path.extension() == Some("move".as_ref())
            {
                Some(path.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect::<Vec<String>>()
}

fn generate_or_check_baseline(
    path: &Path,
    config: TestConfig,
    test_output: String,
) -> anyhow::Result<()> {
    let exp_file_ext = config.exp_suffix.unwrap_or("exp");
    let baseline_path = path.with_extension(exp_file_ext);
    baseline_test::verify_or_update_baseline(baseline_path.as_path(), &test_output)?;
    Ok(())
}

/// Returns a path relative to the crate root.
fn path_from_crate_root(path: &str) -> String {
    let mut buf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    buf.push(path);
    buf.to_string_lossy().to_string()
}

/// Collects tests found under the given root and adds them to a list,
/// where each entry represents one `(path, config)`
/// combination. This errors if not configuration is found for a given test path.
/// TODO: we may want to add some filters here based on env vars (like the prover does),
///    which allow e.g. to filter by configuration name.
#[allow(clippy::borrow_interior_mutable_const)]
fn collect_tests() -> Vec<Trial> {
    let mut test_groups: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();

    for config in TEST_CONFIGS.values() {
        let mut found_one = false;
        // For non-package tests, we walk the sources directory to collect all .move files
        for entry in WalkDir::new(config.sources)
            .follow_links(false)
            .min_depth(1)
            .into_iter()
            .flatten()
        {
            let entry_str = entry.path().to_string_lossy().to_string();
            if !entry_str.ends_with(".move") {
                continue;
            }
            if config.exclude.iter().any(|s| entry_str.contains(s)) {
                // no match
                continue;
            }
            test_groups
                .entry(config.name)
                .or_default()
                .push(entry_str.clone());
            found_one = true;
        }
        assert!(
            found_one,
            "cannot find available test cases for `{}`",
            config.name
        )
    }

    let mut tests = vec![];
    for (name, src_target) in test_groups {
        let config = &get_config_by_name(name);
        for file in src_target {
            let test_prompt = format!("decompiler [config={}]::move-file::{}", config.name, file);
            let test_path = PathBuf::from(file);
            let runner = config.runner;
            tests.push(Trial::test(test_prompt, move || {
                runner(&test_path).map_err(|err| format!("{:?}", err).into())
            }));
        }
    }
    tests
}

fn main() {
    let args = Arguments::from_args();
    let mut tests = collect_tests();
    tests.sort_unstable_by(|a, b| a.name().cmp(b.name()));
    libtest_mimic::run(&args, tests).exit()
}
