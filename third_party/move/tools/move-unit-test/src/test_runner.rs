// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    backend::{BackendConfig, Backends, TestExecution, TestRun, UnitTestBackend},
    differential::{self, Comparison},
    format_module_id,
    test_reporter::{
        CandidateMetrics, FailureReason, MoveError, TestFailure, TestResults, TestRunInfo,
        TestStatistics,
    },
    UnitTestingConfig,
};
use anyhow::Result;
use colored::*;
use legacy_move_compiler::unit_test::{ExpectedFailure, ModuleTestPlan, TestCase, TestPlan};
use move_binary_format::errors::VMError;
use move_core_types::{effects::ChangeSet, vm_status::StatusCode};
use std::{
    io::Write,
    marker::Send,
    panic::resume_unwind,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    thread,
    time::Duration,
};
use thiserror::Error;

/// Fail-fast error to short-circuit test execution
#[derive(Debug, Error)]
#[error("fail-fast triggered with {0:?}")]
struct FailFast(pub TestStatistics);

enum BuiltBackends<'a> {
    Single(Box<dyn UnitTestBackend + 'a>),
    Differential {
        reference: Box<dyn UnitTestBackend + 'a>,
        candidate: Box<dyn UnitTestBackend + 'a>,
    },
}

/// Test state common to all tests
struct SharedTestingConfig<'a> {
    backends: BuiltBackends<'a>,
    /// The state every test starts from.
    genesis: ChangeSet,
    record_writeset: bool,
    fail_fast: bool,
    /// Unsupported and incomparable tests do not fail the run, so they do not
    /// trigger fail-fast.
    allow_inconclusive: bool,
}

pub struct TestRunner<'a> {
    num_threads: usize,
    testing_config: SharedTestingConfig<'a>,
    tests: TestPlan,
}

/// The adjudicated outcome of one test.
enum Verdict {
    Pass(TestRunInfo),
    Fail(TestFailure),
    /// The backend cannot run the test, or judge its failure, yet.
    Unsupported {
        reason: String,
    },
    /// The backends ran the test, but their runs could not be compared.
    Incomparable {
        reason: String,
    },
}

impl<'a> TestRunner<'a> {
    pub fn new(
        config: &UnitTestingConfig,
        tests: TestPlan,
        backends: Backends<'a>,
        genesis: ChangeSet,
    ) -> Result<Self> {
        // Zero uses the available parallelism, or one thread if it is unknown.
        let num_threads = match config.num_threads {
            0 => thread::available_parallelism().map_or(1, |parallelism| parallelism.get()),
            num_threads => num_threads,
        };
        let backend_config = BackendConfig {
            num_threads,
            capture_effects: config.verbose,
            render_storage_dump: config.report_storage_on_error,
            report_stacktrace_on_abort: config.report_stacktrace_on_abort,
        };
        let backends = match backends {
            Backends::Single(factory) => {
                BuiltBackends::Single(factory.build(&tests, &genesis, &backend_config)?)
            },
            Backends::Differential {
                reference,
                candidate,
            } => {
                let backend_config = BackendConfig {
                    capture_effects: true,
                    ..backend_config
                };
                BuiltBackends::Differential {
                    reference: reference.build(&tests, &genesis, &backend_config)?,
                    // Failure reports use the reference's storage dump.
                    candidate: candidate.build(&tests, &genesis, &BackendConfig {
                        render_storage_dump: false,
                        ..backend_config
                    })?,
                }
            },
        };
        Ok(Self {
            testing_config: SharedTestingConfig {
                backends,
                genesis,
                record_writeset: config.verbose,
                fail_fast: config.fail_fast,
                allow_inconclusive: config.allow_inconclusive,
            },
            num_threads,
            tests,
        })
    }

