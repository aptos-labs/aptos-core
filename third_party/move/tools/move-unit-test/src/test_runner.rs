// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    extensions, format_module_id,
    test_reporter::{
        FailureReason, MoveError, TestFailure, TestResults, TestRunInfo, TestStatistics,
        UnitTestFactory,
    },
};
use anyhow::Result;
use colored::*;
use move_binary_format::{errors::VMResult, file_format::CompiledModule};
use move_bytecode_utils::Modules;
use move_compiler::unit_test::{ExpectedFailure, ModuleTestPlan, TestCase, TestPlan};
use move_core_types::{
    account_address::AccountAddress,
    effects::{ChangeSet, Op},
    identifier::IdentStr,
    value::serialize_values,
    vm_status::StatusCode,
};
use move_resource_viewer::MoveValueAnnotator;
use move_vm_runtime::{
    module_traversal::{TraversalContext, TraversalStorage},
    move_vm::MoveVM,
    native_extensions::NativeContextExtensions,
    native_functions::NativeFunctionTable,
};
use move_vm_test_utils::InMemoryStorage;
use rayon::prelude::*;
use std::{io::Write, marker::Send, sync::Mutex, time::Instant};
#[cfg(feature = "evm-backend")]
use {
    evm::{backend::MemoryVicinity, ExitReason},
    evm_exec_utils::exec::{ExecuteResult, Executor},
    move_to_yul,
    primitive_types::{H160, U256},
    std::convert::TryInto,
    std::time::Duration,
};

/// Test state common to all tests
pub struct SharedTestingConfig {
    save_storage_state_on_failure: bool,
    report_stacktrace_on_abort: bool,
    native_function_table: NativeFunctionTable,
    starting_storage_state: InMemoryStorage,
    #[allow(dead_code)] // used by some features
    source_files: Vec<String>,
    record_writeset: bool,

    #[cfg(feature = "evm-backend")]
    evm: bool,
}

pub struct TestRunner {
    num_threads: usize,
    testing_config: SharedTestingConfig,
    tests: TestPlan,
}

/// Setup storage state with the set of modules that will be needed for all tests
fn setup_test_storage<'a>(
    modules: impl Iterator<Item = &'a CompiledModule>,
) -> Result<InMemoryStorage> {
    let mut storage = InMemoryStorage::new();
    let modules = Modules::new(modules);
    for module in modules
        .compute_dependency_graph()
        .compute_topological_order()?
    {
        let module_id = module.self_id();
        let mut module_bytes = Vec::new();
        module.serialize_for_version(Some(module.version), &mut module_bytes)?;
        storage.publish_or_overwrite_module(module_id, module_bytes);
    }

    Ok(storage)
}

/// Print the updates to storage represented by `cs` in the context of the starting storage state
/// `storage`.
fn print_resources_and_extensions(
    cs: &ChangeSet,
    extensions: &mut NativeContextExtensions,
    storage: &InMemoryStorage,
) -> Result<String> {
    use std::fmt::Write;
    let mut buf = String::new();
    let annotator = MoveValueAnnotator::new(storage.clone());
    for (account_addr, account_state) in cs.accounts() {
        writeln!(&mut buf, "0x{}:", account_addr.short_str_lossless())?;

        for (tag, resource_op) in account_state.resources() {
            if let Op::New(resource) | Op::Modify(resource) = resource_op {
                writeln!(
                    &mut buf,
                    "\t{}",
                    format!("=> {}", annotator.view_resource(tag, resource)?).replace('\n', "\n\t")
                )?;
            }
        }
    }

    extensions::print_change_sets(&mut buf, extensions);

    Ok(buf)
}

