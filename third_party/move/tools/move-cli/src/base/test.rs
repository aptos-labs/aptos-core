// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use super::reroot_path;
use crate::{base::test_validation, NativeFunctionRecord};
use anyhow::{bail, Result};
use clap::*;
use codespan_reporting::term::{termcolor, termcolor::StandardStream};
use move_command_line_common::files::{FileHash, MOVE_COVERAGE_MAP_EXTENSION};
use move_compiler::{
    diagnostics::{self, codes::Severity},
    shared::{NumberFormat, NumericalAddress},
    unit_test::{plan_builder::construct_test_plan, TestPlan},
    PASS_CFGIR,
};
use move_compiler_v2::plan_builder as plan_builder_v2;
use move_core_types::effects::ChangeSet;
use move_coverage::coverage_map::{output_map_to_file, CoverageMap};
use move_package::{
    compilation::{build_plan::BuildPlan, compiled_package::build_and_report_v2_driver},
    BuildConfig,
};
use move_unit_test::{
    test_reporter::{UnitTestFactory, UnitTestFactoryWithCostTable},
    UnitTestingConfig,
};
use move_vm_runtime::tracing::{LOGGING_FILE_WRITER, TRACING_ENABLED};
use move_vm_test_utils::gas_schedule::CostTable;
// if unix
#[cfg(target_family = "unix")]
use std::os::unix::prelude::ExitStatusExt;
// if windows
#[cfg(target_family = "windows")]
use std::os::windows::process::ExitStatusExt;
use std::{
    collections::HashMap,
    fs,
    io::Write,
    ops::Deref,
    path::{Path, PathBuf},
    process::ExitStatus,
};
// if not windows nor unix
#[cfg(not(any(target_family = "windows", target_family = "unix")))]
compile_error!("Unsupported OS, currently we only support windows and unix family");

/// Run Move unit tests in this package.
#[derive(Parser)]
#[clap(name = "test")]
pub struct Test {
    /// Bound the amount of gas used by any one test.
    #[clap(name = "gas_limit", short = 'i', long = "gas_limit")]
    pub gas_limit: Option<u64>,
    /// An optional filter string to determine which unit tests to run. A unit test will be run only if it
    /// contains this string in its fully qualified (`<addr>::<module_name>::<fn_name>`) name.
    #[clap(name = "filter")]
    pub filter: Option<String>,
    /// List all tests
    #[clap(name = "list", short = 'l', long = "list")]
    pub list: bool,
    /// Number of threads to use for running tests.
    #[clap(
        name = "num_threads",
        default_value = "8",
        short = 't',
        long = "threads"
    )]
    pub num_threads: usize,
    /// Report test statistics at the end of testing
    #[clap(name = "report_statistics", short = 's', long = "statistics")]
    pub report_statistics: bool,
    /// Show the storage state at the end of execution of a failing test
    #[clap(name = "global_state_on_error", short = 'g', long = "state_on_error")]
    pub report_storage_on_error: bool,

    /// Ignore compiler's warning, and continue run tests
    #[clap(name = "ignore_compile_warnings", long = "ignore_compile_warnings")]
    pub ignore_compile_warnings: bool,

    /// Use the stackless bytecode interpreter to run the tests and cross check its results with
    /// the execution result from Move VM.
    #[clap(long = "stackless")]
    pub check_stackless_vm: bool,
    /// Verbose mode
    #[clap(long = "verbose")]
    pub verbose_mode: bool,
    /// Collect coverage information for later use with the various `move coverage` subcommands
    #[clap(long = "coverage")]
    pub compute_coverage: bool,

    /// Use the EVM-based execution backend.
    /// Does not work with --stackless.
    #[cfg(feature = "evm-backend")]
    #[clap(long = "evm")]
    pub evm: bool,
}