    pub fn run<W: Write + Send>(self, writer: &Mutex<W>) -> Result<TestResults> {
        // Dedicated threads prevent Rayon work stealing inside natives from
        // running another module's tests on a waiting runner thread, which
        // would exceed `num_threads` concurrent test executions.
        let modules = Mutex::new(self.tests.module_tests.values());
        let stop = AtomicBool::new(false);
        let results: Vec<Result<TestStatistics>> = thread::scope(|scope| {
            let workers: Vec<_> = (0..self.num_threads)
                .map(|_| {
                    scope.spawn(|| {
                        let mut stats = TestStatistics::new();
                        while !stop.load(Ordering::Relaxed) {
                            let Some(plan) = modules
                                .lock()
                                .expect("no worker panics while taking a module")
                                .next()
                            else {
                                break;
                            };
                            match self.testing_config.exec_module_tests(plan, writer) {
                                Ok(module_stats) => stats = stats.combine(module_stats),
                                Err(err) => {
                                    stop.store(true, Ordering::Relaxed);
                                    return Err(err);
                                },
                            }
                        }
                        Ok(stats)
                    })
                })
                .collect();
            workers
                .into_iter()
                .map(|worker| worker.join().unwrap_or_else(|panic| resume_unwind(panic)))
                .collect()
        });

        let mut final_stats = TestStatistics::new();
        for result in results {
            match result {
                Ok(stats) => final_stats = final_stats.combine(stats),
                Err(err) => {
                    if !self.testing_config.fail_fast {
                        panic!(
                            "We expect no errors at all when not in fail-fast mode, but got: {:?}",
                            err
                        );
                    }
                    // The error wraps the stats up to the first failure
                    let fail_fast = err.downcast::<FailFast>().unwrap_or_else(|err| {
                        panic!("Only FailFast errors are expected in fail-fast mode, but got a different error: {:?}", err);
                    });
                    return Ok(TestResults::new(fail_fast.0, self.tests));
                },
            }
        }
        Ok(TestResults::new(final_stats, self.tests))
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

struct TestOutput<'a, 'b, W> {
    test_plan: &'a ModuleTestPlan,
    writer: &'b Mutex<W>,
}

impl<W: Write> TestOutput<'_, '_, W> {
    /// Writes `[ <status> ] <module>::<fn_name>`, padding `status` to the
    /// width of `TIMEOUT`.
    fn line(&self, status: ColoredString, fn_name: &str) {
        let padding = "TIMEOUT".len().saturating_sub(status.len());
        writeln!(
            self.writer
                .lock()
                .expect("no thread panics while holding the writer"),
            "[ {status}{:padding$} ] {}::{}",
            "",
            format_module_id(&self.test_plan.module_id),
            fn_name,
        )
        .unwrap()
    }
}

impl SharedTestingConfig<'_> {
    fn exec_module_tests(
        &self,
        test_plan: &ModuleTestPlan,
        writer: &Mutex<impl Write>,
    ) -> Result<TestStatistics> {
        let output = TestOutput { test_plan, writer };
        let mut stats = TestStatistics::new();

        for (function_name, test_info) in &test_plan.tests {
            let verdict = match &self.backends {
                BuiltBackends::Single(backend) => {
                    let execution = backend.execute(&test_plan.module_id, test_info);
                    self.record_output(&mut stats, test_plan, function_name, &execution);
                    adjudicate(function_name, test_info, execution, None)
                },
                BuiltBackends::Differential {
                    reference,
                    candidate,
                } => {
                    let reference_execution = reference.execute(&test_plan.module_id, test_info);
                    let candidate_execution = candidate.execute(&test_plan.module_id, test_info);
                    self.record_output(&mut stats, test_plan, function_name, &reference_execution);
                    self.adjudicate_differential(
                        function_name,
                        test_info,
                        (reference.name(), reference_execution),
                        (candidate.name(), candidate_execution),
                        || reference.execute(&test_plan.module_id, test_info),
                    )
                },
            };

            match verdict {
                Verdict::Pass(test_run_info) => {
                    output.line("PASS".bold().bright_green(), function_name);
                    stats.test_success(test_run_info, test_plan);
                },
                Verdict::Unsupported { reason } => {
                    output.line("UNSUPPORTED".bold().bright_yellow(), function_name);
                    stats.test_unsupported(function_name.to_string(), reason, test_plan);
                    if self.fail_fast && !self.allow_inconclusive {
                        return Err(FailFast(stats).into());
                    }
                },
                Verdict::Incomparable { reason } => {
                    output.line("INCOMPARABLE".bold().bright_yellow(), function_name);
                    stats.test_incomparable(function_name.to_string(), reason, test_plan);
                    if self.fail_fast && !self.allow_inconclusive {
                        return Err(FailFast(stats).into());
                    }
                },
                Verdict::Fail(failure) => {
                    if matches!(failure.failure_reason, FailureReason::Timeout(_)) {
                        output.line("TIMEOUT".bold().bright_yellow(), function_name);
                    } else {
                        output.line("FAIL".bold().bright_red(), function_name);
                    }
                    stats.test_failure(failure, test_plan);
                    if self.fail_fast {
                        return Err(FailFast(stats).into());
                    }
                },
            }
        }

        Ok(stats)
    }

    /// Records the resources a run wrote, for verbose goldens.
    fn record_output(
        &self,
        stats: &mut TestStatistics,
        test_plan: &ModuleTestPlan,
        function_name: &str,
        execution: &TestExecution,
    ) {
        if !self.record_writeset {
            return;
        }
        if let TestExecution::Ran(TestRun {
            effects: Some(effects),
            ..
        }) = execution
        {
            stats.test_output(
                function_name.to_string(),
                test_plan,
                format!("Ok({:?})", effects.resources),
            );
        }
    }

    /// Checks the reference against the test's expectation and compares both
    /// executions. Backend errors and mismatches fail; unsupported and
    /// incomparable outcomes stand unless the reference already fails.
    fn adjudicate_differential(
        &self,
        function_name: &str,
        test: &TestCase,
        (reference_name, reference_execution): (&str, TestExecution),
        (candidate_name, candidate_execution): (&str, TestExecution),
        rerun_reference: impl FnOnce() -> TestExecution,
    ) -> Verdict {
        let candidate_run = match candidate_execution {
            TestExecution::Ran(run) => run,
            TestExecution::Unsupported { reason } => {
                return unless_failing(
                    adjudicate(function_name, test, reference_execution, None),
                    Verdict::Unsupported { reason },
                )
            },
            TestExecution::BackendError { message } => {
                return backend_error(function_name, format!("{candidate_name}: {message}"))
            },
        };
        let reference_run = match reference_execution {
            TestExecution::Ran(run) => run,
            TestExecution::Unsupported { reason } => {
                return Verdict::Unsupported {
                    reason: format!("{reference_name}: {reason}"),
                }
            },
            TestExecution::BackendError { message } => {
                return backend_error(function_name, format!("{reference_name}: {message}"))
            },
        };

        match differential::compare(
            (reference_name, &reference_run),
            (candidate_name, &candidate_run),
            &self.genesis,
            rerun_reference,
        ) {
            Comparison::Agree => adjudicate(
                function_name,
                test,
                TestExecution::Ran(reference_run),
                Some(&candidate_run),
            ),
            Comparison::Incomparable(reason) => unless_failing(
                adjudicate(function_name, test, TestExecution::Ran(reference_run), None),
                Verdict::Incomparable { reason },
            ),
            Comparison::Mismatch(diff) => Verdict::Fail(TestFailure::new(
                FailureReason::backend_mismatch(
                    reference_name.to_string(),
                    candidate_name.to_string(),
                    diff,
                ),
                run_info(function_name, &reference_run, Some(&candidate_run)),
                None,
                reference_run.storage_dump,
            )),
        }
    }
}

/// Adjudicates one execution against the test's `#[expected_failure]`.
/// `candidate` is the run whose metrics are reported alongside it.
fn adjudicate(
    function_name: &str,
    test: &TestCase,
    execution: TestExecution,
    candidate: Option<&TestRun>,
) -> Verdict {
    let run = match execution {
        TestExecution::Ran(run) => run,
        TestExecution::Unsupported { reason } => return Verdict::Unsupported { reason },
        TestExecution::BackendError { message } => return backend_error(function_name, message),
    };
    let test_run_info = run_info(function_name, &run, candidate);
    let TestRun {
        result,
        v1_exact_error,
        storage_dump,
        ..
    } = run;
    let fail = |reason, vm_error| {
        Verdict::Fail(TestFailure::new(
            reason,
            test_run_info.clone(),
            vm_error,
            storage_dump.clone(),
        ))
    };

    let err = match result {
        Ok(_) if test.expected_failure.is_some() => {
            return fail(FailureReason::no_error(), None);
        },
        Ok(_) => return Verdict::Pass(test_run_info),
        Err(err) => err,
    };
    if err.major_status() == StatusCode::EXECUTED {
        return backend_error(
            function_name,
            "reported a failure with status EXECUTED".to_string(),
        );
    }
    let actual_err = MoveError(
        err.major_status(),
        err.sub_status(),
        err.location().clone(),
        err.message().cloned(),
    );
    match test.expected_failure.as_ref() {
        // Even a bare expectation needs an exact status: a fallback may stand
        // for a failure V1 would not raise.
        Some(
            ExpectedFailure::Expected
            | ExpectedFailure::ExpectedWithError(_)
            | ExpectedFailure::ExpectedWithCodeDEPRECATED(_),
        ) if !v1_exact_error => Verdict::Unsupported {
            reason: inexact_status_reason(&err),
        },
        Some(ExpectedFailure::Expected) => Verdict::Pass(test_run_info),
        Some(ExpectedFailure::ExpectedWithError(expected_err)) if expected_err == &actual_err => {
            Verdict::Pass(test_run_info)
        },
        Some(ExpectedFailure::ExpectedWithCodeDEPRECATED(code))
            if actual_err.0 == StatusCode::ABORTED && actual_err.1 == Some(*code) =>
        {
            Verdict::Pass(test_run_info)
        },
        Some(ExpectedFailure::ExpectedWithError(expected_err)) => fail(
            FailureReason::wrong_error(expected_err.clone(), actual_err),
            Some(err),
        ),
        Some(ExpectedFailure::ExpectedWithCodeDEPRECATED(expected_code)) => fail(
            FailureReason::wrong_abort_deprecated(*expected_code, actual_err),
            Some(err),
        ),
        // Unexpected gas exhaustion is reported as a timeout.
        None if err.major_status() == StatusCode::OUT_OF_GAS => {
            fail(FailureReason::timeout(), Some(err))
        },
        None => fail(FailureReason::unexpected_error(actual_err), Some(err)),
    }
}

/// Why a failure whose status is not V1's cannot be judged against an
/// expected error.
pub(crate) fn inexact_status_reason(err: &VMError) -> String {
    match err.message() {
        Some(message) => format!("status not V1-exact: {:?}: {message}", err.major_status()),
        None => format!("status not V1-exact: {:?}", err.major_status()),
    }
}

fn run_info(function_name: &str, run: &TestRun, candidate: Option<&TestRun>) -> TestRunInfo {
    let mut test_run_info = TestRunInfo::new(function_name.to_string(), run.elapsed);
    test_run_info.gas_used = run.gas_used;
    test_run_info.candidate = candidate.map(CandidateMetrics::of);
    test_run_info
}

fn backend_error(function_name: &str, message: String) -> Verdict {
    Verdict::Fail(TestFailure::new(
        FailureReason::backend_error(message),
        TestRunInfo::new(function_name.to_string(), Duration::ZERO),
        None,
        None,
    ))
}

/// A failing reference verdict stands. Otherwise `verdict` replaces it, since
/// a candidate run that was not checked never counts as agreement.
fn unless_failing(reference: Verdict, verdict: Verdict) -> Verdict {
    match reference {
        reference @ Verdict::Fail(_) => reference,
        Verdict::Pass(_) | Verdict::Unsupported { .. } | Verdict::Incomparable { .. } => verdict,
    }
}
