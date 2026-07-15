// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Reusable "compile → load → run" engine over the mono-move pipeline.

use crate::{
    compile::{compile, SourceKind},
    module_provider::InMemoryModuleProvider,
};
use anyhow::{anyhow, bail, Result};
use mono_move_core::{
    native::{NativeExtensions, NativeName},
    types::EMPTY_TYPE_LIST,
    Function, GasMeter, Interner, NoResourceProvider,
};
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy, ModuleReadSet};
use mono_move_natives::{make_all_production_natives, make_all_test_natives, Dispatch};
use mono_move_runtime::{
    InterpreterContext, ProductionContextFamily, ProductionNativeRegistry, RuntimeStatus,
};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

/// Gas budget for engine runs. Effectively unbounded.
const GAS_BUDGET: u64 = u64::MAX;

/// Outcome of a single interpreter run.
pub enum RunResult<R> {
    /// The function returned a value of type `R`.
    Success(R),
    /// The function aborted with this code and optional message.
    Aborted { code: u64, message: Option<String> },
    /// An internal VM error.
    Error(String),
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
            Err(err) => RunResult::Error(format!("{}", err)),
            Ok(RuntimeStatus::Success) => RunResult::Success(extract_returns(&self.interp)),
            Ok(RuntimeStatus::Aborted { code, message }) => RunResult::Aborted { code, message },
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
            RunResult::Aborted { code, message } => match message {
                Some(message) => bail!("aborted: code {} ({})", code, message),
                None => bail!("aborted: code {}", code),
            },
            RunResult::Error(err) => bail!("vm error: {}", err),
        }
    }
}

/// Build the native registry mono-move executes against: the synthetic test
/// natives plus the real production natives, keyed by interned name.
pub fn build_natives(guard: &ExecutionGuard<'_>) -> ProductionNativeRegistry {
    let mut natives = ProductionNativeRegistry::new();
    natives
        .register_all(
            make_all_test_natives::<ProductionContextFamily>()
                .into_iter()
                .chain(make_all_production_natives::<ProductionContextFamily>())
                .map(|(addr, module, function, dispatch, func)| {
                    let module = guard.module_id_of(&addr, &module);
                    let function = guard.identifier_of(&function);
                    let name = match dispatch {
                        Dispatch::Polymorphic => NativeName::Polymorphic { module, function },
                        Dispatch::Monomorphic(ty_args) => NativeName::Monomorphic {
                            module,
                            function,
                            ty_args: guard.type_list_of(ty_args),
                        },
                    };
                    (name, func)
                }),
        )
        .expect("natives have unique qualified names");
    natives
}

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
    let natives = build_natives(guard);

    let loader = Loader::new_with_policy(
        guard,
        module_provider,
        LoadingPolicy::Lazy(LoweringPolicy::Lazy),
        &natives,
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
            Err(err) => return Err(anyhow!("failed to load function: {}", err)),
        };

    let interp = match heap_size {
        Some(n) => InterpreterContext::with_heap_size(
            loader,
            read_set,
            gas_meter,
            &NoResourceProvider,
            &natives,
            function,
            n,
        ),
        None => InterpreterContext::new(
            loader,
            read_set,
            gas_meter,
            &NoResourceProvider,
            &natives,
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
