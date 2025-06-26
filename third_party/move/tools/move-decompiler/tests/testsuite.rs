// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use codespan_reporting::{diagnostic::Severity, term::termcolor::Buffer};
use libtest_mimic::{Arguments, Trial};
use move_compiler_v2::{logging, run_move_compiler_for_analysis, Experiment, Options};
use move_model::{metadata::LanguageVersion, model::GlobalEnv};
use move_stackless_bytecode::function_target_pipeline::FunctionTargetsHolder;
use once_cell::unsync::Lazy;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

/// Which level of testing to run.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Default)]
enum TestLevel {
    // Test if decompilation works
    Decompile,
    // Test if decompiled code can be recompiled
    Recompile,
    // Test if recompiled code can pass available tests
    #[default]
    Rerun,
}

/// When to stop the decompilation process.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Default)]
enum StopAfter {
    // stop after file format bytecode is lifted to stackless bytecode
    StackLessBytecodeGen,
    // stop after stackless bytecode is lifted to ast
    AstGen,
    // stop after ast is sourcified
    #[default]
    SourcifierRun,
}

/// Configuration for a set of tests.
#[derive(Clone)]
struct TestConfig {
    /// Name of this configuration
    name: &'static str,
    /// If the decompilation is a package
    is_package: bool,
    /// A static function pointer to the runner to be used for datatest. Since datatest
    /// does not support closures but only function pointers which can't capture
    /// context variables, we need to inject the config here. This MUST be set
    /// to `runner: |p| run_test(p, get_config_by_name("<name-of-this-config>"))`.
    /// See existing configurations before. NOTE: a common error is to use the
    /// wrong config name via copy & paste here, so watch out.
    runner: fn(&Path) -> anyhow::Result<()>,
    /// Path to sources of test cases.
    sources: &'static str,
    /// List of dependencies to be compiled.
    sources_deps: Vec<&'static str>,
    /// List of locations to look up already-compiled dependencies.
    dependencies: Vec<&'static str>,
    /// Path substring for tests to exclude.
    exclude: Vec<&'static str>,
    /// The options used for this run
    options: Options,
    /// After which step to stop decompilation
    stop_after: StopAfter,
    /// Which level of testing to run
    test_level: TestLevel,
}

impl TestConfig {
    fn new(opts: Options) -> Self {
        Self {
            name: "",
            is_package: false,
            runner: |_| anyhow::bail!("no test runner"),
            sources: Default::default(),
            sources_deps: vec![],
            dependencies: vec![],
            exclude: vec![],
            options: opts,
            stop_after: Default::default(),
            test_level: Default::default(),
        }
    }

    fn lang(self, v: LanguageVersion) -> Self {
        Self {
            options: self.options.clone().set_language_version(v),
            ..self
        }
    }
}

