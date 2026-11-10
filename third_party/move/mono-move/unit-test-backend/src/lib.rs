// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Runs Move unit tests on MonoMove for `move-unit-test`'s runner.
//!
//! Test-only natives (signer creation, key generation, event queries) are
//! registered with the `testing` feature.

mod module_provider;
mod resource_provider;

use anyhow::{ensure, Result};
use aptos_types::{chain_id::ChainId, contract_event::ContractEvent};
use bytes::Bytes;
use legacy_move_compiler::unit_test::{NamedOrBytecodeModule, TestCase, TestPlan};
pub use module_provider::{InMemoryModuleProvider, ModuleProviderError};
use mono_move_core::{
    native::NativeExtensions,
    nominal_tag,
    storage::resource_provider::InMemoryStorageKey,
    types::{is_signer_or_signer_immut_ref, EMPTY_TYPE_LIST},
    ExecutionErrorKind, Function, GasMeter, VMInternalError,
};
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use mono_move_loader::{Loader, LoaderError, LoadingPolicy, LoweringPolicy};
use mono_move_natives::{
    AlgebraStore, EventStore, ObjectContextExtension, RandomnessContext, RistrettoPointStore,
    TransactionContextExtension,
};
use mono_move_output::{
    to_contract_events,
    v1_error::{describe, v1_vm_error, MonoFailure, V1Equivalent},
    v1_return_bcs,
};
use mono_move_runtime::{
    serialize, InterpreterContext, ProductionContextFamily, ProductionNativeRegistry, RuntimeError,
    RuntimeStatus, SessionEffects, WriteClass,
};
use move_binary_format::errors::VMError;
use move_core_types::{
    account_address::AccountAddress,
    effects::{ChangeSet, Op},
    identifier::IdentStr,
    language_storage::{ModuleId, StructTag},
    value::MoveValue,
};
use move_unit_test::backend::{
    BackendConfig, TestEffects, TestEvent, TestExecution, TestRun, UnitTestBackend,
    UnitTestBackendFactory,
};
pub use resource_provider::LazyResourceProvider;
use std::{
    collections::HashMap,
    panic::{self, AssertUnwindSafe},
    ptr::NonNull,
    sync::LazyLock,
    time::Instant,
};

// Native context values matching V1's unit-test environment.
const TXN_HASH: [u8; 1] = [1];
const SCRIPT_HASH: [u8; 1] = [1];
const TABLE_HANDLE_SEED: [u8; 32] = [0; 32];
const SESSION_COUNTER: u8 = 0;

/// Builds [`MonoMoveBackend`]s.
pub struct MonoMoveBackendFactory {
    gas_budget: u64,
}

impl MonoMoveBackendFactory {
    /// `gas_budget` bounds each test, in MonoMove's gas units.
    pub fn new(gas_budget: u64) -> Self {
        Self { gas_budget }
    }
}

impl UnitTestBackendFactory for MonoMoveBackendFactory {
    fn build<'a>(
        &'a self,
        plan: &TestPlan,
        genesis: &ChangeSet,
        config: &BackendConfig,
    ) -> Result<Box<dyn UnitTestBackend + 'a>> {
        ensure!(
            !config.render_storage_dump,
            "the MonoMove backend does not render storage dumps"
        );
        let mut module_provider = InMemoryModuleProvider::new();
        for info in plan.module_info.values() {
            module_provider.add_module(match info {
                NamedOrBytecodeModule::Named(named) => &named.module,
                NamedOrBytecodeModule::Bytecode(module) => module,
            });
        }
        let genesis = genesis
            .resources()
            .filter_map(|(address, tag, op)| match op {
                Op::New(blob) | Op::Modify(blob) => Some(((address, tag.clone()), blob.clone())),
                Op::Delete => None,
            })
            .collect();
        let num_workers = config.num_threads.max(1);
        Ok(Box::new(MonoMoveBackend {
            ctx: GlobalContext::with_num_execution_workers(num_workers),
            num_workers,
            module_provider,
            genesis,
            gas_budget: self.gas_budget,
            capture_effects: config.capture_effects,
            report_stacktrace_on_abort: config.report_stacktrace_on_abort,
        }))
    }
}

/// Resource BCS bytes by address and struct tag.
type Resources = HashMap<(AccountAddress, StructTag), Bytes>;

/// Runs each test on a worker of a shared [`GlobalContext`], so lowered code
/// is shared across threads and tests.
pub struct MonoMoveBackend {
    ctx: GlobalContext,
    num_workers: usize,
    module_provider: InMemoryModuleProvider,
    genesis: Resources,
    gas_budget: u64,
    capture_effects: bool,
    report_stacktrace_on_abort: bool,
}

