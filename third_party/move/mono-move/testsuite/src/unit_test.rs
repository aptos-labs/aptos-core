// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Runs real Move `#[test]` unit tests on mono-move as a coverage scoreboard: a
//! test it can't yet run is recorded as *unsupported*; only a test it runs but
//! gets wrong is a failure.

use crate::{
    engine::build_natives, extensions::seed_extensions, module_provider::InMemoryModuleProvider,
    resource_provider::InMemoryResourceProvider,
};
use aptos_types::on_chain_config::{Features, OnChainConfig};
use legacy_move_compiler::unit_test::{
    ExpectedFailure, ExpectedMoveError, NamedOrBytecodeModule, TestCase,
};
use mono_move_core::{
    types::EMPTY_TYPE_LIST, DescriptorProvider, Function, GasMeter, Interner, LayoutProvider,
};
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use mono_move_loader::{Loader, LoaderError, LoadingPolicy, LoweringPolicy};
use mono_move_runtime::{
    ExecutionContext, InterpreterContext, ProductionNativeRegistry, RuntimeError, RuntimeStatus,
    TransactionContext,
};
use move_binary_format::CompiledModule;
use move_core_types::{
    identifier::IdentStr, language_storage::ModuleId, value::MoveValue, vm_status::StatusCode,
};
use move_model::metadata::LanguageVersion;
use move_package::BuildConfig;
use move_unit_test::UnitTestingConfig;
use std::{
    collections::BTreeMap,
    fmt::Write,
    panic::{self, AssertUnwindSafe},
    path::Path,
};

/// Effectively unbounded gas budget for running a test.
const GAS_BUDGET: u64 = u64::MAX;

pub fn run_package_unit_tests(
    pkg_path: &Path,
    use_latest_language: bool,
) -> anyhow::Result<RunSummary> {
    let mut build_config = BuildConfig::default();
    if use_latest_language {
        let language_version = LanguageVersion::latest();
        let bytecode_version = language_version.infer_bytecode_version(None);
        build_config.compiler_config.language_version = Some(language_version);
        build_config.compiler_config.bytecode_version = Some(bytecode_version);
    }

    let test_plan = move_unit_test::package_test::build_test_plan_for_package(
        pkg_path,
        build_config,
        &mut UnitTestingConfig::default(),
        &mut std::io::sink(),
    )?;

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).expect("worker 0 is free");
    let natives = build_natives(&guard);

    let mut module_provider = InMemoryModuleProvider::new();
    for info in test_plan.module_info.values() {
        module_provider.add_module(module_of(info));
    }

    let resource_provider = seed_features(&guard);

    let mut summary = RunSummary::default();
    for (module_id, module_plan) in &test_plan.module_tests {
        for (test_name, test) in &module_plan.tests {
            let outcome = run_test(
                &guard,
                &module_provider,
                &resource_provider,
                &natives,
                module_id,
                test,
            );
            summary.record(module_id, test_name, outcome);
        }
    }
    Ok(summary)
}

fn module_of(info: &NamedOrBytecodeModule) -> &CompiledModule {
    match info {
        NamedOrBytecodeModule::Named(named) => &named.module,
        NamedOrBytecodeModule::Bytecode(module) => module,
    }
}

/// Heap for the seeded `Features` resource. Only that one small resource lives
/// here, so a modest fixed size is plenty.
const RESOURCE_HEAP_SIZE: usize = 1 << 20;

/// Builds a resource provider publishing the framework `Features` resource at
/// `0x1`, initialized to [`Features::default_for_tests`].
fn seed_features<'guard, 'ctx>(
    guard: &'guard ExecutionGuard<'ctx>,
) -> InMemoryResourceProvider<'guard, 'ctx> {
    let struct_tag = Features::struct_tag();
    let module_id = guard.module_id_of(&struct_tag.address, struct_tag.module.as_ident_str());
    let name = guard.identifier_of(struct_tag.name.as_ident_str());
    let ty = guard.nominal_of(module_id, name, guard.type_list_of(&[]));
    let bytes = bcs::to_bytes(&Features::default_for_tests()).expect("Features serializes");

    let mut provider = InMemoryResourceProvider::new(guard, RESOURCE_HEAP_SIZE);
    provider.add_resource(struct_tag.address, ty, bytes);
    provider
}