/// Decompilation test configurations. A test configuration is selected by
/// matching the include/exclude path specifications with the test file's path.
/// One test file can be selected for multiple configurations, in which case multiple
/// tests are generated for it. Each test file must have at least one matching
/// configuration.
#[allow(clippy::declare_interior_mutable_const)]
const TEST_CONFIGS: Lazy<BTreeMap<&str, TestConfig>> = Lazy::new(|| {
    // Create default options
    let mut opts = Options::default()
        // Spec rewriter is always on, so we test it, even though it's not part of regular compiler.
        .set_experiment(Experiment::SPEC_REWRITE, true)
        // Turn optimization on by default. Some configs below may turn it off.
        .set_experiment(Experiment::OPTIMIZE, true)
        .set_experiment(Experiment::OPTIMIZE_WAITING_FOR_COMPARE_TESTS, true)
        .set_language_version(LanguageVersion::latest());
    opts.testing = true;

    let config = || TestConfig::new(opts.clone());
    let configs = vec![
        // --- Tests for checking and ast processing
        // Tests for model building and various post-processing checking
        TestConfig {
            name: "compiler-v2-tests",
            runner: |p| run_test(p, get_config_by_name("compiler-v2-tests")),
            is_package: false,
            sources: "../../move-compiler-v2/tests",
            sources_deps: vec![],
            dependencies: vec!["../../move-stdlib/sources"],
            // Need to exclude `inlining` because it is under checking
            // TODO: move `inlining` tests to top-level test directory
            exclude: vec![],
            stop_after: StopAfter::SourcifierRun,
            test_level: TestLevel::Decompile,
            ..config().lang(LanguageVersion::latest())
        },
        TestConfig {
            name: "compiler-v2-transactional-tests",
            runner: |p| run_test(p, get_config_by_name("compiler-v2-transactional-tests")),
            is_package: false,
            sources: "../../move-compiler-v2/transactional-tests",
            sources_deps: vec![],
            dependencies: vec!["../../move-stdlib/sources"],
            // Need to exclude `inlining` because it is under checking
            // TODO: move `inlining` tests to top-level test directory
            exclude: vec![],
            stop_after: StopAfter::SourcifierRun,
            test_level: TestLevel::Decompile,
            ..config().lang(LanguageVersion::latest())
        },
        TestConfig {
            name: "move-stdlib-tests",
            runner: |p| run_test(p, get_config_by_name("move-stdlib-tests")),
            is_package: true,
            sources: "../../../../aptos-move/framework/move-stdlib/sources",
            sources_deps: vec![],
            dependencies: vec![],
            // Need to exclude `inlining` because it is under checking
            // TODO: move `inlining` tests to top-level test directory
            exclude: vec![],
            stop_after: StopAfter::SourcifierRun,
            test_level: TestLevel::Decompile,
            ..config().lang(LanguageVersion::latest())
        },
        TestConfig {
            name: "aptos-stdlib-tests",
            runner: |p| run_test(p, get_config_by_name("aptos-stdlib-tests")),
            is_package: true,
            sources: "../../../../aptos-move/framework/aptos-stdlib/sources",
            sources_deps: vec![],
            dependencies: vec!["../../../../aptos-move/framework/move-stdlib/sources"],
            // Need to exclude `inlining` because it is under checking
            // TODO: move `inlining` tests to top-level test directory
            exclude: vec![],
            stop_after: StopAfter::SourcifierRun,
            test_level: TestLevel::Decompile,
            ..config().lang(LanguageVersion::latest())
        },
        TestConfig {
            name: "aptos-framework-tests",
            runner: |p| run_test(p, get_config_by_name("aptos-framework-tests")),
            is_package: true,
            sources: "../../../../aptos-move/framework/aptos-framework/sources",
            sources_deps: vec![],
            dependencies: vec![
                "../../../../aptos-move/framework/move-stdlib/sources",
                "../../../../aptos-move/framework/aptos-stdlib/sources",
            ],
            // Need to exclude `inlining` because it is under checking
            // TODO: move `inlining` tests to top-level test directory
            exclude: vec![],
            stop_after: StopAfter::SourcifierRun,
            test_level: TestLevel::Decompile,
            ..config().lang(LanguageVersion::latest())
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
    logging::setup_logging_for_testing();
    let path_str = path.display().to_string();
    let mut compiler_options = config.options.clone();
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
    compiler_options.named_address_mapping = vec![
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

    run_compile_decompile_test_workflow(&config, compiler_options)?;
    Ok(())
}

fn run_compile_decompile_test_workflow(
    config: &TestConfig,
    compiler_options: Options,
) -> anyhow::Result<String> {
    // Step 1: compile the test case
    let mut error_writer = Buffer::no_color();
    let env = run_move_compiler_for_analysis(&mut error_writer, compiler_options.clone());
    let env = match env {
        Ok(env) => env,
        Err(_) => {
            // skip the test if compilation fails
            print!(
                "\n\nWarning: skipping test `{}` due to compilation failure {}.\n",
                config.name,
                String::from_utf8_lossy(&error_writer.into_inner())
            );
            return Ok(String::default());
        },
    };

    // Step 2: decompile the test case according to the StopAfter option
    decompile_test_case(config, env)?;

    // the test only runs until decompilation
    if config.test_level == TestLevel::Decompile {
        // if we only want to test decompilation, we are done
        return Ok(String::default());
    }

    // Step 3: test if the decompiled code can be recompiled
    {
        // TBD
    }
    // the test only runs until decompilation
    if config.test_level == TestLevel::Recompile {
        // if we only want to test decompilation, we are done
        return Ok(String::default());
    }
    // Step 4: test if the recompiled code can pass available tests
    {
        // TBD
    }
    Ok(String::default())
}

fn decompile_test_case(
    config: &TestConfig,
    env: GlobalEnv,
) -> anyhow::Result<(move_decompiler::Decompiler, FunctionTargetsHolder)> {
    // create a new decompiler instance
    let decompiler_options = move_decompiler::Options {
        no_expressions: false,
        ..move_decompiler::Options::default()
    };
    let mut decompiler = move_decompiler::Decompiler::new(decompiler_options);
    // holder for lifted stackless bytecode
    let mut targets = FunctionTargetsHolder::default();

    // decompile all qualified modules in the global environment
    for module_env in env.get_modules() {
        // filter 1: skip non-primary targets
        if !module_env.is_target() {
            continue;
        }
        // filter 2: skip modules that do not have a CompiledModule attached
        if let Some(compiled_module) = module_env.get_verified_module() {
            let source_map = module_env.get_source_map().cloned().unwrap_or_else(|| {
                let mut bytes = vec![];
                compiled_module
                    .serialize(&mut bytes)
                    .expect("expected serialization success");
                decompiler.empty_source_map(&module_env.get_full_name_str(), &bytes)
            });

            // filter 3: skip modules that do not pass bytecode verification
            if !decompiler.validate_module(compiled_module, &source_map) {
                continue;
            }

            // load the compiled module into decompiler
            let module_id = match decompiler.load_module(compiled_module.clone(), source_map) {
                Some(id) => id,
                None => {
                    return Err(anyhow::anyhow!(
                        "Module `{}` failed during loading",
                        module_env.get_full_name_str()
                    ));
                },
            };

            // lift file format bytecode to stackless bytecode
            decompiler.lift_to_stackless_bytecode(module_id, &mut targets);
            if config.stop_after == StopAfter::StackLessBytecodeGen {
                print!(
                    "[Debug]: Decompilation one module of `{}` to stackless bytecode completed.\n",
                    config.name
                );
                continue;
            }


            let decompile_module_env = decompiler.env().get_module(module_id);
            for func_env in decompile_module_env.get_functions() {
                if func_env.is_inline() {
                    continue;
                }
                for (variant, target) in targets.get_targets(&func_env) {
                    if !target.data.code.is_empty() || target.func_env.is_native_or_intrinsic() {
                        print!("\n[variant {}]\n{}", variant, target);
                    }
                }
            }


            // lift stackless bytecode to AST
            decompiler.lift_to_ast(&targets);
            if config.stop_after == StopAfter::AstGen {
                print!(
                    "[Debug]: Decompilation one module of `{}` to AST completed.\n",
                    config.name
                );
                continue;
            }
            // sourcify the AST
            let res = decompiler.sourcify_ast(module_id);

            let mut error_writer = Buffer::no_color();
            if decompiler
                .env()
                .check_diag(&mut error_writer, Severity::Warning, "decompilation")
                .is_err()
            {
                return Err(anyhow::anyhow!(
                    "Module `{}` failed during decompilation",
                    module_env.get_full_name_str()
                ));
            }
            print!(
                "[Debug]: Decompilation one module of `{}` completed and produce {}.\n",
                config.name, res
            );
        }
    }
    Ok((decompiler, targets))
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
        if !config.is_package {
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
        } else {
            if WalkDir::new(config.sources)
                .follow_links(false)
                .min_depth(1)
                .into_iter()
                .filter_map(Result::ok)
                .any(|e| e.path().extension() == Some("move".as_ref()))
            {
                test_groups
                    .entry(config.name)
                    .or_default()
                    .push(config.sources.to_string());
                found_one = true;
            }
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
        if !config.is_package {
            for file in src_target {
                let test_prompt =
                    format!("decompiler[config={}]::move-file::{}", config.name, file);
                let test_path = PathBuf::from(file);
                let runner = config.runner;
                tests.push(Trial::test(test_prompt, move || {
                    runner(&test_path).map_err(|err| format!("{:?}", err).into())
                }));
            }
        } else {
            assert!(
                src_target.len() == 1,
                "Error: package tests {} should have exactly one source folder",
                config.name
            );
            let folder = src_target[0].clone();
            let test_prompt = format!(
                "decompiler[config={}]::move-package::{}",
                config.name, folder
            );
            let test_path = PathBuf::from(folder);
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