impl UnitTestBackend for MonoMoveBackend {
    fn name(&self) -> &str {
        "MonoMove"
    }

    fn execute(&self, module_id: &ModuleId, test: &TestCase) -> TestExecution {
        // At most `num_workers` runner threads call in at once.
        let Some(guard) =
            (0..self.num_workers).find_map(|worker_id| self.ctx.try_execution_context(worker_id))
        else {
            return backend_error("no free execution worker".to_string());
        };
        panic::catch_unwind(AssertUnwindSafe(|| self.run(&guard, module_id, test))).unwrap_or_else(
            |payload| backend_error(format!("panicked: {}", panic_message(&*payload))),
        )
    }
}

impl MonoMoveBackend {
    fn run(
        &self,
        guard: &ExecutionGuard<'_>,
        module_id: &ModuleId,
        test: &TestCase,
    ) -> TestExecution {
        let natives = &*NATIVES;
        let loader = Loader::new_with_policy(
            guard,
            &self.module_provider,
            LoadingPolicy::Lazy(LoweringPolicy::Lazy),
            natives,
        );
        let resource_provider = LazyResourceProvider::new(guard, |address, tag| {
            Ok(self.genesis.get(&(address, tag)).cloned())
        });
        let mut interpreter = InterpreterContext::new(
            loader,
            GasMeter::new(self.gas_budget),
            &resource_provider,
            natives,
        )
        .with_extensions(unit_test_extensions());

        let Ok(function_name) = IdentStr::new(&test.test_name) else {
            return backend_error(format!("invalid test name `{}`", test.test_name));
        };
        let module = guard
            .intern_address_name(module_id.address(), module_id.name())
            .into_global_arena_ptr();
        let function_name = guard
            .intern_identifier(function_name)
            .into_global_arena_ptr();

        let start = Instant::now();
        let outcome = match interpreter.load_function(module, function_name, EMPTY_TYPE_LIST) {
            Ok(function) => self.call(&mut interpreter, function, test),
            Err(err) => Outcome::Error(err),
        };
        let elapsed = start.elapsed();
        let gas_used = self.gas_budget.saturating_sub(interpreter.gas_balance());

        // Return the interpreter's buffers before classifying the outcome.
        let finished = interpreter.finish();
        let (result, v1_exact_error) = match outcome {
            Outcome::Returned(values) => (Ok(values), true),
            Outcome::Failed(error, v1_exact_error) => (Err(error), v1_exact_error),
            Outcome::Error(err) => match classify(&err) {
                Classification::Unsupported => {
                    return TestExecution::Unsupported {
                        reason: err.to_string(),
                    }
                },
                Classification::BackendError => return backend_error(err.to_string()),
                Classification::Ran => {
                    let (error, v1_exact_error) =
                        v1_vm_error(MonoFailure::Error(&err), self.report_stacktrace_on_abort);
                    (Err(error), v1_exact_error)
                },
            },
            Outcome::BackendError(message) => return backend_error(message),
        };
        let effects = match finished {
            // Effects of a failed run are never compared.
            Ok(effects) if self.capture_effects && result.is_ok() => {
                match test_effects(guard, &effects) {
                    Ok(effects) => Some(effects),
                    Err(err) => {
                        return match err.downcast_ref::<VMInternalError>().map(classify) {
                            Some(Classification::Unsupported) => TestExecution::Unsupported {
                                reason: format!("{err:#}"),
                            },
                            Some(Classification::BackendError | Classification::Ran) | None => {
                                backend_error(format!("failed to collect effects: {err:#}"))
                            },
                        }
                    },
                }
            },
            Ok(_) => None,
            Err(err) => return backend_error(format!("failed to finish the session: {err}")),
        };

        TestExecution::Ran(TestRun {
            result,
            v1_exact_error,
            effects,
            storage_dump: None,
            gas_used,
            elapsed,
        })
    }

