// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Captures transactions from chain into the benchmark's on-disk dump. For each version: fetch the
//! transaction and a chain-backed state view, run it on V1 to record the read-set, then close the
//! module dependency graph so V2 has every module it needs (not just the ones V1's path loads).

use crate::data::ReadSet;
use anyhow::{anyhow, Context, Result};
use aptos_block_executor::txn_provider::default::DefaultTxnProvider;
use aptos_move_debugger::aptos_debugger::AptosDebugger;
use aptos_rest_client::{AptosBaseUrl, Client};
use aptos_types::{
    block_executor::{
        config::{BlockExecutorConfig, BlockExecutorConfigFromOnchain, BlockExecutorLocalConfig},
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    state_store::{
        state_key::{inner::StateKeyInner, StateKey},
        state_slot::{StateSlot, StateSlotKind},
        state_storage_usage::StateStorageUsage,
        state_value::StateValue,
        StateView, StateViewResult, TStateView,
    },
    transaction::{
        signature_verified_transaction::into_signature_verified_block, AuxiliaryInfo,
        PersistedAuxiliaryInfo, Transaction, TransactionBlock, Version,
    },
};
use aptos_vm::{aptos_vm::AptosVMBlockExecutor, VMBlockExecutor};
use move_binary_format::{access::ModuleAccess, CompiledModule};
use move_core_types::language_storage::ModuleId;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::Mutex,
};

/// Captures each version into `out_dir` as `<version>_txns` / `<version>_inputs`.
pub fn run(
    base_url: AptosBaseUrl,
    api_key: Option<String>,
    versions: Vec<Version>,
    out_dir: PathBuf,
) -> Result<()> {
    std::fs::create_dir_all(&out_dir)
        .with_context(|| format!("failed to create output dir {:?}", out_dir))?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime")?;
    runtime.block_on(async move {
        let debugger = build_debugger(base_url, api_key)?;
        for version in versions {
            match capture_version(&debugger, version, &out_dir).await {
                Ok(()) => println!("captured version {version}"),
                Err(err) => eprintln!("version {version}: skip: {err:#}"),
            }
        }
        Ok(())
    })
}

fn build_debugger(base_url: AptosBaseUrl, api_key: Option<String>) -> Result<AptosDebugger> {
    let mut builder = Client::builder(base_url);
    if let Some(key) = api_key {
        builder = builder.api_key(&key)?;
    }
    AptosDebugger::rest_client(builder.build())
}

async fn capture_version(debugger: &AptosDebugger, version: Version, out_dir: &Path) -> Result<()> {
    let (mut txns, _, mut aux_infos) = debugger.get_committed_transactions(version, 1).await?;
    let txn = txns
        .pop()
        .ok_or_else(|| anyhow!("no transaction at version {version}"))?;
    let aux_info = aux_infos.pop();
    let state_view = debugger.state_view_at_version(version);

    // Executing the transaction performs blocking state reads (via the debugger's REST-backed
    // state view), so it must run off the async worker threads.
    let out_dir = out_dir.to_path_buf();
    tokio::task::spawn_blocking(move || {
        capture_blocking(version, txn, aux_info, state_view, out_dir)
    })
    .await
    .context("capture task panicked")?
}

fn capture_blocking(
    version: Version,
    txn: Transaction,
    aux_info: Option<PersistedAuxiliaryInfo>,
    state_view: impl StateView + Sync,
    out_dir: PathBuf,
) -> Result<()> {
    // Write the transactions file first (one single-transaction block).
    let block = TransactionBlock {
        begin_version: version,
        transactions: vec![txn.clone()],
        persisted_auxiliary_infos: aux_info.into_iter().collect(),
    };
    let txns_bytes =
        bcs::to_bytes(&vec![block]).context("failed to serialize transaction block")?;

    // Capture the read-set by executing the transaction on V1.
    let capturing = ReadSetCapturingStateView::new(&state_view);
    execute(version, txn, aux_info, &capturing)?;
    let mut read_set = capturing.into_captured();

    // Close the module dependency graph so V2 (which needs the static closure) has every module.
    close_module_graph(&mut read_set, &state_view)?;

    let inputs_bytes =
        bcs::to_bytes(&vec![ReadSet { data: read_set }]).context("failed to serialize read-set")?;

    std::fs::write(out_dir.join(format!("{version}_txns")), &txns_bytes)?;
    std::fs::write(out_dir.join(format!("{version}_inputs")), &inputs_bytes)?;
    Ok(())
}