/// Execute one test function on mono-move and adjudicate it against its
/// `#[expected_failure]` annotation. A panic on an unimplemented construct is
/// caught and recorded as unsupported.
fn run_test(
    guard: &ExecutionGuard<'_>,
    module_provider: &InMemoryModuleProvider,
    resource_provider: &InMemoryResourceProvider<'_, '_>,
    natives: &ProductionNativeRegistry,
    module_id: &ModuleId,
    test: &TestCase,
) -> TestOutcome {
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        execute(
            guard,
            natives,
            module_provider,
            resource_provider,
            module_id,
            test,
        )
    }));
    match result {
        Ok(result) => adjudicate(result, &test.expected_failure),
        Err(payload) => TestOutcome::Unsupported(panic_message(payload)),
    }
}

fn execute(
    guard: &ExecutionGuard<'_>,
    natives: &ProductionNativeRegistry,
    module_provider: &InMemoryModuleProvider,
    resource_provider: &InMemoryResourceProvider<'_, '_>,
    module_id: &ModuleId,
    test: &TestCase,
) -> TestResult {
    let loader = Loader::new_with_policy(
        guard,
        module_provider,
        LoadingPolicy::Lazy(LoweringPolicy::Lazy),
        natives,
    );
    let mut txn_ctx = TransactionContext::new(
        loader,
        GasMeter::new(GAS_BUDGET),
        resource_provider,
        natives,
    )
    .with_extensions(seed_extensions());

    let module_id = guard
        .intern_address_name(module_id.address(), module_id.name())
        .into_global_arena_ptr();
    let func = guard
        .intern_identifier(IdentStr::new(&test.test_name).unwrap())
        .into_global_arena_ptr();

    // SAFETY: the pointer lives in a `LoadedModule`'s arena; while `guard` is
    // held the executable cache cannot reset that arena.
    let function = match txn_ctx.load_function(module_id, func, EMPTY_TYPE_LIST) {
        Ok(ptr) => unsafe { ptr.as_ref_unchecked() },
        Err(err) => return classify_loader_error(&err),
    };

    let mut interpreter = InterpreterContext::new(&mut txn_ctx, function);

    // Reference arguments point into this storage, so it must outlive `run()`.
    let _ref_args = marshal_args(&mut interpreter, function, &test.arguments);

    match interpreter.run() {
        Ok(RuntimeStatus::Success) => TestResult::Success,
        Ok(RuntimeStatus::Aborted { code, message }) => TestResult::Abort { code, message },
        Err(err) => classify_runtime_error(&err),
    }
}

// ---------------------------------------------------------------------------
// Argument marshalling
// ---------------------------------------------------------------------------

/// A reference is a 16-byte fat pointer (8-byte base + 8-byte offset).
/// TODO(cleanup): this reference size should be a shared constant, not redefined here.
const REFERENCE_SIZE: u32 = 16;

