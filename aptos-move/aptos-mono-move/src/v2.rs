// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Runs an entry function on MonoMove against a materializing provider, and
//! reads back the writes it made for a given set of resource keys.
//!
//! Mirrors the testsuite engine's loader/native/transaction setup
//! (`mono-move-testsuite/src/engine.rs`) but injects a
//! [`MaterializingResourceProvider`] (instead of the no-op provider) and passes
//! the transaction's resolved type arguments to `load_function`.

use crate::{
    args::{ArgKind, ArgLayout},
    cache::FlatState,
    resolver::{resolve_struct_tag, resolve_type_tag},
    txn::EntryCall,
};
use anyhow::{anyhow, Result};
use mono_move_core::{
    align_up_u32, native::NativeName, storage::resource_provider::InMemoryStorageKey, GasMeter,
    Interner,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy};
use mono_move_natives::{make_all_production_natives, make_all_test_natives, Dispatch};
use mono_move_runtime::{
    ExecutionContext, InterpreterContext, MaterializingResourceProvider, ProductionContextFamily,
    ProductionNativeRegistry, ResourceWrite, RuntimeStatus, TransactionContext,
};
use mono_move_testsuite::InMemoryModuleProvider;
use move_binary_format::CompiledModule;
use move_core_types::{account_address::AccountAddress, language_storage::StructTag};
use std::{
    collections::{BTreeMap, HashMap},
    time::{Duration, Instant},
};

const GAS_BUDGET: u64 = u64::MAX;

/// Result of running the entry function on MonoMove.
pub struct V2Outcome {
    /// Wall-clock time of `InterpreterContext::run`.
    pub elapsed: Duration,
    /// True if execution aborted / errored.
    pub aborted: bool,
    /// The Move abort (code + message) or the VM error when it aborted/errored,
    /// `None` on success.
    pub abort_reason: Option<String>,
    /// MonoMove's write for each queried key (`None` = untouched / only read).
    pub writes: BTreeMap<(AccountAddress, StructTag), Option<ResourceWrite>>,
}

/// Builds a module provider from the flattened module bytecode.
pub fn build_module_provider(flat: &FlatState) -> Result<InMemoryModuleProvider> {
    let mut module_provider = InMemoryModuleProvider::new();
    for bytes in flat.modules.values() {
        let module = CompiledModule::deserialize(bytes)
            .map_err(|err| anyhow!("deserializing module: {err:?}"))?;
        module_provider.add_module(&module);
    }
    Ok(module_provider)
}

/// Builds the native registry (test + production natives) keyed by interned
/// name.
pub fn build_natives(guard: &ExecutionGuard) -> Result<ProductionNativeRegistry> {
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
        .map_err(|_| anyhow!("natives have duplicate qualified names"))?;
    Ok(natives)
}

/// Builds a materializing provider over every resolvable flat resource.
/// Resources whose type cannot be resolved (e.g. function tags) are dropped.
pub fn build_provider<'g, 'ctx>(
    guard: &'g ExecutionGuard<'ctx>,
    flat: &FlatState,
) -> MaterializingResourceProvider<'g, 'ctx> {
    let mut bcs_by_key = HashMap::new();
    for ((addr, tag), bytes) in &flat.resources {
        if let Ok(ty) = resolve_struct_tag(guard, tag) {
            bcs_by_key.insert(InMemoryStorageKey::resource(*addr, ty), bytes.to_vec());
        }
    }
    MaterializingResourceProvider::new(guard, bcs_by_key)
}

