// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Captures transactions from chain into a corpus. For each version: fetch the transaction and a
//! chain-backed state view, run it on V1 to record the read-set, then close the module dependency
//! graph so V2 has every module it needs (not just the ones V1's path loads). The on-chain status
//! and auxiliary info come from the committed transactions, fetched in pages.

use crate::{
    completion::{self, Completion},
    corpus::{CorpusWriter, Manifest, Origin, RecordInput},
    import::{
        fetch_committed, Cache, CommittedTxn, Frameworks, RestChainSource, FRAMEWORK_ADDRESSES,
    },
    isolated::{self, V2Limits},
    overrides::OverrideConfig,
    txn::{is_replayed, txn_kind},
    v1::{self, hit_metering_limit_on_chain},
};
use anyhow::{anyhow, bail, Context, Result};
use aptos_move_debugger::aptos_debugger::AptosDebugger;
use aptos_rest_client::AptosBaseUrl;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue, StateView},
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, AuxiliaryInfo,
        PersistedAuxiliaryInfo, Transaction, Version,
    },
};
use futures::{stream, StreamExt};
use mono_move_replay_common::{
    capture::ReadSetCapturingStateView,
    cli::rest_client,
    modules::{close_module_graph, head_framework_keys},
};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::PathBuf,
    sync::Arc,
};

pub struct CaptureConfig {
    pub base_url: AptosBaseUrl,
    pub api_key: Option<String>,
    pub versions: Vec<Version>,
    pub out: PathBuf,
    pub corpus_id: String,
    pub network: String,
    pub shard_size: usize,
    /// Transactions captured at once.
    pub concurrency: usize,
    /// Whether to complete the state by replaying both VMs.
    pub complete: bool,
    /// The limits each MonoMove replay during completion runs under.
    pub limits: V2Limits,
}

#[derive(Debug, Default, Serialize)]
pub struct CaptureReport {
    pub captured: usize,
    /// Versions left out, by reason.
    pub skipped: BTreeMap<String, usize>,
    /// Keys the replays read beyond the capture run, added as values and as absences.
    pub completed_values: usize,
    pub completed_absent: usize,
    /// Records whose MonoMove replay crashed during completion (see [`crate::isolated`]): kept,
    /// with only V1's reads completed, so the comparison reports the crash.
    pub v2_crashes: usize,
}