    /// Places `test`'s arguments and calls `function`.
    fn call<'guard>(
        &self,
        interpreter: &mut InterpreterContext<'guard>,
        function: &'guard Function,
        test: &TestCase,
    ) -> Outcome {
        if test.arguments.len() != function.param_tys.len() {
            return Outcome::BackendError(format!(
                "the test has {} arguments for {} parameters",
                test.arguments.len(),
                function.param_tys.len()
            ));
        }
        let mut call = match interpreter.build_call(function) {
            Ok(call) => call,
            Err(err) => return Outcome::Error(err),
        };
        for (arg, &ty) in test.arguments.iter().zip(&function.param_tys) {
            let placed = if is_signer_or_signer_immut_ref(ty) {
                match arg {
                    MoveValue::Signer(address) | MoveValue::Address(address) => {
                        call.signer(address)
                    },
                    other => {
                        return Outcome::BackendError(format!(
                            "a signer parameter got the argument {other:?}"
                        ))
                    },
                }
            } else {
                call.arg(arg)
            };
            if let Err(err) = placed {
                return match classify(&err) {
                    Classification::Unsupported => Outcome::Error(err),
                    Classification::BackendError | Classification::Ran => Outcome::BackendError(
                        format!("failed to place the argument {arg:?}: {err}"),
                    ),
                };
            }
        }
        let call = match call.run() {
            Ok(call) => call,
            Err(err) => {
                return match err.stack_trace() {
                    Ok(stack_trace) => {
                        Outcome::Error(err.into_error().with_stack_trace(stack_trace))
                    },
                    Err(trace_err) => Outcome::Error(trace_err),
                }
            },
        };
        match call.status() {
            RuntimeStatus::Success => match call.bcs_serialize_return_values() {
                Ok(values) => match values
                    .into_iter()
                    .map(|(ty, bytes)| v1_return_bcs(ty, bytes))
                    .collect()
                {
                    Ok(values) => Outcome::Returned(values),
                    Err(err) => Outcome::BackendError(err.to_string()),
                },
                Err(err) => Outcome::Error(err),
            },
            RuntimeStatus::Aborted {
                code,
                message,
                location,
                offset,
            } => {
                let stack_trace = match call.stack_trace() {
                    Ok(stack_trace) => stack_trace,
                    Err(err) => return Outcome::Error(err),
                };
                let (error, v1_exact_error) = v1_vm_error(
                    MonoFailure::Abort {
                        code: *code,
                        message: message.clone(),
                        location: location.clone(),
                        offset: *offset,
                        stack_trace: &stack_trace,
                    },
                    self.report_stacktrace_on_abort,
                );
                Outcome::Failed(error, v1_exact_error)
            },
        }
    }
}

/// How a call ended, before classification.
enum Outcome {
    Returned(Vec<Vec<u8>>),
    /// A Move abort as V1's error, and whether its status is V1's.
    Failed(VMError, bool),
    /// A VM error, to classify.
    Error(VMInternalError),
    BackendError(String),
}

enum Classification {
    /// An explicitly unsupported MonoMove feature.
    Unsupported,
    /// A MonoMove bug, or a failure V1 would not raise.
    BackendError,
    /// A failure of the test itself.
    Ran,
}

fn classify(err: &VMInternalError) -> Classification {
    if let Some(err) = err.downcast_ref::<LoaderError>() {
        return match err {
            LoaderError::NativeFunctionNotLoadable { .. }
            | LoaderError::LoweringSkipped { .. }
            | LoaderError::ResourceLayoutNotDerivable => Classification::Unsupported,
            // Unit tests link against their whole package and load no scripts.
            LoaderError::ModuleNotFound { .. }
            | LoaderError::FunctionNotFound { .. }
            | LoaderError::GlobalContext(_)
            | LoaderError::InvariantViolation(_)
            | LoaderError::NativeShadowsMoveFunction { .. }
            | LoaderError::ScriptDeserializationFailed { .. }
            | LoaderError::ScriptVerificationFailed { .. } => Classification::BackendError,
        };
    }
    // Check `Unsupported` directly because `kind()` classifies it as an invariant violation.
    if matches!(
        err.downcast_ref::<RuntimeError>(),
        Some(RuntimeError::Unsupported(_))
    ) {
        return Classification::Unsupported;
    }
    if matches!(describe(err), V1Equivalent::NoV1Failure) {
        return Classification::BackendError;
    }
    match err.kind() {
        ExecutionErrorKind::OutOfGas
        | ExecutionErrorKind::RuntimeLimitExceeded
        | ExecutionErrorKind::InvalidOperation => Classification::Ran,
        ExecutionErrorKind::InvariantViolation
        | ExecutionErrorKind::LinkingError
        | ExecutionErrorKind::Placeholder => Classification::BackendError,
    }
}

/// Creates native extensions with the same initial values as V1's unit-test environment.
fn unit_test_extensions() -> NativeExtensions {
    let mut extensions = NativeExtensions::new();
    extensions.add(TransactionContextExtension::for_unit_test(
        &TXN_HASH,
        TABLE_HANDLE_SEED,
        SCRIPT_HASH.to_vec(),
        ChainId::test().id(),
        SESSION_COUNTER,
    ));
    extensions.add(ObjectContextExtension::new());
    extensions.add(EventStore::new());
    extensions.add(RistrettoPointStore::new());
    extensions.add(AlgebraStore::new());
    // Tests may call the randomness API freely.
    let mut randomness = RandomnessContext::new();
    randomness.mark_unbiasable();
    extensions.add(randomness);
    extensions
}

