// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Reusable "compile → load → run" engine over the mono-move pipeline.

use crate::{
    compile::{compile, SourceKind},
    module_provider::InMemoryModuleProvider,
};
use anyhow::{anyhow, bail, Error, Result};
use mono_move_core::{
    native::{NativeExtensions, NoNatives},
    types::EMPTY_TYPE_LIST,
    Function, GasMeter, NoResourceProvider, VMInternalError,
};
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy, ModuleReadSet};
use mono_move_natives::{
    make_all_bls12381_test_natives, make_all_bulletproofs_test_natives,
    make_all_ed25519_test_natives, make_all_multi_ed25519_test_natives,
    make_all_production_natives, make_all_ristretto255_scalar_test_natives, make_all_test_natives,
    make_all_unit_test_natives,
};
use mono_move_runtime::{
    InterpreterContext, ProductionContextFamily, ProductionNativeRegistry, RuntimeStatus,
};
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, vm_status::AbortLocation,
};
use specializer::ModuleIR;
use std::sync::LazyLock;

/// Gas budget for engine runs. Effectively unbounded.
const GAS_BUDGET: u64 = u64::MAX;

/// Outcome of a single interpreter run.
pub enum RunResult<R> {
    /// The function returned a value of type `R`.
    Success(R),
    /// The function aborted with this code and optional message.
    Aborted {
        code: u64,
        message: Option<String>,
        location: AbortLocation,
    },
    /// An internal VM error.
    Error(VMInternalError),
}

/// A loaded entry function bound to a live [`InterpreterContext`], ready to be
/// run one or more times. Each [`run`](Self::run) resets the context to a
/// clean state, reusing its stack and heap buffers.
pub struct MonoRunner<'guard> {
    interp: InterpreterContext<'guard>,
    function: &'guard Function,
    /// Number of garbage collections the most recent [`run`](Self::run)
    /// performed.
    gc_count: usize,
}

impl<'guard> MonoRunner<'guard> {
    /// Number of garbage collections the most recent [`run`](Self::run) ran.
    pub fn gc_count(&self) -> usize {
        self.gc_count
    }