/// Captures `config.versions` into a new corpus.
pub fn run(config: CaptureConfig) -> Result<(Manifest, CaptureReport)> {
    aptos_logger::Logger::new().init();
    let mut report = CaptureReport::default();
    let mut skip = |reason: String| *report.skipped.entry(reason).or_default() += 1;

    let mut versions = config.versions.clone();
    versions.sort_unstable();
    versions.dedup();
    let client = rest_client(config.base_url, config.api_key)?;
    let chain = RestChainSource::from_client(client.clone(), config.concurrency)?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime")?;
    let debugger = Arc::new(AptosDebugger::rest_client(client)?);
    let mut writer = CorpusWriter::create(
        &config.out,
        &config.corpus_id,
        &config.network,
        Origin::Capture,
        config.shard_size,
    )?;
    // Each release is fetched and added as a framework set once, rather than module by module for
    // every version.
    let mut frameworks = Frameworks::default();
    let complete = config.complete;
    let limits = config.limits;
    // A shard at a time, so that memory holds one shard's transactions however long the run.
    for chunk in versions.chunks(config.shard_size.max(1)) {
        let mut committed = or_stop!(
            writer,
            fetch_committed(&chain, &Cache::none(), chunk),
            "fetching the committed transactions"
        );
        // The shard's framework releases, resolved together: a shard rarely spans an upgrade, so
        // this usually reads the registries twice; each release is fetched once per run.
        let state_versions: Vec<Version> = chunk
            .iter()
            .filter_map(|version| committed.get(version))
            .filter_map(|committed| capture_state_version(committed).ok())
            .collect();
        let releases = frameworks.releases(&chain, &Cache::none(), &state_versions);
        let mut releases: HashMap<Version, _> = state_versions.into_iter().zip(releases).collect();
        let mut jobs = vec![];
        for version in chunk {
            let Some(committed) = committed.remove(version) else {
                skip("not on chain".to_string());
                continue;
            };
            let state_version = match capture_state_version(&committed) {
                Ok(state_version) => state_version,
                Err(reason) => {
                    skip(reason);
                    continue;
                },
            };
            let (release, modules) = match releases.remove(&state_version) {
                Some(Ok(Some(found))) => found,
                Some(Ok(None)) | None => {
                    skip("no 0x1 package registry".to_string());
                    continue;
                },
                // A version whose release cannot be looked up (its state pruned, say) is skipped
                // like any other that cannot be captured.
                Some(Err(err)) => {
                    eprintln!("version {}: skip: {:#}", version, err);
                    skip("framework release lookup failed".to_string());
                    continue;
                },
            };
            jobs.push((committed, release, modules));
        }
        // Capturing performs blocking state reads through the REST-backed state view, so each
        // runs off the async worker threads.
        let mut captured: Vec<_> = runtime.block_on(
            stream::iter(jobs)
                .map(|(committed, release, modules)| {
                    let debugger = debugger.clone();
                    async move {
                        let state_view = debugger.state_view_at_version(committed.version);
                        let txn = committed.txn.clone();
                        let aux_info = committed.aux_info;
                        let seed = modules.clone();
                        let read_set = tokio::task::spawn_blocking(move || {
                            capture_read_set(txn, aux_info, state_view, &seed, complete, &limits)
                        })
                        .await
                        .map_err(|e| anyhow!("capture task panicked: {e}"))
                        .and_then(|r| r);
                        (committed, release, modules, read_set)
                    }
                })
                .buffer_unordered(config.concurrency.max(1))
                .collect(),
        );
        captured.sort_by_key(|(committed, ..)| committed.version);
        for (committed, release, modules, read_set) in captured {
            let (mut state, absent, completion) = match read_set {
                Ok(captured) => captured,
                // A broken harness would fail every record alike, so it stops the capture.
                Err(err) if isolated::is_harness_error(&err) => {
                    return Err(writer
                        .finish_after(err, &format!("capturing version {}", committed.version)));
                },
                Err(err) => {
                    eprintln!("version {}: skip: {err:#}", committed.version);
                    skip(v1::MeteringStop::skip_reason(&err, "capture failed").to_string());
                    continue;
                },
            };
            // The capture was seeded with the release, so its framework modules are the release's,
            // which the record refers to as a set. A framework module read from chain besides them
            // would be lost, so such a record is skipped.
            if state
                .keys()
                .any(|key| key.is_aptos_code() && !modules.contains_key(key))
            {
                eprintln!(
                    "version {}: skip: read a framework module its release lacks",
                    committed.version
                );
                skip("framework module outside its release".to_string());
                continue;
            }
            state.retain(|key, _| !key.is_aptos_code());
            report.completed_values += completion.values;
            report.completed_absent += completion.absent;
            report.v2_crashes += usize::from(completion.v2_crashed);
            let version = committed.version;
            let framework = or_stop!(
                writer,
                frameworks.add(&mut writer, release, &modules),
                format!("capturing version {}", version)
            );
            or_stop!(
                writer,
                writer.add(RecordInput {
                    version,
                    txn: committed.txn,
                    aux_info: committed.aux_info,
                    state,
                    framework: Some(framework),
                    absent,
                    onchain: Some(committed.onchain),
                }),
                format!("capturing version {}", version)
            );
            report.captured += 1;
        }
        println!("captured up to version {:?}", chunk.last());
    }
    let manifest = writer.finish()?;
    Ok((manifest, report))
}