/// Write each argument into the root frame at its parameter-slot offset,
/// returning backing storage that reference arguments point into; the caller
/// must keep it alive until execution finishes.
// Each target is boxed for a stable heap address: the fat pointer stores that
// address, so a `Vec<[u8; 32]>` (whose elements move on reallocation) won't do.
#[allow(clippy::vec_box)]
fn marshal_args<T>(
    interpreter: &mut InterpreterContext<'_, T>,
    function: &Function,
    args: &[MoveValue],
) -> Vec<Box<[u8; 32]>>
where
    T: ExecutionContext + DescriptorProvider + LayoutProvider,
{
    assert_eq!(
        args.len(),
        function.param_slots.len(),
        "test argument count does not match the function's parameter count"
    );

    let mut ref_args = Vec::new();
    for (arg, slot) in args.iter().zip(&function.param_slots) {
        let bytes = match arg {
            // A `&signer`/`&address` is a fat pointer to a target kept alive past `run()`.
            MoveValue::Signer(addr) | MoveValue::Address(addr) if slot.size == REFERENCE_SIZE => {
                let boxed_bytes = Box::new(addr.into_bytes());
                let mut fat = vec![0u8; REFERENCE_SIZE as usize];
                fat[..8].copy_from_slice(&(boxed_bytes.as_ptr().addr() as u64).to_ne_bytes());
                ref_args.push(boxed_bytes);
                fat
            },
            _ => inline_bytes(arg),
        };
        assert_eq!(
            bytes.len() as u32,
            slot.size,
            "argument size does not match its parameter slot"
        );
        interpreter.set_root_arg(slot.offset.0, &bytes);
    }
    ref_args
}

fn inline_bytes(value: &MoveValue) -> Vec<u8> {
    match value {
        MoveValue::Bool(b) => vec![*b as u8],
        MoveValue::U8(x) => vec![*x],
        MoveValue::U16(x) => x.to_le_bytes().to_vec(),
        MoveValue::U32(x) => x.to_le_bytes().to_vec(),
        MoveValue::U64(x) => x.to_le_bytes().to_vec(),
        MoveValue::U128(x) => x.to_le_bytes().to_vec(),
        MoveValue::U256(x) => x.to_le_bytes().to_vec(),
        MoveValue::I8(x) => vec![*x as u8],
        MoveValue::I16(x) => x.to_le_bytes().to_vec(),
        MoveValue::I32(x) => x.to_le_bytes().to_vec(),
        MoveValue::I64(x) => x.to_le_bytes().to_vec(),
        MoveValue::I128(x) => x.to_le_bytes().to_vec(),
        MoveValue::I256(x) => x.to_le_bytes().to_vec(),
        MoveValue::Address(a) | MoveValue::Signer(a) => a.into_bytes().to_vec(),
        MoveValue::Vector(_) | MoveValue::Struct(_) | MoveValue::Closure(_) => {
            panic!("test argument cannot be a heap value")
        },
    }
}

// ---------------------------------------------------------------------------
// Outcome classification and adjudication
// ---------------------------------------------------------------------------

/// What happened when mono-move executed one test function.
enum TestResult {
    /// Ran to completion.
    Success,
    /// An explicit Move `abort`, with its code and optional message.
    Abort { code: u64, message: Option<String> },
    /// An implicit runtime failure: arithmetic overflow, index out of bounds, etc.
    RuntimeFailure(String),
    /// Cannot build or run this yet: missing native, unlowered construct, etc.
    Unsupported(String),
    /// A VM or harness fault.
    Error(String),
}

/// The verdict for one test after comparing its [`TestResult`]
/// against its expected failure (if the test has one).
pub enum TestOutcome {
    Pass,
    Fail(String),
    Unsupported(String),
}

fn classify_loader_error(err: &LoaderError) -> TestResult {
    match err {
        // mono-move can't build or resolve this yet: a native it doesn't
        // implement (which surfaces as a missing function, since natives have
        // no body to lower), or a feature the specializer/verifier can't lower.
        LoaderError::FunctionIrMissing
        | LoaderError::LoweringSkipped { .. }
        | LoaderError::Specializer(_)
        | LoaderError::Deserialization(_)
        | LoaderError::Verification(_)
        | LoaderError::ModuleNotFound { .. }
        | LoaderError::FunctionNotFound { .. } => TestResult::Unsupported(err.to_string()),
        // Genuine problems: a runaway against the (effectively unbounded)
        // budget, storage/context infrastructure errors, or a VM bug.
        LoaderError::GasExhausted(_)
        | LoaderError::ModuleProvider(_)
        | LoaderError::GlobalContext(_)
        | LoaderError::InvariantViolation(_) => TestResult::Error(err.to_string()),
    }
}