impl TestRunner {
    pub fn new(
        num_threads: usize,
        save_storage_state_on_failure: bool,
        report_stacktrace_on_abort: bool,
        tests: TestPlan,
        // TODO: maybe we should require the clients to always pass in a list of native functions so
        // we don't have to make assumptions about their gas parameters.
        native_function_table: Option<NativeFunctionTable>,
        genesis_state: Option<ChangeSet>,
        record_writeset: bool,
        #[cfg(feature = "evm-backend")] evm: bool,
    ) -> Result<Self> {
        let source_files = tests
            .files
            .values()
            .map(|(filepath, _)| filepath.to_string())
            .collect();
        let modules = tests.module_info.values().map(|info| &info.module);
        let mut starting_storage_state = setup_test_storage(modules)?;
        if let Some(genesis_state) = genesis_state {
            starting_storage_state.apply(genesis_state)?;
        }
        let native_function_table = native_function_table.unwrap_or_else(|| {
            move_stdlib::natives::all_natives(
                AccountAddress::from_hex_literal("0x1").unwrap(),
                move_stdlib::natives::GasParameters::zeros(),
            )
        });
        Ok(Self {
            testing_config: SharedTestingConfig {
                save_storage_state_on_failure,
                report_stacktrace_on_abort,
                starting_storage_state,
                native_function_table,
                source_files,
                record_writeset,
                #[cfg(feature = "evm-backend")]
                evm,
            },
            num_threads,
            tests,
        })
    }

    pub fn run<W: Write + Send, F: UnitTestFactory + Send>(
        self,
        writer: &Mutex<W>,
        options: &Mutex<F>,
    ) -> Result<TestResults> {
        rayon::ThreadPoolBuilder::new()
            .num_threads(self.num_threads)
            .build()
            .unwrap()
            .install(|| {
                let final_statistics = self
                    .tests
                    .module_tests
                    .par_iter()
                    .map(|(_, test_plan)| {
                        self.testing_config
                            .exec_module_tests(test_plan, writer, options)
                    })
                    .reduce(TestStatistics::new, |acc, stats| acc.combine(stats));

                Ok(TestResults::new(final_statistics, self.tests))
            })
    }

    pub fn filter(&mut self, test_name_slice: &str) {
        for (module_id, module_test) in self.tests.module_tests.iter_mut() {
            if module_id.name().as_str().contains(test_name_slice) {
                continue;
            } else {
                let tests = std::mem::take(&mut module_test.tests);
                module_test.tests = tests
                    .into_iter()
                    .filter(|(test_name, _)| {
                        let full_name =
                            format!("{}::{}", module_id.name().as_str(), test_name.as_str());
                        full_name.contains(test_name_slice)
                    })
                    .collect();
            }
        }
    }
}

// TODO: do not expose this to backend implementations
struct TestOutput<'a, 'b, W> {
    test_plan: &'a ModuleTestPlan,
    writer: &'b Mutex<W>,
}

impl<'a, 'b, W: Write> TestOutput<'a, 'b, W> {
    fn pass(&self, fn_name: &str) {
        writeln!(
            self.writer.lock().unwrap(),
            "[ {}    ] {}::{}",
            "PASS".bold().bright_green(),
            format_module_id(&self.test_plan.module_id),
            fn_name
        )
        .unwrap()
    }

    fn fail(&self, fn_name: &str) {
        writeln!(
            self.writer.lock().unwrap(),
            "[ {}    ] {}::{}",
            "FAIL".bold().bright_red(),
            format_module_id(&self.test_plan.module_id),
            fn_name,
        )
        .unwrap()
    }

    fn timeout(&self, fn_name: &str) {
        writeln!(
            self.writer.lock().unwrap(),
            "[ {} ] {}::{}",
            "TIMEOUT".bold().bright_yellow(),
            format_module_id(&self.test_plan.module_id),
            fn_name,
        )
        .unwrap();
    }
}

