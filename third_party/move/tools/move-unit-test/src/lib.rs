// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

pub mod extensions;
mod filter;
pub mod test_reporter;
pub mod test_runner;
mod filter;

pub use crate::filter::FilterOptions;
use crate::test_runner::TestRunner;
use clap::*;
use legacy_move_compiler::{
    self,
    shared::{self, NumericalAddress},
    unit_test::TestPlan,
};
use move_command_line_common::files::verify_and_create_named_address_mapping;
use move_compiler_v2::plan_builder as plan_builder_v2;
use move_core_types::{effects::ChangeSet, language_storage::ModuleId};
use move_model::metadata::{CompilerVersion, LanguageVersion};
use move_package::compilation::compiled_package::build_and_report_v2_driver;
use move_vm_runtime::native_functions::NativeFunctionTable;
use std::{
    collections::BTreeMap,
    io::{Result, Write},
    marker::Send,
    sync::Mutex,
};
use test_reporter::UnitTestFactory;

pub use crate::filter::FilterOptions;

/// The default value bounding the amount of gas consumed in a test.
const DEFAULT_EXECUTION_BOUND: u64 = 1_000_000;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about)]
pub struct UnitTestingConfig {
    #[clap(flatten)]
    pub filter_options: FilterOptions,

    /// List all tests
    #[clap(name = "list", short = 'l', long = "list")]
    pub list: bool,

    /// Number of threads to use for running tests.
    #[clap(
        name = "num_threads",
        default_value_t = 8,
        short = 't',
        long = "threads"
    )]
    pub num_threads: usize,

    /// Dependency files
    #[clap(
        name = "dependencies",
        long = "dependencies",
        short = 'd',
        num_args = 0..
    )]
    pub dep_files: Vec<String>,

    /// Report test statistics at the end of testing
    #[clap(name = "report_statistics", short = 's', long = "statistics")]
    pub report_statistics: bool,

    /// Show the storage state at the end of execution of a failing test
    #[clap(name = "global_state_on_error", short = 'g', long = "state_on_error")]
    pub report_storage_on_error: bool,

    #[clap(
        name = "report_stacktrace_on_abort",
        short = 'r',
        long = "stacktrace_on_abort"
    )]
    pub report_stacktrace_on_abort: bool,

    /// Ignore compiler's warning, and continue run tests
    #[clap(name = "ignore_compile_warnings", long = "ignore_compile_warnings")]
    pub ignore_compile_warnings: bool,

    /// Named address mapping
    #[clap(
        name = "NAMED_ADDRESSES",
        short = 'a',
        long = "addresses",
        value_parser = shared::parse_named_address
    )]
    pub named_address_values: Vec<(String, NumericalAddress)>,

    /// Source files
    #[clap(
        name = "sources",
        num_args = 0..
    )]
    pub source_files: Vec<String>,

    /// Verbose mode
    #[clap(short = 'v', long = "verbose")]
    pub verbose: bool,
}

fn format_module_id(module_id: &ModuleId) -> String {
    format!(
        "0x{}::{}",
        module_id.address().short_str_lossless(),
        module_id.name()
    )
}

impl Default for UnitTestingConfig {
    fn default() -> Self {
        Self {
            filter_options: FilterOptions::default(),
            num_threads: 8,
            report_statistics: false,
            report_storage_on_error: false,
            report_stacktrace_on_abort: true,
            ignore_compile_warnings: false,
            source_files: vec![],
            dep_files: vec![],
            verbose: false,
            list: false,
            named_address_values: vec![],
        }
    }
}

impl UnitTestingConfig {
    pub fn with_named_addresses(
        mut self,
        named_address_values: BTreeMap<String, NumericalAddress>,
    ) -> Self {
        assert!(self.named_address_values.is_empty());
        self.named_address_values = named_address_values.into_iter().collect();
        self
    }

    fn compile_to_test_plan(
        &self,
        source_files: Vec<String>,
        deps: Vec<String>,
    ) -> Option<TestPlan> {
        let addresses =
            verify_and_create_named_address_mapping(self.named_address_values.clone()).ok()?;
        let (test_plan, files, units) = {
            let options = move_compiler_v2::Options {
                compile_test_code: true,
                testing: true,
                sources: source_files,
                dependencies: deps,
                compiler_version: Some(CompilerVersion::latest_stable()),
                language_version: Some(LanguageVersion::latest_stable()),
                named_address_mapping: addresses
                    .iter()
                    .map(|(string, num_addr)| format!("{}={}", string, num_addr))
                    .collect(),
                ..Default::default()
            };
            let (files, units, env) = build_and_report_v2_driver(options).unwrap();
            let test_plan = plan_builder_v2::construct_test_plan(&env, None);
            (test_plan, files, units)
        };
        test_plan.map(|tests| TestPlan::new(tests, files, units, vec![]))
    }

    /// Build a test plan from a unit test config
    pub fn build_test_plan(&self) -> Option<TestPlan> {
        let deps = self.dep_files.clone();

        let TestPlan {
            files, module_info, ..
        } = self.compile_to_test_plan(deps.clone(), vec![])?;

        let mut test_plan = self.compile_to_test_plan(self.source_files.clone(), deps)?;
        test_plan.module_info.extend(module_info);
        test_plan.files.extend(files);
        Some(test_plan)
    }

    /// Public entry point to Move unit testing as a library
    /// Returns `true` if all unit tests passed. Otherwise, returns `false`.
    pub fn run_and_report_unit_tests<W: Write + Send, F: UnitTestFactory + Send>(
        &self,
        test_plan: TestPlan,
        native_function_table: Option<NativeFunctionTable>,
        genesis_state: Option<ChangeSet>,
        writer: W,
        factory: F,
    ) -> Result<(W, bool)> {
        let shared_writer = Mutex::new(writer);
        let shared_options = Mutex::new(factory);

        if self.list {
            for (module_id, test_plan) in &test_plan.module_tests {
                for test_name in test_plan.tests.keys() {
                    writeln!(
                        shared_writer.lock().unwrap(),
                        "{}::{}: test",
                        format_module_id(module_id),
                        test_name
                    )?;
                }
            }
            return Ok((shared_writer.into_inner().unwrap(), true));
        }

        writeln!(shared_writer.lock().unwrap(), "Running Move unit tests")?;
        let mut test_runner = TestRunner::new(
            self.num_threads,
            self.report_storage_on_error,
            self.report_stacktrace_on_abort,
            test_plan,
            native_function_table,
            genesis_state,
            self.verbose,
        )
        .unwrap();

        test_runner.filter(&self.filter_options);

        let test_results = test_runner.run(&shared_writer, &shared_options).unwrap();
        if self.report_statistics {
            test_results.report_statistics(&shared_writer)?;
        }

        if self.verbose {
            test_results.report_goldens(&shared_writer)?;
        }

        let ok = test_results.summarize(&shared_writer)?;

        let writer = shared_writer.into_inner().unwrap();
        Ok((writer, ok))
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    UnitTestingConfig::command().debug_assert()
}
