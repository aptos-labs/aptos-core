// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Runs an entry function on the legacy MoveVM and extracts its writes.
//!
//! Seeds an in-memory storage with the flattened resources and modules, loads
//! the function, prepends a sender signer per leading `&signer` parameter, runs
//! it unmetered through `MoveVM::execute_loaded_function`, and converts the
//! data cache into a per-resource change set. Mirrors the differential
//! testsuite's V1 path (`mono-move-testsuite/src/runner.rs`).

use crate::{
    args::ArgLayout,
    cache::FlatState,
    extensions::{replay_extensions, HarnessView},
    txn::EntryCall,
};
use anyhow::{anyhow, Result};
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_types::{
    on_chain_config::{Features, TimedFeaturesBuilder},
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{PersistedAuxiliaryInfo, SignedTransaction},
};
use aptos_vm::natives::aptos_natives;
use bytes::Bytes;
use move_core_types::{
    account_address::AccountAddress,
    effects::{AccountChanges, ChangeSet, Op},
    language_storage::StructTag,
    value::MoveValue,
};
use move_vm_runtime::{
    data_cache::{MoveVmDataCacheAdapter, TransactionDataCache},
    module_traversal::{TraversalContext, TraversalStorage},
    move_vm::MoveVM,
    AsUnsyncModuleStorage, InstantiatedFunctionLoader, LazyLoader, LegacyLoaderConfig,
    RuntimeEnvironment, WithRuntimeEnvironment,
};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::gas::UnmeteredGasMeter;
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

/// Result of running the entry function on the legacy MoveVM.
pub struct V1Outcome {
    /// Wall-clock time of the `execute_loaded_function` call.
    pub elapsed: Duration,
    /// True if execution aborted / errored (no write set produced).
    pub aborted: bool,
    /// The VM status when it aborted/errored (Move abort code + location, or
    /// the error status code), `None` on success.
    pub abort_reason: Option<String>,
    /// Per-resource write ops keyed by publishing address and type.
    pub writes: BTreeMap<(AccountAddress, StructTag), Op<Bytes>>,
    /// Frame layout for the V2 runner. `Err(reason)` names the first argument
    /// type that is not yet placeable in a MonoMove frame (V1 itself runs
    /// regardless, since the legacy VM deserializes the raw BCS args against the
    /// parameter types).
    pub layout: Result<ArgLayout, String>,
}

/// Builds an in-memory storage seeded from `flat`, runs `entry`, and returns
/// its writes. The layout is reported as unsupported (error) if any parameter
/// is not a scalar/address/signer.
/// Builds an in-memory storage seeded with the flattened modules and resources
/// (framework + user), using a zeroed-gas, all-features Aptos native table.
pub fn build_storage(flat: &FlatState) -> Result<InMemoryStorage> {
    let native_table = aptos_natives(
        LATEST_GAS_FEATURE_VERSION,
        NativeGasParameters::zeros(),
        MiscGasParameters::zeros(),
        TimedFeaturesBuilder::enable_all().build(),
        Features::default(),
    );
    let runtime_env = RuntimeEnvironment::new(native_table);
    let mut storage = InMemoryStorage::new_with_runtime_environment(runtime_env);
    for ((addr, name), bytes) in &flat.modules {
        storage.add_module_bytes(addr, name, bytes.clone());
    }
    seed_resources(&mut storage, flat)?;
    Ok(storage)
}

/// Builds the serialized argument vector: one sender signer per leading
/// `&signer`, then the BCS arguments from the transaction (passed through
/// verbatim — the legacy VM deserializes them against the parameter types, so
/// vector/struct/`Object` args need no special handling here).
pub fn serialized_args(entry: &EntryCall, num_signers: usize) -> Result<Vec<Vec<u8>>> {
    let signer_arg = MoveValue::Signer(entry.sender)
        .simple_serialize()
        .ok_or_else(|| anyhow!("failed to serialize signer"))?;
    let mut args = vec![signer_arg; num_signers];
    args.extend(entry.args.iter().cloned());
    Ok(args)
}