fn classify_runtime_error(err: &RuntimeError) -> TestResult {
    match err {
        // A loader error surfaced during lazy dispatch keeps its precise meaning.
        RuntimeError::Loader(inner) => classify_loader_error(inner),

        // The program failed at runtime: overflow, OOB, missing resource, a hit
        // limit, ... mono-move matches the existing VM's behaviour and limits,
        // so these are real failures a `#[expected_failure]` test may want.
        RuntimeError::ArithmeticOverflow { .. }
        | RuntimeError::ArithmeticUnderflow { .. }
        | RuntimeError::DivisionByZero { .. }
        | RuntimeError::ShiftAmountOutOfRange { .. }
        | RuntimeError::ArithmeticUnderOverflow { .. }
        | RuntimeError::DivisionByZeroOrOverflow { .. }
        | RuntimeError::NegateMinOverflow { .. }
        | RuntimeError::CastOutOfRange { .. }
        | RuntimeError::PopFromEmptyVector
        | RuntimeError::VecUnpackLengthMismatch { .. }
        | RuntimeError::VectorIndexOutOfBounds { .. }
        | RuntimeError::ResourceDoesNotExist { .. }
        | RuntimeError::ResourceAlreadyExists { .. }
        | RuntimeError::EnumVariantMismatch { .. }
        | RuntimeError::InvalidAbortMessage
        | RuntimeError::BCSEof
        | RuntimeError::BCSInvalidUleb
        | RuntimeError::BCSInvalidBool { .. }
        | RuntimeError::BCSSequenceTooLong { .. }
        | RuntimeError::BCSRemainingInput { .. }
        | RuntimeError::StackOverflow
        | RuntimeError::OutOfHeapMemory { .. }
        | RuntimeError::AllocationTooLarge { .. }
        | RuntimeError::VecAllocSizeOverflow
        | RuntimeError::AbortMessageTooLong { .. } => TestResult::RuntimeFailure(err.to_string()),

        // Genuine problems: runaway gas, infrastructure failure, or a VM bug.
        RuntimeError::GasExhausted(_)
        | RuntimeError::InvariantViolation(_)
        | RuntimeError::ResourceProvider(_) => TestResult::Error(err.to_string()),
    }
}

fn adjudicate(result: TestResult, expected: &Option<ExpectedFailure>) -> TestOutcome {
    // Outcomes that don't depend on the expectation; otherwise normalize the
    // failure to (abort code, detail) — an explicit abort carries a code, an
    // implicit runtime failure carries none.
    let (code, detail) = match result {
        TestResult::Success => {
            return match expected {
                None => TestOutcome::Pass,
                Some(_) => {
                    TestOutcome::Fail("expected this test to fail, but it succeeded".to_string())
                },
            };
        },
        TestResult::Abort { code, message } => (Some(code), message),
        TestResult::RuntimeFailure(failure) => (None, Some(failure)),
        TestResult::Unsupported(reason) => return TestOutcome::Unsupported(reason),
        TestResult::Error(error) => return TestOutcome::Fail(error),
    };

    // The program failed; judge it against the expectation, matching on the
    // abort code only.
    //
    // TODO(completeness): a non-abort expected error (e.g. `arithmetic_error`) is currently
    // satisfied by any failure; match it against mono-move's own error
    // categories instead. Fine for now: move-stdlib and aptos-framework only
    // use abort-code (or bare) expected failures.
    let reason = match expected {
        // No failure was expected.
        None => match code {
            Some(code) => format!("unexpected abort with code {code}"),
            None => "unexpected runtime failure".to_string(),
        },
        // A failure was expected; if it requires a specific abort code, check it.
        Some(expected) => match expected_abort_code(expected) {
            None => return TestOutcome::Pass,
            Some(want) if code == Some(want) => return TestOutcome::Pass,
            Some(want) => match code {
                Some(got) => format!("expected abort with code {want}, got code {got}"),
                None => format!("expected abort with code {want}, got a runtime failure"),
            },
        },
    };

    // On a fail, fold in the abort message / runtime failure for diagnostics.
    let detailed_reason = match detail {
        Some(detail) => format!("{reason} ({detail})"),
        None => reason,
    };

    TestOutcome::Fail(detailed_reason)
}