/// Executes the single transaction through the production block executor (the legacy Move VM) so
/// that every state read it performs is recorded by the capturing state view.
fn execute(
    version: Version,
    txn: Transaction,
    aux_info: Option<PersistedAuxiliaryInfo>,
    state_view: &(impl StateView + Sync),
) -> Result<()> {
    let signature_verified = into_signature_verified_block(vec![txn]);
    let txn_provider = match aux_info {
        Some(aux) => {
            DefaultTxnProvider::new(signature_verified, vec![AuxiliaryInfo::new(aux, None)])
        },
        None => DefaultTxnProvider::new_without_info(signature_verified),
    };
    let config = BlockExecutorConfig {
        local: BlockExecutorLocalConfig::default_with_concurrency_level(1),
        onchain: BlockExecutorConfigFromOnchain::on_but_large_for_test(),
    };
    let metadata = TransactionSliceMetadata::chunk(version, version + 1);
    AptosVMBlockExecutor::new()
        .execute_block_with_config(&txn_provider, state_view, config, metadata)
        .map_err(|err| anyhow!("block execution failed: {:?}", err))?;
    Ok(())
}

/// Walks the module dependency graph of every module already in `read_set`, pulling any missing
/// module's bytecode from `state_view`, until the closure is complete.
fn close_module_graph(
    read_set: &mut HashMap<StateKey, StateValue>,
    state_view: &impl StateView,
) -> Result<()> {
    let mut visited: HashSet<ModuleId> = HashSet::new();
    let mut queue: VecDeque<ModuleId> = VecDeque::new();
    for key in read_set.keys() {
        if let Some(module_id) = module_id_of(key) {
            if visited.insert(module_id.clone()) {
                queue.push_back(module_id);
            }
        }
    }

    while let Some(module_id) = queue.pop_front() {
        let key = StateKey::module_id(&module_id);
        let bytes = match read_set.get(&key) {
            Some(value) => value.bytes().to_vec(),
            None => {
                let Some(value) = state_view
                    .get_state_value(&key)
                    .map_err(|e| anyhow!("failed to fetch module {}: {:?}", module_id, e))?
                else {
                    // A referenced dependency that isn't on chain shouldn't happen; skip it.
                    continue;
                };
                let bytes = value.bytes().to_vec();
                read_set.insert(key, value);
                bytes
            },
        };
        let module = CompiledModule::deserialize(&bytes)
            .map_err(|e| anyhow!("failed to deserialize module {}: {:?}", module_id, e))?;
        for dep in module.immediate_dependencies() {
            if visited.insert(dep.clone()) {
                queue.push_back(dep);
            }
        }
    }
    Ok(())
}

fn module_id_of(key: &StateKey) -> Option<ModuleId> {
    match key.inner() {
        StateKeyInner::AccessPath(ap) => ap.try_get_module_id(),
        _ => None,
    }
}

/// A [`StateView`] that records every read so the set can be persisted as the dump's read-set.
/// Mirrors `aptos-replay-benchmark`'s capturing view, including preloading the framework so the
/// prologue never misses framework modules.
struct ReadSetCapturingStateView<'s, S> {
    captured: Mutex<HashMap<StateKey, StateValue>>,
    state_view: &'s S,
}

impl<'s, S: StateView> ReadSetCapturingStateView<'s, S> {
    fn new(state_view: &'s S) -> Self {
        let mut captured = HashMap::new();
        for package in &aptos_cached_packages::head_release_bundle().packages {
            for (_, module) in package.sorted_code_and_modules() {
                let key = StateKey::module(module.self_addr(), module.self_name());
                if let Ok(Some(value)) = state_view.get_state_value(&key) {
                    captured.entry(key).or_insert(value);
                }
            }
        }
        Self {
            captured: Mutex::new(captured),
            state_view,
        }
    }