impl Test {
    pub fn execute(
        self,
        path: Option<PathBuf>,
        config: BuildConfig,
        natives: Vec<NativeFunctionRecord>,
        genesis: ChangeSet,
        cost_table: Option<CostTable>,
    ) -> anyhow::Result<()> {
        let rerooted_path = reroot_path(path)?;
        let Self {
            gas_limit,
            filter,
            list,
            num_threads,
            report_statistics,
            report_storage_on_error,
            ignore_compile_warnings,
            check_stackless_vm,
            verbose_mode,
            compute_coverage,
            #[cfg(feature = "evm-backend")]
            evm,
        } = self;
        let unit_test_config = UnitTestingConfig {
            filter,
            list,
            num_threads,
            report_statistics,
            report_storage_on_error,
            check_stackless_vm,
            verbose: verbose_mode,
            ignore_compile_warnings,
            #[cfg(feature = "evm-backend")]
            evm,

            ..UnitTestingConfig::default()
        };
        let result = run_move_unit_tests(
            &rerooted_path,
            config,
            unit_test_config,
            natives,
            genesis,
            gas_limit,
            cost_table,
            compute_coverage,
            &mut std::io::stdout(),
        )?;

        // Return a non-zero exit code if any test failed
        if let UnitTestResult::Failure = result {
            std::process::exit(1)
        }
        Ok(())
    }
}

/// Encapsulates the possible returned states when running unit tests on a move package.
#[derive(PartialEq, Eq, Debug)]
pub enum UnitTestResult {
    Success,
    Failure,
}

pub fn run_move_unit_tests<W: Write + Send>(
    pkg_path: &Path,
    build_config: move_package::BuildConfig,
    unit_test_config: UnitTestingConfig,
    natives: Vec<NativeFunctionRecord>,
    genesis: ChangeSet,
    gas_limit: Option<u64>,
    cost_table: Option<CostTable>,
    compute_coverage: bool,
    writer: &mut W,
) -> Result<UnitTestResult> {
    run_move_unit_tests_with_factory(
        pkg_path,
        build_config,
        unit_test_config,
        natives,
        genesis,
        compute_coverage,
        writer,
        UnitTestFactoryWithCostTable::new(cost_table, gas_limit),
    )
}