/// The state version `committed` is captured at, or why it is not captured: a kind that is not
/// replayed; a stop at a metering limit on chain (replayed gas-free, a loop that gas bounded could
/// run for ever, and it would not be comparable anyway); or genesis.
fn capture_state_version(committed: &CommittedTxn) -> std::result::Result<Version, String> {
    if !is_replayed(&committed.txn) {
        return Err(format!("{} not replayed", txn_kind(&committed.txn)));
    }
    if hit_metering_limit_on_chain(&committed.onchain.status) {
        return Err("hit a metering limit on chain".to_string());
    }
    committed
        .version
        .checked_sub(1)
        .ok_or_else(|| "genesis".to_string())
}

/// Every value the transaction reads on V1 plus the closure of the modules among them, and the
/// keys it read and found absent. A key a replay reads that is in neither was never observed, and
/// the comparison reports the record as incomplete rather than trusting a guess.
fn capture_read_set(
    txn: Transaction,
    aux_info: PersistedAuxiliaryInfo,
    state_view: impl StateView + Sync,
    framework: &BTreeMap<StateKey, StateValue>,
    complete: bool,
    limits: &V2Limits,
) -> Result<(
    BTreeMap<StateKey, StateValue>,
    BTreeSet<StateKey>,
    Completion,
)> {
    // Run on the same V1 path a replay takes, so the capture records exactly the reads a replay
    // performs, including the environment's on-chain config reads (features, gas schedule).
    let capturing = capturing_view(&state_view, framework);
    let verified = SignatureVerifiedTransaction::Valid(txn.clone());
    let run = v1::execute(&capturing, &verified, &AuxiliaryInfo::new(aux_info, None))?;
    // As for a stop on chain: a metering limit its status on chain does not show (see `V1Run`).
    if run.hit_metering_limit {
        return Err(v1::MeteringStop.into());
    }
    let (read_set, mut absent) = capturing.into_captured()?;
    let mut read_set: BTreeMap<_, _> = read_set.into_iter().collect();

    let fetch = |key: &StateKey| {
        state_view
            .get_state_value(key)
            .map_err(|e| anyhow!("{:?}", e))
    };
    // Close the module dependency graph so V2 (which needs the static closure) has every module.
    // The framework was seeded whole, with its closure: only the other modules are walked.
    let closure = close_module_graph(
        &mut read_set,
        |id| FRAMEWORK_ADDRESSES.contains(id.address()),
        fetch,
    )?;
    // A published module's dependencies are on chain, so a missing one means the fetch failed.
    if !closure.missing.is_empty() {
        bail!("incomplete module closure, missing {:?}", closure.missing);
    }

    if !complete {
        return Ok((read_set, absent, Completion::default()));
    }
    // Add what the patched replay of either VM reads beyond V1's capture run.
    let completion = completion::complete(
        &txn,
        aux_info,
        &BTreeMap::new(),
        &mut read_set,
        &mut absent,
        &OverrideConfig::for_origin(&Origin::Capture),
        limits,
        fetch,
    )?;
    if completion.incomplete {
        bail!("state still incomplete after completing it");
    }
    Ok((read_set, absent, completion))
}

/// A capturing view preloaded with `framework`, every module at the framework accounts at this
/// version, so the prologue never misses a framework module.
fn capturing_view<'s, S: StateView>(
    state_view: &'s S,
    framework: &BTreeMap<StateKey, StateValue>,
) -> ReadSetCapturingStateView<'s, S> {
    let preloaded = framework
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    // `framework` lists every module at the framework accounts, so a head module there that it
    // lacks is absent on chain. Head modules at other accounts (`aptos-trading`,
    // `aptos-experimental`) are left to be read from chain.
    let absent = head_framework_keys()
        .iter()
        .filter(|key| key.is_aptos_code() && !framework.contains_key(*key))
        .cloned()
        .collect();
    ReadSetCapturingStateView::new(state_view, preloaded, absent, vec![])
}