/// Production natives, plus the test-only natives with `testing`.
static NATIVES: LazyLock<ProductionNativeRegistry> = LazyLock::new(|| {
    #[cfg(feature = "testing")]
    let natives =
        mono_move_natives::make_all_test_only_and_production_natives::<ProductionContextFamily>();
    #[cfg(not(feature = "testing"))]
    let natives = mono_move_natives::make_all_production_natives::<ProductionContextFamily>();
    ProductionNativeRegistry::with_natives(natives)
});

/// The effects of a finished test. Writes are keyed per resource type, as in
/// V1's unit tests: resource groups are not packed.
fn test_effects(guard: &ExecutionGuard<'_>, effects: &SessionEffects) -> Result<TestEffects> {
    let mut test_effects = TestEffects::new(ChangeSet::new());
    for (key, class, _group) in effects.read_write_set().writes_unordered() {
        let written = |ptr: NonNull<u8>| -> Result<Bytes> {
            // SAFETY: written pointers refer to live values in the effects'
            // frozen heap, and `guard` is the guard that described them.
            Ok(unsafe { serialize(guard, ptr.as_ptr(), key.value_ty()) }?.into())
        };
        let op = match class {
            WriteClass::Creation(ptr) => Op::New(written(ptr)?),
            WriteClass::Modification(ptr) => Op::Modify(written(ptr)?),
            WriteClass::Deletion => Op::Delete,
        };
        match key {
            InMemoryStorageKey::Resource { address, ty } => {
                test_effects
                    .resources
                    .add_resource_op(*address, nominal_tag(*ty)?, op)?;
            },
            InMemoryStorageKey::TableItem { handle, key, .. } => {
                test_effects
                    .table_items
                    .entry(handle.address())
                    .or_default()
                    .insert(key.to_vec(), op);
            },
        }
    }
    for event in to_contract_events(effects, guard)? {
        let handle = match &event {
            ContractEvent::V1(event) => Some((event.key().to_bytes(), event.sequence_number())),
            ContractEvent::V2(_) => None,
        };
        test_effects.events.push(TestEvent {
            handle,
            type_tag: event.type_tag().clone(),
            data: event.event_data().to_vec(),
        });
    }
    Ok(test_effects)
}

fn backend_error(message: String) -> TestExecution {
    TestExecution::BackendError { message }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("<non-string panic payload>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use mono_move_core::GasExhaustedError;
    use mono_move_runtime::ArithOp;

    fn classified(err: impl mono_move_core::IntoExecutionError) -> &'static str {
        match classify(&VMInternalError::new(err)) {
            Classification::Unsupported => "unsupported",
            Classification::BackendError => "backend error",
            Classification::Ran => "ran",
        }
    }

    #[test]
    fn typed_gaps_are_unsupported() {
        assert_eq!(
            classified(LoaderError::NativeFunctionNotLoadable {
                address: AccountAddress::ONE,
                module: "debug".to_string(),
                name: "native_print".to_string(),
            }),
            "unsupported"
        );
        assert_eq!(
            classified(LoaderError::LoweringSkipped { reason: "gap" }),
            "unsupported"
        );
        assert_eq!(
            classified(LoaderError::ResourceLayoutNotDerivable),
            "unsupported"
        );
        assert_eq!(
            classified(RuntimeError::Unsupported("reference parameters")),
            "unsupported"
        );
    }

    /// A unit test links against its whole package, so a missing module or
    /// function is a MonoMove bug.
    #[test]
    fn linking_and_invariant_failures_are_backend_errors() {
        assert_eq!(
            classified(LoaderError::ModuleNotFound {
                address: AccountAddress::ONE,
                name: "m".to_string(),
            }),
            "backend error"
        );
        assert_eq!(
            classified(LoaderError::FunctionNotFound {
                address: AccountAddress::ONE,
                module: "m".to_string(),
                name: "f".to_string(),
            }),
            "backend error"
        );
        assert_eq!(
            classified(ModuleProviderError::InvalidModuleName {
                module_name: "?".to_string(),
            }),
            "backend error"
        );
    }

    #[test]
    fn program_failures_ran() {
        assert_eq!(
            classified(RuntimeError::DivisionByZero { op: ArithOp::Div }),
            "ran"
        );
        assert_eq!(classified(RuntimeError::StackOverflow), "ran");
        assert_eq!(classified(GasExhaustedError), "ran");
        // V1 fails here too, with a status MonoMove cannot determine.
        assert_eq!(classified(RuntimeError::BCSEof), "ran");
    }
}