fn expected_abort_code(expected: &ExpectedFailure) -> Option<u64> {
    match expected {
        ExpectedFailure::Expected => None,
        ExpectedFailure::ExpectedWithCodeDEPRECATED(code) => Some(*code),
        ExpectedFailure::ExpectedWithError(ExpectedMoveError(status, sub, ..)) => {
            if *status == StatusCode::ABORTED {
                *sub
            } else {
                None
            }
        },
    }
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("panic")
        .to_string()
}

// ---------------------------------------------------------------------------
// Scoreboard
// ---------------------------------------------------------------------------

/// Aggregated results of running a [`TestPlan`] on mono-move.
#[derive(Default)]
pub struct RunSummary {
    pub passed: usize,
    /// `"<module>::<test>: <reason>"` for each failing test.
    pub failed: Vec<String>,
    pub unsupported_total: usize,
    /// Reason -> count, so the most common gaps are obvious.
    pub unsupported_reasons: BTreeMap<String, usize>,
    pub per_module: BTreeMap<ModuleId, ModuleStats>,
}

#[derive(Default)]
pub struct ModuleStats {
    pub passed: usize,
    pub failed: usize,
    pub unsupported: usize,
}

impl RunSummary {
    fn record(&mut self, module_id: &ModuleId, test_name: &str, outcome: TestOutcome) {
        let stats = self.per_module.entry(module_id.clone()).or_default();
        match outcome {
            TestOutcome::Pass => {
                self.passed += 1;
                stats.passed += 1;
            },
            TestOutcome::Fail(reason) => {
                stats.failed += 1;
                self.failed
                    .push(format!("{module_id}::{test_name}: {reason}"));
            },
            TestOutcome::Unsupported(reason) => {
                self.unsupported_total += 1;
                stats.unsupported += 1;
                *self.unsupported_reasons.entry(reason).or_insert(0) += 1;
            },
        }
    }

    pub fn total(&self) -> usize {
        self.passed + self.failed.len() + self.unsupported_total
    }

    /// Render the scoreboard as a human-readable report.
    pub fn render(&self) -> String {
        let mut out = String::new();
        writeln!(out, "mono-move unit-test scoreboard").unwrap();
        writeln!(
            out,
            "  {} passed, {} failed, {} unsupported (of {} total)",
            self.passed,
            self.failed.len(),
            self.unsupported_total,
            self.total()
        )
        .unwrap();

        writeln!(out, "\nper module:").unwrap();
        for (module_id, stats) in &self.per_module {
            writeln!(
                out,
                "  {:<44} pass {:>4}  fail {:>4}  unsupported {:>4}",
                module_id.to_string(),
                stats.passed,
                stats.failed,
                stats.unsupported
            )
            .unwrap();
        }

        if !self.failed.is_empty() {
            writeln!(out, "\nfailures:").unwrap();
            for failure in &self.failed {
                writeln!(out, "  {failure}").unwrap();
            }
        }

        if !self.unsupported_reasons.is_empty() {
            writeln!(out, "\nunsupported (reason histogram):").unwrap();
            let mut reasons: Vec<_> = self.unsupported_reasons.iter().collect();
            reasons.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            for (reason, count) in reasons {
                writeln!(out, "  {count:>4}  {reason}").unwrap();
            }
        }

        out
    }
}