    fn into_captured(self) -> HashMap<StateKey, StateValue> {
        self.captured.into_inner().unwrap()
    }
}

impl<S: StateView> TStateView for ReadSetCapturingStateView<'_, S> {
    type Key = StateKey;

    fn get_state_slot(&self, state_key: &Self::Key) -> StateViewResult<StateSlot> {
        if let Some(value) = self.captured.lock().unwrap().get(state_key) {
            return Ok(StateSlot::new(
                state_key.clone(),
                StateSlotKind::ColdOccupied {
                    value_version: 0,
                    value: value.clone(),
                },
            ));
        }
        let slot = self.state_view.get_state_slot(state_key)?;
        if let Some(value) = slot.as_state_value_opt() {
            let mut captured = self.captured.lock().unwrap();
            captured
                .entry(state_key.clone())
                .or_insert_with(|| value.clone());
        }
        Ok(slot)
    }

    fn next_version(&self) -> Version {
        0
    }

    fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
        Ok(StateStorageUsage::new_untracked())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mono_move_testsuite::compile_move_source;

    /// Offline check against the committed dump: confirms the missing modules really are in the
    /// static dependency closure of the modules already captured — i.e. the closure walk would
    /// request them from chain. Ignored by default (depends on a local `data/` dir); run with
    /// `cargo test -p mono-move-replay-benchmark --lib -- --ignored closure_requests`.
    #[test]
    #[ignore]
    fn closure_requests_the_missing_modules() {
        use crate::data::load_read_sets;
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/data");
        let cases = [
            (5663916074u64, "perp_positions"),
            (5663983784, "accounts_collateral"),
            (5781418865, "flashloan_logic"),
        ];
        for (version, target) in cases {
            let path = format!("{dir}/{version}_inputs");
            if !std::path::Path::new(&path).exists() {
                eprintln!("v{version}: no dump, skipping");
                continue;
            }
            let read_sets = load_read_sets(&path).expect("load read-set");
            let modules = read_sets[0].modules();
            let present: HashSet<ModuleId> = modules.iter().map(|(id, _)| id.clone()).collect();

            // Modules referenced as dependencies of present modules but not themselves present.
            let mut requested_missing: HashSet<ModuleId> = HashSet::new();
            for (_, bytes) in &modules {
                let module = CompiledModule::deserialize(bytes).unwrap();
                for dep in module.immediate_dependencies() {
                    if !present.contains(&dep) {
                        requested_missing.insert(dep);
                    }
                }
            }
            let found = requested_missing
                .iter()
                .any(|m| m.name().as_str() == target);
            println!(
                "v{version}: {} present modules, {} missing deps; would request `{target}` = {found}",
                present.len(),
                requested_missing.len()
            );
            assert!(
                found,
                "closure should request missing module `{target}` for v{version}"
            );
        }
    }

    #[test]
    fn closes_module_dependency_graph() {
        // `a` depends on `b`; `b` has no dependencies.
        let modules = compile_move_source(
            r#"
            module 0xc0ffee::b { public fun f(): u64 { 1 } }
            module 0xc0ffee::a { use 0xc0ffee::b; public fun g(): u64 { b::f() } }
            "#,
        )
        .expect("compile");

        let bytes = |name: &str| {
            let m = modules
                .iter()
                .find(|m| m.self_id().name().as_str() == name)
                .unwrap();
            let mut v = vec![];
            m.serialize(&mut v).unwrap();
            (m.self_id(), StateValue::new_legacy(v.into()))
        };
        let (a_id, a_val) = bytes("a");
        let (b_id, b_val) = bytes("b");

        // "Chain" has both modules; the read-set initially has only `a`.
        let chain = ReadSet {
            data: HashMap::from([
                (StateKey::module_id(&a_id), a_val.clone()),
                (StateKey::module_id(&b_id), b_val),
            ]),
        };
        let mut read_set = HashMap::from([(StateKey::module_id(&a_id), a_val)]);

        close_module_graph(&mut read_set, &chain).expect("close");

        assert!(
            read_set.contains_key(&StateKey::module_id(&b_id)),
            "closing the graph should pull in the missing dependency `b`"
        );
    }
}
