// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Criterion benchmark: replay each dumped transaction's entry function on the
//! legacy MoveVM (`v1/<version>`) and MonoMove (`v2/<version>`).
//!
//! Each VM's execution context is built once per transaction, outside the timed
//! closure, so its loader / code cache (and the provider's materialized state)
//! stay warm across samples. The timed region runs one execution over a fresh
//! write layer so a mutating transaction stays idempotent: V1 gets a fresh data
//! cache, and the V2 interpreter is built once (verification + allocation
//! happen outside the loop) and reset in place between samples. V1 loads its
//! `LoadedFunction` once and clones it per sample; the struct is `Arc`-backed,
//! so the clone is cheap, and the by-value `execute_loaded_function` API is the
//! only reason a clone is needed at all (no reload, no re-resolution).
//!
//! By default it reads the committed `data/` dir; set `APTOS_MONO_MOVE_DUMP` to
//! point at a different dump produced by `scripts/download.sh` (the
//! `<version>_txns` / `<version>_inputs` files). Set `APTOS_MONO_MOVE_LIMIT` to
//! cap the number of versions benched.

use anyhow::{anyhow, Result};
use aptos_mono_move::{
    args::ArgLayout,
    cache::FlatState,
    dump::Dump,
    extensions::{replay_extensions, HarnessView},
    resolver::resolve_type_tag,
    txn, v1, v2,
};
use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, Criterion,
};
use mono_move_core::{GasMeter, Interner};
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy};
use mono_move_runtime::{ExecutionContext, InterpreterContext, TransactionContext};
use move_vm_runtime::{
    data_cache::{MoveVmDataCacheAdapter, TransactionDataCache},
    module_traversal::{TraversalContext, TraversalStorage},
    move_vm::MoveVM,
    AsUnsyncModuleStorage, InstantiatedFunctionLoader, LazyLoader, LegacyLoaderConfig,
    WithRuntimeEnvironment,
};
use move_vm_types::gas::UnmeteredGasMeter;

const GAS_BUDGET: u64 = u64::MAX;

fn replay(c: &mut Criterion) {
    // Default to the committed data dir (`CARGO_MANIFEST_DIR` resolves to the
    // crate root at compile time); `APTOS_MONO_MOVE_DUMP` overrides it.
    let dir = std::env::var("APTOS_MONO_MOVE_DUMP")
        .unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/data").to_string());
    let dump = Dump::open(&dir).expect("open dump");
    let versions = dump.versions().expect("read versions");
    let limit = std::env::var("APTOS_MONO_MOVE_LIMIT")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(usize::MAX);

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx
        .try_execution_context(0)
        .expect("acquire execution guard");

    let mut group = c.benchmark_group("replay");
    for version in versions.into_iter().take(limit) {
        if let Err(err) = bench_version(&dump, &guard, &mut group, version) {
            eprintln!("v{version}: skip: {err}");
        }
    }
    group.finish();
}

fn bench_version(
    dump: &Dump,
    guard: &ExecutionGuard,
    group: &mut BenchmarkGroup<'_, WallTime>,
    version: u64,
) -> Result<()> {
    let Some(transaction) = dump.transaction(version)? else {
        return Ok(());
    };
    let Some(entry) = txn::entry_call(&transaction) else {
        return Ok(());
    };
    let Some(signed) = txn::signed_user_txn(&transaction) else {
        return Ok(());
    };
    let raw_state = dump.state(version)?;
    let aux_info = dump.aux_info(version)?;
    let flat = FlatState::build(&raw_state)?;

    // ---- Legacy MoveVM (V1) ----
    // Paranoid type checks off: the benchmark measures steady-state execution
    // cost, and V2 has no equivalent runtime type-check pass.
    let storage = v1::build_storage(&flat, /* paranoid_type_checks */ false)?;
    let module_storage = storage.as_unsync_module_storage();
    let loader = LazyLoader::new(&module_storage);

    // Load the function once, outside the timed region: the module-cache
    // lookup, function resolution, and type-argument instantiation all happen
    // here. `LoadedFunction` is `Arc`-backed, so cloning it per sample (forced
    // by the by-value `execute_loaded_function` API) is cheap.
    let load_ts = TraversalStorage::new();
    let mut load_tc = TraversalContext::new(&load_ts);
    let mut load_gm = UnmeteredGasMeter;
    let loaded = loader
        .load_instantiated_function(
            &LegacyLoaderConfig::unmetered(),
            &mut load_gm,
            &mut load_tc,
            entry.module,
            entry.function,
            entry.ty_args,
        )
        .map_err(|err| anyhow!("v1 load: {err}"))?;
    let layout = ArgLayout::from_param_tys(
        loaded.param_tys(),
        entry.args.len(),
        module_storage.runtime_environment().struct_name_index_map(),
    )
    .map_err(|reason| anyhow!("unsupported argument types: {reason}"))?;
    let call_args = v1::serialized_args(&entry, layout.num_signers)?;

    // The native-context state view is built once; the extensions are rebuilt
    // per sample (cheap) so each run starts from the captured state. Uses the
    // same `replay_extensions` setup as the CLI runner.
    let view = HarnessView::new(raw_state);

    group.bench_function(format!("v1/{version}"), |b| {
        b.iter(|| {
            let ts = TraversalStorage::new();
            let mut tc = TraversalContext::new(&ts);
            let mut gm = UnmeteredGasMeter;
            let mut data_cache = TransactionDataCache::empty();
            let mut extensions = replay_extensions(&view, signed, aux_info, &entry);
            let _ = MoveVM::execute_loaded_function(
                loaded.clone(),
                call_args.clone(),
                &mut MoveVmDataCacheAdapter::new(&mut data_cache, &storage, &loader),
                &mut gm,
                &mut tc,
                &mut extensions,
                &loader,
            );
        })
    });

    // ---- MonoMove (V2) ----
    let module_provider = v2::build_module_provider(&flat)?;
    let natives = v2::build_natives(guard)?;
    let provider = v2::build_provider(guard, &flat);
    let loader2 = Loader::new_with_policy(
        guard,
        &module_provider,
        LoadingPolicy::Lazy(LoweringPolicy::Lazy),
        &natives,
    );
    let mut txn_ctx =
        TransactionContext::new(loader2, GasMeter::new(GAS_BUDGET), &provider, &natives);

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
    // duration of `guard`; mirrors the testsuite engine and the V2 runner.
    let function = unsafe {
        txn_ctx
            .load_function(id, func, ty_args)
            .map_err(|err| anyhow!("v2 load: {err}"))?
            .as_ref_unchecked()
    };

    // Build the interpreter once: verification and stack/heap allocation happen
    // here, outside the timed region. Each sample resets it in place.
    let mut interp = InterpreterContext::new(&mut txn_ctx, function);
    group.bench_function(format!("v2/{version}"), |b| {
        b.iter(|| {
            interp.reset_root(function);
            let signers = vec![entry.sender; layout.num_signers];
            let args_offset = interp.set_root_signers(function, &signers);
            v2::place_args(&mut interp, guard, &entry, &layout, args_offset)
                .expect("place args");
            let _ = interp.run();
        })
    });

    Ok(())
}

criterion_group!(benches, replay);
criterion_main!(benches);