/// Runs `entry` on MonoMove and queries the writes for `query_keys`.
pub fn run(
    guard: &ExecutionGuard,
    flat: &FlatState,
    entry: &EntryCall,
    layout: &ArgLayout,
    query_keys: &[(AccountAddress, StructTag)],
) -> Result<V2Outcome> {
    let module_provider = build_module_provider(flat)?;
    let natives = build_natives(guard)?;
    let provider = build_provider(guard, flat);

    let loader = Loader::new_with_policy(
        guard,
        &module_provider,
        LoadingPolicy::Lazy(LoweringPolicy::Lazy),
        &natives,
    );
    let mut txn_ctx =
        TransactionContext::new(loader, GasMeter::new(GAS_BUDGET), &provider, &natives);

    // Resolve type arguments and load the function.
    let resolved = entry
        .ty_args
        .iter()
        .map(|t| resolve_type_tag(guard, t))
        .collect::<Result<Vec<_>>>()?;
    let ty_args = guard.type_list_of(&resolved);
    let id = guard
        .intern_address_name(entry.module.address(), entry.module.name())
        .into_global_arena_ptr();
    let func = guard
        .intern_identifier(entry.function)
        .into_global_arena_ptr();
    // SAFETY: the pointer lives in a loaded module's arena, kept alive for the
    // duration of `guard`; mirrors the testsuite engine.
    let function = unsafe {
        txn_ctx
            .load_function(id, func, ty_args)
            .map_err(|err| anyhow!("failed to load function: {err}"))?
            .as_ref_unchecked()
    };

    let mut interp = InterpreterContext::new(&mut txn_ctx, function);
    // The interpreter sets up the leading `&signer` references (pointing into
    // its own stable storage) and tells us where the real arguments begin.
    let signers = vec![entry.sender; layout.num_signers];
    let args_offset = interp.set_root_signers(function, &signers);
    place_args(&mut interp, guard, entry, layout, args_offset)?;

    let start = Instant::now();
    let status = interp.run();
    let elapsed = start.elapsed();

    let (aborted, abort_reason) = match status {
        Ok(RuntimeStatus::Success) => (false, None),
        Ok(RuntimeStatus::Aborted { code, message }) => {
            let reason = match message {
                Some(message) => format!("ABORTED code {code}: {message}"),
                None => format!("ABORTED code {code}"),
            };
            (true, Some(reason))
        },
        Err(err) => (true, Some(err.to_string())),
    };
    let mut writes = BTreeMap::new();
    if !aborted {
        for (addr, tag) in query_keys {
            let ty = resolve_struct_tag(guard, tag)?;
            let key = InMemoryStorageKey::resource(*addr, ty);
            let write = interp
                .resource_write(&key)
                .map_err(|err| anyhow!("reading MonoMove write: {err}"))?;
            writes.insert((*addr, tag.clone()), write);
        }
    }

    Ok(V2Outcome {
        elapsed,
        aborted,
        abort_reason,
        writes,
    })
}

/// Places the non-signer BCS arguments into the root frame, starting at
/// `start_offset` (the end of the signer parameter region, as returned by
/// `InterpreterContext::set_root_signers`). Scalars are copied verbatim;
/// heap-boxed arguments (`vector`/`String`) are deserialized into the heap.
pub fn place_args<T>(
    interp: &mut InterpreterContext<'_, T>,
    guard: &ExecutionGuard,
    entry: &EntryCall,
    layout: &ArgLayout,
    start_offset: u32,
) -> Result<()>
where
    T: mono_move_runtime::ExecutionContext
        + mono_move_core::DescriptorProvider
        + mono_move_core::LayoutProvider,
{
    let mut offset = start_offset;
    for (arg_idx, kind) in layout.kinds.iter().enumerate() {
        offset = align_up_u32(offset, kind.align());
        let bytes = &entry.args[arg_idx];
        match kind {
            ArgKind::Scalar(_) => interp.set_root_arg(offset, bytes),
            ArgKind::Structured(tag) => {
                let ty = resolve_type_tag(guard, tag)?;
                interp
                    .deserialize_arg(offset, ty, bytes)
                    .map_err(|err| anyhow!("placing arg #{arg_idx}: {err}"))?;
            },
        }
        offset += kind.size();
    }
    Ok(())
}