impl SharedTestingConfig {
    #[allow(clippy::field_reassign_with_default)]
    fn execute_via_move_vm<F: UnitTestFactory>(
        &self,
        test_plan: &ModuleTestPlan,
        function_name: &str,
        test_info: &TestCase,
        factory: &Mutex<F>,
    ) -> (
        VMResult<ChangeSet>,
        VMResult<NativeContextExtensions>,
        VMResult<Vec<Vec<u8>>>,
        TestRunInfo,
    ) {
        let move_vm = MoveVM::new(self.native_function_table.clone());
        let extensions = extensions::new_extensions();
        let mut session =
            move_vm.new_session_with_extensions(&self.starting_storage_state, extensions);
        let mut gas_meter = factory.lock().unwrap().new_gas_meter();

        // TODO: collect VM logs if the verbose flag (i.e, `self.verbose`) is set

        let now = Instant::now();
        let storage = TraversalStorage::new();
        let serialized_return_values_result = session.execute_function_bypass_visibility(
            &test_plan.module_id,
            IdentStr::new(function_name).unwrap(),
            vec![], // no ty args, at least for now
            serialize_values(test_info.arguments.iter()),
            &mut gas_meter,
            &mut TraversalContext::new(&storage),
        );
        let mut return_result = serialized_return_values_result.map(|res| {
            res.return_values
                .into_iter()
                .map(|(bytes, _layout)| bytes)
                .collect()
        });
        if !self.report_stacktrace_on_abort {
            if let Err(err) = &mut return_result {
                err.remove_exec_state();
            }
        }

        let test_run_info = TestRunInfo::new(function_name.to_string(), now.elapsed());
        match session.finish_with_extensions() {
            Ok((cs, mut extensions)) => {
                let finalized_test_run_info = factory.lock().unwrap().finalize_test_run_info(
                    &cs,
                    &mut extensions,
                    gas_meter,
                    test_run_info,
                );

                (
                    Ok(cs),
                    Ok(extensions),
                    return_result,
                    finalized_test_run_info,
                )
            },
            Err(err) => (Err(err.clone()), Err(err), return_result, test_run_info),
        }
    }

    fn exec_module_tests_move_vm_and_stackless_vm<F: UnitTestFactory>(
        &self,
        test_plan: &ModuleTestPlan,
        output: &TestOutput<impl Write>,
        factory: &Mutex<F>,
    ) -> TestStatistics {
        let mut stats = TestStatistics::new();

        for (function_name, test_info) in &test_plan.tests {
            let (cs_result, ext_result, exec_result, test_run_info) =
                self.execute_via_move_vm(test_plan, function_name, test_info, factory);

            if self.record_writeset {
                stats.test_output(
                    function_name.to_string(),
                    test_plan,
                    format!("{:?}", cs_result),
                );
            }

            let save_session_state = || {
                if self.save_storage_state_on_failure {
                    cs_result.ok().and_then(|changeset| {
                        ext_result.ok().and_then(|mut extensions| {
                            print_resources_and_extensions(
                                &changeset,
                                &mut extensions,
                                &self.starting_storage_state,
                            )
                            .ok()
                        })
                    })
                } else {
                    None
                }
            };

            match exec_result {
                Err(err) => {
                    let actual_err = MoveError(
                        err.major_status(),
                        err.sub_status(),
                        err.location().clone(),
                        err.message().cloned(),
                    );
                    assert!(err.major_status() != StatusCode::EXECUTED);
                    match test_info.expected_failure.as_ref() {
                        Some(ExpectedFailure::Expected) => {
                            output.pass(function_name);
                            stats.test_success(test_run_info, test_plan);
                        },
                        Some(ExpectedFailure::ExpectedWithError(expected_err))
                            if expected_err == &actual_err =>
                        {
                            output.pass(function_name);
                            stats.test_success(test_run_info, test_plan);
                        },
                        Some(ExpectedFailure::ExpectedWithCodeDEPRECATED(code))
                            if actual_err.0 == StatusCode::ABORTED
                                && actual_err.1.is_some()
                                && actual_err.1.unwrap() == *code =>
                        {
                            output.pass(function_name);
                            stats.test_success(test_run_info, test_plan);
                        },
                        // incorrect cases
                        Some(ExpectedFailure::ExpectedWithError(expected_err)) => {
                            output.fail(function_name);
                            stats.test_failure(
                                TestFailure::new(
                                    FailureReason::wrong_error(expected_err.clone(), actual_err),
                                    test_run_info,
                                    Some(err),
                                    save_session_state(),
                                ),
                                test_plan,
                            )
                        },
                        Some(ExpectedFailure::ExpectedWithCodeDEPRECATED(expected_code)) => {
                            output.fail(function_name);
                            stats.test_failure(
                                TestFailure::new(
                                    FailureReason::wrong_abort_deprecated(
                                        *expected_code,
                                        actual_err,
                                    ),
                                    test_run_info,
                                    Some(err),
                                    save_session_state(),
                                ),
                                test_plan,
                            )
                        },
                        None if err.major_status() == StatusCode::OUT_OF_GAS => {
                            // Ran out of ticks, report a test timeout and log a test failure
                            output.timeout(function_name);
                            stats.test_failure(
                                TestFailure::new(
                                    FailureReason::timeout(),
                                    test_run_info,
                                    Some(err),
                                    save_session_state(),
                                ),
                                test_plan,
                            )
                        },
                        None => {
                            output.fail(function_name);
                            stats.test_failure(
                                TestFailure::new(
                                    FailureReason::unexpected_error(actual_err),
                                    test_run_info,
                                    Some(err),
                                    save_session_state(),
                                ),
                                test_plan,
                            )
                        },
                    }
                },
                Ok(_) => {
                    // Expected the test to fail, but it executed
                    if test_info.expected_failure.is_some() {
                        output.fail(function_name);
                        stats.test_failure(
                            TestFailure::new(
                                FailureReason::no_error(),
                                test_run_info,
                                None,
                                save_session_state(),
                            ),
                            test_plan,
                        )
                    } else {
                        // Expected the test to execute fully and it did
                        output.pass(function_name);
                        stats.test_success(test_run_info, test_plan);
                    }
                },
            }
        }

        stats
    }

