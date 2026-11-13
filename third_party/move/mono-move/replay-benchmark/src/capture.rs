// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Captures transactions from chain into the benchmark's on-disk dump. For each version: fetch the
//! transaction and a chain-backed state view, run it on V1 to record the read-set, then close the
//! module dependency graph so V2 has every module it needs (not just the ones V1's path loads).

use anyhow::{anyhow, Context, Result};
use aptos_move_debugger::aptos_debugger::AptosDebugger;
use aptos_rest_client::AptosBaseUrl;
use aptos_types::{
    state_store::StateView,
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, AuxiliaryInfo,
        PersistedAuxiliaryInfo, Transaction, TransactionBlock, Version,
    },
};
use aptos_vm::{data_cache::AsMoveResolver, AptosVM};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::module_and_script_storage::AsAptosCodeStorage;
use mono_move_replay_common::{
    capture::ReadSetCapturingStateView,
    cli::rest_client,
    modules::{close_module_graph, head_framework_keys},
};
use std::{
    collections::{BTreeSet, HashMap},
    path::{Path, PathBuf},
};

/// Captures each version into `out_dir` as `<version>_txns` / `<version>_inputs`.
pub fn run(
    base_url: AptosBaseUrl,
    api_key: Option<String>,
    versions: Vec<Version>,
    out_dir: PathBuf,
) -> Result<()> {
    aptos_logger::Logger::new().init();
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
    AptosDebugger::rest_client(rest_client(base_url, api_key)?)
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
    let capturing = capturing_view(&state_view);
    execute(txn, aux_info, &capturing)?;
    let (mut read_set, _) = capturing.into_captured()?;

    // Close the module dependency graph so V2 (which needs the static closure) has every module.
    close_module_graph(
        &mut read_set,
        |_| false,
        |key| {
            state_view
                .get_state_value(key)
                .map_err(|e| anyhow!("{:?}", e))
        },
    )?;

    let inputs_bytes = bcs::to_bytes(&vec![read_set]).context("failed to serialize read-set")?;

    std::fs::write(out_dir.join(format!("{version}_txns")), &txns_bytes)?;
    std::fs::write(out_dir.join(format!("{version}_inputs")), &inputs_bytes)?;
    Ok(())
}

/// Executes the single transaction through the same V1 path the benchmark replays it on (see
/// `v1.rs`), so the capturing state view records exactly the reads a replay performs — including
/// the environment's on-chain config reads (features, gas schedule).
fn execute(
    txn: Transaction,
    aux_info: Option<PersistedAuxiliaryInfo>,
    state_view: &(impl StateView + Sync),
) -> Result<()> {
    let env = AptosEnvironment::new(state_view);
    let vm = AptosVM::new(&env);
    let resolver = state_view.as_move_resolver();
    let code_storage = state_view.as_aptos_code_storage(&env);
    let log_context = AdapterLogSchema::new(state_view.id(), 0);
    let aux_info = AuxiliaryInfo::new(aux_info.unwrap_or(PersistedAuxiliaryInfo::None), None);
    match txn {
        Transaction::UserTransaction(txn) => {
            vm.execute_user_transaction(&resolver, &code_storage, &txn, &log_context, &aux_info);
        },
        txn => {
            let txn = SignatureVerifiedTransaction::Valid(txn);
            vm.execute_single_transaction(&txn, &resolver, &code_storage, &log_context, &aux_info)
                .map_err(|status| anyhow!("V1 rejected the transaction: {:?}", status))?;
        },
    }
    Ok(())
}

/// A capturing view preloaded with every module of the current framework found in `state_view`,
/// so the prologue never misses a framework module.
fn capturing_view<S: StateView>(state_view: &S) -> ReadSetCapturingStateView<'_, S> {
    let mut preloaded = HashMap::new();
    let mut failures = vec![];
    for key in head_framework_keys() {
        // A module the current framework has but the captured state does not is fine; only
        // failed fetches make the dump unreliable.
        match state_view.get_state_value(key) {
            Ok(Some(value)) => {
                preloaded.insert(key.clone(), value);
            },
            Ok(None) => {},
            Err(err) => failures.push(format!("preload of {key:?} failed: {err}")),
        }
    }
    ReadSetCapturingStateView::new(state_view, preloaded, BTreeSet::new(), failures)
}

#[cfg(test)]
mod tests {
    use mono_move_replay_common::modules::module_id_of;
    use move_binary_format::{access::ModuleAccess, CompiledModule};
    use move_core_types::language_storage::ModuleId;
    use std::collections::HashSet;

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
            let modules: Vec<(ModuleId, Vec<u8>)> = read_sets[0]
                .iter()
                .filter_map(|(key, value)| module_id_of(key).map(|id| (id, value.bytes().to_vec())))
                .collect();
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
}