    /// Run the entry function once. `set_args` places arguments into the root
    /// frame before execution; on success `extract_returns` reads results from
    /// it.
    pub fn run<R>(
        &mut self,
        set_args: impl FnOnce(&mut InterpreterContext<'guard>),
        extract_returns: impl FnOnce(&InterpreterContext<'guard>) -> R,
    ) -> RunResult<R> {
        // Each run starts from a clean state with a full budget, reusing the
        // already-allocated stack and heap buffers.
        self.interp.reset(self.function, GAS_BUDGET);
        set_args(&mut self.interp);
        let result = match self.interp.run() {
            Err(err) => RunResult::Error(err),
            Ok(RuntimeStatus::Success) => RunResult::Success(extract_returns(&self.interp)),
            Ok(RuntimeStatus::Aborted {
                code,
                message,
                location,
            }) => RunResult::Aborted {
                code,
                message,
                location,
            },
        };
        self.gc_count = self.interp.gc_count();
        result
    }

    /// Call an entry whose args are 8-byte words and that returns a single
    /// 8-byte word. Each arg is written at a consecutive 8-byte offset; the
    /// lone result is read from offset 0 as a raw `u64`. Callers reinterpret
    /// those bits (e.g. `as i64`) when the entry's return type is signed.
    pub fn call_words(&mut self, args: &[u64]) -> Result<u64> {
        match self.run(
            |interp| {
                for (index, value) in args.iter().enumerate() {
                    interp.set_root_arg((index * 8) as u32, &value.to_le_bytes());
                }
            },
            |interp| interp.root_result(),
        ) {
            RunResult::Success(value) => Ok(value),
            RunResult::Aborted { code, message, .. } => match message {
                Some(message) => bail!("aborted: code {} ({})", code, message),
                None => bail!("aborted: code {}", code),
            },
            RunResult::Error(err) => bail!("vm error: {}", err),
        }
    }
}

/// Build the native registry mono-move executes against: the synthetic test
/// natives plus the real production natives.
pub fn build_natives() -> &'static ProductionNativeRegistry {
    &ALL_NATIVES
}

static ALL_NATIVES: LazyLock<ProductionNativeRegistry> = LazyLock::new(|| {
    let mut natives = make_all_test_natives::<ProductionContextFamily>();
    natives.extend(make_all_unit_test_natives::<ProductionContextFamily>());
    natives.extend(make_all_ed25519_test_natives::<ProductionContextFamily>());
    natives.extend(make_all_multi_ed25519_test_natives::<ProductionContextFamily>());
    natives.extend(make_all_bls12381_test_natives::<ProductionContextFamily>());
    natives.extend(make_all_ristretto255_scalar_test_natives::<
        ProductionContextFamily,
    >());
    natives.extend(make_all_bulletproofs_test_natives::<ProductionContextFamily>());
    natives.extend(make_all_production_natives::<ProductionContextFamily>());
    ProductionNativeRegistry::with_natives(natives)
});

/// Build the loader/native/interpreter stack over an existing guard and module
/// provider, install `extensions`, load `address::module_name::function_name`,
/// and hand a [`MonoRunner`] to `body`. `heap_size` sizes the interpreter heap
/// (`None` for the default); a small size makes GC-pressure tests trigger
/// collections.
pub fn with_mono_function<'guard, 'ctx, R>(
    guard: &'guard ExecutionGuard<'ctx>,
    module_provider: &'guard InMemoryModuleProvider,
    address: AccountAddress,
    module_name: &IdentStr,
    function_name: &IdentStr,
    extensions: NativeExtensions,
    heap_size: Option<usize>,
    body: impl FnOnce(&mut MonoRunner<'_>) -> R,
) -> Result<R> {
    let natives = build_natives();

    let loader = Loader::new_with_policy(
        guard,
        module_provider,
        LoadingPolicy::Lazy(LoweringPolicy::Lazy),
        natives,
    );

    let id = guard
        .intern_address_name(&address, module_name)
        .into_global_arena_ptr();
    let func = guard
        .intern_identifier(function_name)
        .into_global_arena_ptr();

    let mut read_set = ModuleReadSet::new();
    let mut gas_meter = GasMeter::new(GAS_BUDGET);
    // SAFETY: the pointer lives in a `LoadedModule`'s arena. While `guard` is
    // held the global executable cache cannot enter maintenance, so no arena
    // reset can happen for the duration of `body`.
    let function =
        match loader.load_function(&mut read_set, &mut gas_meter, id, func, EMPTY_TYPE_LIST) {
            Ok(ptr) => unsafe { ptr.as_ref_unchecked() },
            // Attached as the source rather than formatted in, so callers can
            // recover the typed error.
            Err(err) => return Err(Error::new(err).context("failed to load function")),
        };

    let interp = match heap_size {
        Some(n) => InterpreterContext::with_heap_size(
            loader,
            read_set,
            gas_meter,
            &NoResourceProvider,
            natives,
            function,
            n,
        ),
        None => InterpreterContext::new(
            loader,
            read_set,
            gas_meter,
            &NoResourceProvider,
            natives,
            function,
        ),
    }
    .with_extensions(extensions);

    let mut runner = MonoRunner {
        interp,
        function,
        gc_count: 0,
    };
    Ok(body(&mut runner))
}

/// Compile `source`, load `0x1::module_name`, and hand its IR to `body`
/// alongside the live execution guard.
///
/// `R` must not contain an `InternedType`: those are arena pointers that
/// dangle once the guard is dropped.
pub fn with_loaded_module<R>(
    source: &str,
    module_name: &IdentStr,
    body: impl FnOnce(&ExecutionGuard, &ModuleIR) -> R,
) -> Result<R> {
    let modules = compile(source, SourceKind::Move)?;
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx
        .try_execution_context(0)
        .ok_or_else(|| anyhow!("failed to acquire execution guard 0"))?;
    let mut module_provider = InMemoryModuleProvider::new();
    module_provider.add_modules(&modules);

    let loader = Loader::new_with_policy(
        &guard,
        &module_provider,
        LoadingPolicy::Lazy(LoweringPolicy::Eager),
        &NoNatives,
    );
    let mut read_set = ModuleReadSet::new();
    let mut gas_meter = GasMeter::with_max_budget();
    let id = guard.intern_address_name(&AccountAddress::ONE, module_name);
    let module_ir = loader.load_module(&mut read_set, &mut gas_meter, id)?.ir();
    Ok(body(&guard, module_ir))
}

/// Compile/assemble `source`, build a fresh [`GlobalContext`] + module
/// provider, then load `address::module_name::function_name` and hand a
/// [`MonoRunner`] to `body`.
pub fn with_loaded_mono_function<R>(
    source: &str,
    kind: SourceKind,
    address: AccountAddress,
    module_name: &IdentStr,
    function_name: &IdentStr,
    body: impl FnOnce(&mut MonoRunner<'_>) -> R,
) -> Result<R> {
    let modules = compile(source, kind)?;
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx
        .try_execution_context(0)
        .ok_or_else(|| anyhow!("failed to acquire execution guard 0"))?;
    let mut module_provider = InMemoryModuleProvider::new();
    module_provider.add_modules(&modules);
    with_mono_function(
        &guard,
        &module_provider,
        address,
        module_name,
        function_name,
        NativeExtensions::new(),
        None,
        body,
    )
}