    #[cfg(feature = "evm-backend")]
    fn execute_via_evm(&self, yul_source: &str) -> (ExecuteResult, Duration) {
        let (code, _) = evm_exec_utils::compile::solc_yul(yul_source, false).expect(
            "Failed to compile yul source into EVM bytecode. This should not have happened.",
        );

        let vicinity = MemoryVicinity {
            gas_price: 0.into(),
            origin: H160::zero(),
            chain_id: 0.into(),
            block_hashes: vec![],
            block_number: 0.into(),
            block_coinbase: H160::zero(),
            block_timestamp: 0.into(),
            block_difficulty: 0.into(),
            block_gas_limit: U256::MAX,
            block_base_fee_per_gas: 0.into(),
        };

        let mut exec = Executor::new(&vicinity);

        let now = Instant::now();
        let res = exec.execute_custom_code(H160::zero(), H160::zero(), code, vec![]);
        let elapsed = now.elapsed();

        (res, elapsed)
    }

    #[cfg(feature = "evm-backend")]
    fn exec_module_tests_evm(
        &self,
        test_plan: &ModuleTestPlan,
        output: &TestOutput<impl Write>,
    ) -> TestStatistics {
        use move_binary_format::errors::Location;

        let mut stats = TestStatistics::new();

        // TODO: Somehow, paths of some temporary Move interface files are being passed in after those files
        // have been removed. This is a dirty hack to work around the problem while we investigate the root
        // cause.
        let filtered_sources = self
            .source_files
            .iter()
            .filter(|s| !s.contains("mv_interfaces"))
            .cloned()
            .collect::<Vec<_>>();

        let model = run_model_builder_with_options_and_compilation_flags(
            vec![PackagePaths {
                name: None,
                paths: filtered_sources,
                named_address_map: self.named_address_values.clone(),
            }],
            vec![],
            ModelBuilderOptions::default(),
            Flags::testing(),
        )
        .unwrap_or_else(|e| panic!("Unable to build move model: {}", e));

        if model.has_errors() {
            panic!("Move model has errors");
        }

        let gen_options = move_to_yul::options::Options::default();
        for (function_name, test_info) in &test_plan.tests {
            let yul_code = match move_to_yul::generator::Generator::run_for_unit_test(
                &gen_options,
                &model,
                &test_plan.module_id,
                IdentStr::new(function_name).unwrap(),
                &test_info.arguments,
            ) {
                Ok(yul_code) => yul_code,
                Err(diagnostics) => {
                    // Failed to generate yul code due to some user errors.
                    // Mark test as failed.
                    output.fail(function_name);
                    stats.test_failure(
                        TestFailure::new(
                            FailureReason::move_to_evm_error(diagnostics),
                            TestRunInfo::new(function_name.to_string(), Duration::ZERO, 0),
                            None,
                            None,
                        ),
                        test_plan,
                    );
                    return stats;
                },
            };

            let (res, duration) = self.execute_via_evm(&yul_code);

            let abort_code = || -> u64 {
                assert!(res.return_value.len() == 8);

                u64::from_be_bytes(res.return_value.as_slice().try_into().unwrap())
            };

            let test_run_info =
                || -> TestRunInfo { TestRunInfo::new(function_name.to_string(), duration, 0) };

            // TODO: gas/timeout
            // TODO: arguments
            // TODO: locations

            match (test_info.expected_failure.as_ref(), &res.exit_reason) {
                // Test expected to succeed or abort with a specific abort code, but ran into an internal error.
                (
                    None
                    | Some(
                        ExpectedFailure::ExpectedWithCodeDEPRECATED(_)
                        | ExpectedFailure::ExpectedWithError(_),
                    ),
                    ExitReason::Revert(_),
                ) if abort_code() == u64::MAX => {
                    output.fail(function_name);
                    stats.test_failure(
                        TestFailure::new(
                            FailureReason::unexpected_error(MoveError(
                                StatusCode::UNKNOWN_STATUS,
                                None,
                                Location::Undefined,
                            )),
                            test_run_info(),
                            None,
                            None,
                        ),
                        test_plan,
                    );
                },

                // Test expected to succeed, but aborted.
                (None, ExitReason::Revert(_)) => {
                    output.fail(function_name);
                    stats.test_failure(
                        TestFailure::new(
                            FailureReason::unexpected_error(MoveError(
                                StatusCode::ABORTED,
                                Some(abort_code()),
                                Location::Undefined,
                            )),
                            test_run_info(),
                            None,
                            None,
                        ),
                        test_plan,
                    )
                },

                // Expect the test to abort with a specific code.
                (
                    Some(
                        ExpectedFailure::ExpectedWithError(MoveError(_, Some(exp_abort_code), _))
                        | ExpectedFailure::ExpectedWithCodeDEPRECATED(exp_abort_code),
                    ),
                    ExitReason::Revert(_),
                ) => {
                    let abort_code = abort_code();
                    if abort_code == *exp_abort_code {
                        output.pass(function_name);
                        stats.test_success(test_run_info(), test_plan);
                    } else {
                        output.fail(function_name);
                        stats.test_failure(
                            TestFailure::new(
                                FailureReason::wrong_abort_deprecated(
                                    *exp_abort_code,
                                    MoveError(
                                        StatusCode::ABORTED,
                                        Some(abort_code),
                                        Location::Undefined,
                                    ),
                                ),
                                test_run_info(),
                                None,
                                None,
                            ),
                            test_plan,
                        );
                    }
                },

                // Test expected to abort but succeeded.
                (
                    Some(
                        ExpectedFailure::Expected
                        | ExpectedFailure::ExpectedWithCodeDEPRECATED(_)
                        | ExpectedFailure::ExpectedWithError(_),
                    ),
                    ExitReason::Succeed(_),
                ) => {
                    output.fail(function_name);
                    stats.test_failure(
                        TestFailure::new(FailureReason::no_error(), test_run_info(), None, None),
                        test_plan,
                    )
                },

                // Test succeeded or failed as expected.
                (None, ExitReason::Succeed(_))
                | (Some(ExpectedFailure::Expected), ExitReason::Revert(_)) => {
                    output.pass(function_name);
                    stats.test_success(test_run_info(), test_plan);
                },

                (exp, reason) => {
                    unreachable!("Unexpected (exp, exit reason) pair: ({:?}, {:?}). This should not have happened.", exp, reason)
                },
            }
        }

        stats
    }

    // TODO: comparison of results via different backends

    fn exec_module_tests<F: UnitTestFactory>(
        &self,
        test_plan: &ModuleTestPlan,
        writer: &Mutex<impl Write>,
        factory: &Mutex<F>,
    ) -> TestStatistics {
        let output = TestOutput { test_plan, writer };

        #[cfg(feature = "evm-backend")]
        if self.evm {
            return self.exec_module_tests_evm(test_plan, &output);
        }

        self.exec_module_tests_move_vm_and_stackless_vm(test_plan, &output, factory)
    }
}