pub fn run(
    flat: &FlatState,
    raw_state: &BTreeMap<StateKey, StateValue>,
    signed: &SignedTransaction,
    aux_info: PersistedAuxiliaryInfo,
    entry: &EntryCall,
) -> Result<V1Outcome> {
    let storage = build_storage(flat)?;

    let mut gas_meter = UnmeteredGasMeter;
    let traversal_storage = TraversalStorage::new();
    let mut traversal_context = TraversalContext::new(&traversal_storage);
    let module_storage = storage.as_unsync_module_storage();
    let loader = LazyLoader::new(&module_storage);

    let function = loader
        .load_instantiated_function(
            &LegacyLoaderConfig::unmetered(),
            &mut gas_meter,
            &mut traversal_context,
            entry.module,
            entry.function,
            entry.ty_args,
        )
        .map_err(|err| anyhow!("failed to load function: {err}"))?;

    // The number of leading `&signer` parameters: every parameter without a
    // transaction argument. (Move requires signer parameters to come first.)
    let num_signers = function
        .param_tys()
        .len()
        .checked_sub(entry.args.len())
        .ok_or_else(|| anyhow!("transaction has more arguments than parameters"))?;
    // Best-effort frame layout for the V2 runner; `Err` names the first arg
    // that isn't yet placeable there. V1 runs regardless.
    let layout = ArgLayout::from_param_tys(
        function.param_tys(),
        entry.args.len(),
        module_storage.runtime_environment().struct_name_index_map(),
    );

    let call_args = serialized_args(entry, num_signers)?;

    // Native context extensions over the raw captured state (groups + table
    // items intact), with a session id and user-transaction context derived
    // from the transaction.
    let view = HarnessView::new(raw_state.clone());
    let mut extensions = replay_extensions(&view, signed, aux_info, entry);

    let mut data_cache = TransactionDataCache::empty();

    let start = Instant::now();
    let result = MoveVM::execute_loaded_function(
        function,
        call_args,
        &mut MoveVmDataCacheAdapter::new(&mut data_cache, &storage, &loader),
        &mut gas_meter,
        &mut traversal_context,
        &mut extensions,
        &loader,
    );
    let elapsed = start.elapsed();

    if let Err(err) = result {
        let status = err.into_vm_status();
        return Ok(V1Outcome {
            elapsed,
            aborted: true,
            abort_reason: Some(format!("{status:?}")),
            writes: BTreeMap::new(),
            layout,
        });
    }

    let change_set = data_cache
        .into_effects(&module_storage)
        .map_err(|err| anyhow!("into_effects failed: {err:?}"))?;
    let mut writes = BTreeMap::new();
    for (addr, account_changes) in change_set.into_inner() {
        for (tag, op) in account_changes.into_resources() {
            writes.insert((addr, tag), op);
        }
    }

    Ok(V1Outcome {
        elapsed,
        aborted: false,
        abort_reason: None,
        writes,
        layout,
    })
}

/// Inserts the flattened resources into `storage` as a `New` change set.
fn seed_resources(storage: &mut InMemoryStorage, flat: &FlatState) -> Result<()> {
    let mut by_addr: BTreeMap<AccountAddress, BTreeMap<StructTag, Op<Bytes>>> = BTreeMap::new();
    for ((addr, tag), bytes) in &flat.resources {
        by_addr
            .entry(*addr)
            .or_default()
            .insert(tag.clone(), Op::New(bytes.clone()));
    }
    let mut change_set = ChangeSet::new();
    for (addr, resources) in by_addr {
        change_set
            .add_account_changeset(addr, AccountChanges::from_resources(resources))
            .map_err(|err| anyhow!("building seed change set: {err:?}"))?;
    }
    storage
        .apply(change_set)
        .map_err(|err| anyhow!("seeding resources: {err:?}"))?;
    Ok(())
}
