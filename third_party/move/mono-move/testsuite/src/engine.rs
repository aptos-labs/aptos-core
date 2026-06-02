// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Reusable "compile → load → run" engine over the mono-move pipeline.

use crate::{
    compile::{compile, SourceKind},
    module_provider::InMemoryModuleProvider,
};
use anyhow::{anyhow, bail, Result};
use mono_move_core::{
    native::NativeName, types::EMPTY_TYPE_LIST, Function, Interner, NO_RESOURCE_PROVIDER,
};
use mono_move_gas::SimpleGasMeter;
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy};
use mono_move_natives::{make_all_production_natives, make_all_test_natives};
use mono_move_runtime::{
    ExecutionContext, InterpreterContext, ProductionContextFamily, ProductionNativeRegistry,
    RuntimeStatus, TransactionContext,
};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

/// Gas budget for engine runs. Effectively unbounded.
const GAS_BUDGET: u64 = u64::MAX;

/// The concrete per-transaction context the engine runs against.
type TxnCtx<'guard, 'ctx> = TransactionContext<'guard, 'ctx, SimpleGasMeter>;
/// The interpreter context handed to `set`/`read` closures in [`MonoRunner::run`].
type Interp<'i, 'guard, 'ctx> = InterpreterContext<'i, TxnCtx<'guard, 'ctx>>;

/// Outcome of a single interpreter run.
pub enum RunResult<R> {
    /// The function returned a value of type `R`.
    Success(R),
    /// The function aborted with this code and optional message.
    Aborted { code: u64, message: Option<String> },
    /// An internal VM error.
    Error(String),
}

/// A loaded entry function bound to a live [`TransactionContext`], ready to be
/// run one or more times. Each [`run`](Self::run) builds a fresh
/// [`InterpreterContext`] over the shared transaction context.
pub struct MonoRunner<'a, 'guard, 'ctx> {
    txn_ctx: &'a mut TxnCtx<'guard, 'ctx>,
    function: &'guard Function,
    /// Initial heap size for each run, or `None` for the interpreter default.
    /// A small size makes GC-pressure tests trigger collections.
    heap_size: Option<usize>,
    /// Number of garbage collections the most recent [`run`](Self::run)
    /// performed.
    gc_count: usize,
}

impl<'guard, 'ctx> MonoRunner<'_, 'guard, 'ctx> {
    /// Sets the initial heap size used to build the interpreter on each run.
    pub fn set_heap_size(&mut self, heap_size: Option<usize>) {
        self.heap_size = heap_size;
    }

    /// Number of garbage collections the most recent [`run`](Self::run) ran.
    pub fn gc_count(&self) -> usize {
        self.gc_count
    }

    /// Run the entry function once. `set_args` places arguments into the root
    /// frame before execution; on success `extract_returns` reads results from
    /// it.
    pub fn run<R>(
        &mut self,
        set_args: impl FnOnce(&mut Interp<'_, 'guard, 'ctx>),
        extract_returns: impl FnOnce(&Interp<'_, 'guard, 'ctx>) -> R,
    ) -> RunResult<R> {
        // Each run starts with a full budget; the meter is shared across
        // repeated runs on this context (e.g. bench iterations).
        self.txn_ctx.gas_meter().reset(GAS_BUDGET);
        let mut interp = match self.heap_size {
            Some(n) => InterpreterContext::with_heap_size(&mut *self.txn_ctx, self.function, n),
            None => InterpreterContext::new(&mut *self.txn_ctx, self.function),
        };
        set_args(&mut interp);
        let result = match interp.run() {
            Err(err) => RunResult::Error(format!("{}", err)),
            Ok(RuntimeStatus::Success) => RunResult::Success(extract_returns(&interp)),
            Ok(RuntimeStatus::Aborted { code, message }) => RunResult::Aborted { code, message },
        };
        self.gc_count = interp.gc_count();
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

/// Build the loader/native/transaction stack over an existing guard and module
/// provider, load `address::module_name::function_name`, and hand a
/// [`MonoRunner`] to `body`.
pub fn with_mono_function<'guard, 'ctx, R>(
    guard: &'guard ExecutionGuard<'ctx>,
    module_provider: &'guard InMemoryModuleProvider,
    address: AccountAddress,
    module_name: &IdentStr,
    function_name: &IdentStr,
    body: impl FnOnce(&mut MonoRunner<'_, '_, 'ctx>) -> R,
) -> Result<R> {
    let mut natives = ProductionNativeRegistry::<SimpleGasMeter>::new();
    natives
        .register_all(
            make_all_test_natives::<ProductionContextFamily<SimpleGasMeter>>()
                .into_iter()
                .chain(make_all_production_natives::<
                    ProductionContextFamily<SimpleGasMeter>,
                >())
                .map(|(addr, module, function, func)| {
                    let name = NativeName {
                        module: guard.module_id_of(&addr, &module),
                        function: guard.identifier_of(&function),
                    };
                    (name, func)
                }),
        )
        .expect("natives have unique qualified names");

    let loader = Loader::new_with_policy(
        guard,
        module_provider,
        LoadingPolicy::Lazy(LoweringPolicy::Lazy),
        &natives,
    );
    let mut txn_ctx = TransactionContext::new(
        loader,
        SimpleGasMeter::new(GAS_BUDGET),
        &NO_RESOURCE_PROVIDER,
        &natives,
    );

    let id = guard
        .intern_address_name(&address, module_name)
        .into_global_arena_ptr();
    let func = guard
        .intern_identifier(function_name)
        .into_global_arena_ptr();

    // SAFETY: the pointer lives in a `LoadedModule`'s arena. While `guard` is
    // held the global executable cache cannot enter maintenance, so no arena
    // reset can happen for the duration of `body`.
    let function = match txn_ctx.load_function(id, func, EMPTY_TYPE_LIST) {
        Ok(ptr) => unsafe { ptr.as_ref_unchecked() },
        Err(err) => return Err(anyhow!("failed to load function: {}", err)),
    };

    let mut runner = MonoRunner {
        txn_ctx: &mut txn_ctx,
        function,
        heap_size: None,
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
    body: impl FnOnce(&mut MonoRunner<'_, '_, '_>) -> R,
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
        body,
    )
}