pub fn run_move_unit_tests_with_factory<W: Write + Send, F: UnitTestFactory + Send>(
    pkg_path: &Path,
    mut build_config: move_package::BuildConfig,
    mut unit_test_config: UnitTestingConfig,
    natives: Vec<NativeFunctionRecord>,
    genesis: ChangeSet,
    compute_coverage: bool,
    writer: &mut W,
    factory: F,
) -> Result<UnitTestResult> {
    let mut test_plan = None;
    let mut test_plan_v2 = None;

    build_config.test_mode = true;
    build_config.dev_mode = true;
    build_config.generate_move_model = test_validation::needs_validation();

    // Build the resolution graph (resolution graph diagnostics are only needed for CLI commands so
    // ignore them by passing a vector as the writer)
    let resolution_graph = build_config
        .clone()
        .resolution_graph_for_package(pkg_path, &mut Vec::new())?;

    // Note: unit_test_config.named_address_values is always set to vec![] (the default value) before
    // being passed in.
    unit_test_config.named_address_values = resolution_graph
        .extract_named_address_mapping()
        .map(|(name, addr)| {
            (
                name.to_string(),
                NumericalAddress::new(addr.into_bytes(), NumberFormat::Hex),
            )
        })
        .collect();

    // Get the source files for all modules. We need this in order to report source-mapped error
    // messages.
    let dep_file_map: HashMap<_, _> = resolution_graph
        .package_table
        .iter()
        .flat_map(|(_, rpkg)| {
            rpkg.get_sources(&resolution_graph.build_options)
                .unwrap()
                .iter()
                .map(|fname| {
                    let contents = fs::read_to_string(Path::new(fname.as_str())).unwrap();
                    let fhash = FileHash::new(&contents);
                    (fhash, (*fname, contents))
                })
                .collect::<HashMap<_, _>>()
        })
        .collect();
    let root_package = resolution_graph.root_package.package.name;
    let build_plan = BuildPlan::create(resolution_graph)?;
    // Compile the package. We need to intercede in the compilation, process being performed by the
    // Move package system, to first grab the compilation env, construct the test plan from it, and
    // then save it, before resuming the rest of the compilation and returning the results and
    // control back to the Move package system.
    let (_compiled_package, model_opt) = build_plan.compile_with_driver(
        writer,
        &build_config.compiler_config,
        vec![],
        |compiler| {
            let (files, comments_and_compiler_res) = compiler.run::<PASS_CFGIR>().unwrap();
            let (_, compiler) =
                diagnostics::unwrap_or_report_diagnostics(&files, comments_and_compiler_res);
            let (mut compiler, cfgir) = compiler.into_ast();
            let compilation_env = compiler.compilation_env();
            let built_test_plan = construct_test_plan(compilation_env, Some(root_package), &cfgir);
            if let Err(diags) = compilation_env.check_diags_at_or_above_severity(
                if unit_test_config.ignore_compile_warnings {
                    Severity::NonblockingError
                } else {
                    Severity::Warning
                },
            ) {
                diagnostics::report_diagnostics_exit_on_error(&files, diags);
            }

            let compilation_result = compiler.at_cfgir(cfgir).build();

            let (units, _) = diagnostics::unwrap_or_report_diagnostics(&files, compilation_result);
            test_plan = Some((built_test_plan, files.clone(), units.clone()));
            Ok((files, units, None))
        },
        |options| {
            let (files, units, opt_env) = build_and_report_v2_driver(options).unwrap();
            let env = opt_env.expect("v2 driver should return env");
            let root_package_in_model = env.symbol_pool().make(root_package.deref());
            let built_test_plan =
                plan_builder_v2::construct_test_plan(&env, Some(root_package_in_model));

            test_plan_v2 = Some((built_test_plan, files.clone(), units.clone()));
            Ok((files, units, Some(env)))
        },
    )?;

    // If configured, run extra validation
    if test_validation::needs_validation() {
        let model = &model_opt.expect("move model");
        test_validation::validate(model);
        let diag_count = model.diag_count(codespan_reporting::diagnostic::Severity::Warning);
        if diag_count > 0 {
            model.report_diag(
                &mut StandardStream::stderr(termcolor::ColorChoice::Auto),
                codespan_reporting::diagnostic::Severity::Warning,
            );
            if model.has_errors() {
                bail!("compilation failed")
            }
        }
    }

    let test_plan = if test_plan.is_some() {
        test_plan
    } else {
        test_plan_v2
    };

    let (test_plan, mut files, units) = test_plan.unwrap();
    files.extend(dep_file_map);
    let test_plan = test_plan.unwrap();
    let no_tests = test_plan.is_empty();
    let test_plan = TestPlan::new(test_plan, files, units);

    let trace_path = pkg_path.join(".trace");
    let coverage_map_path = pkg_path
        .join(".coverage_map")
        .with_extension(MOVE_COVERAGE_MAP_EXTENSION);
    let cleanup_trace = || {
        if compute_coverage && trace_path.exists() {
            std::fs::remove_file(&trace_path).unwrap();
        }
    };

    cleanup_trace();

    // If we need to compute test coverage set the VM tracking environment variable since we will
    // need this trace to construct the coverage information.
    if compute_coverage {
        std::env::set_var("MOVE_VM_TRACE", &trace_path);
    }

    // Run the tests. If any of the tests fail, then we don't produce a coverage report, so cleanup
    // the trace files.
    if !unit_test_config
        .run_and_report_unit_tests(test_plan, Some(natives), Some(genesis), writer, factory)
        .unwrap()
        .1
    {
        cleanup_trace();
        return Ok(UnitTestResult::Failure);
    }

    // Compute the coverage map. This will be used by other commands after this.
    if compute_coverage && !no_tests {
        if *TRACING_ENABLED {
            let buf_writer = &mut *LOGGING_FILE_WRITER.lock().unwrap();
            buf_writer.flush().unwrap();
        }
        let coverage_map = CoverageMap::from_trace_file(trace_path.clone());
        output_map_to_file(coverage_map_path, &coverage_map).unwrap();
    }
    cleanup_trace();
    Ok(UnitTestResult::Success)
}

impl From<UnitTestResult> for ExitStatus {
    fn from(result: UnitTestResult) -> Self {
        match result {
            UnitTestResult::Success => ExitStatus::from_raw(0),
            UnitTestResult::Failure => ExitStatus::from_raw(1),
        }
    }
}
