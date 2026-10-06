// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Converts a legacy dump (see [`crate::legacy`]) into a corpus, filling in what the dump lacks:
//!
//! - the framework: the on-chain one at each record's version, fetched once per release (a
//!   release is identified by the `0x1::code::PackageRegistry` value the record read), or the
//!   framework this binary was built with;
//! - the on-chain status, gas used and auxiliary info, fetched in pages of committed
//!   transactions;
//! - user modules missing from the record's module dependency closure, which MonoMove loads
//!   eagerly but the legacy VM did not need to read.
//!
//! Chain access is slow, so it is behind [`ChainSource`] and every fetch is cached on disk when a
//! cache directory is given, which also makes an interrupted import resumable. Without a chain
//! source the import is offline: the head framework is used and records whose module closure is
//! incomplete are skipped.

use crate::{
    completion,
    corpus::{
        bcs_sha3, value_hash, CorpusWriter, FrameworkSource, Manifest, OnChainInfo, Origin,
        RecordInput,
    },
    isolated::{self, V2Limits},
    legacy::{LegacyDump, LegacyEra},
    overrides::OverrideConfig,
    v1::{self, hit_metering_limit_on_chain},
};
use anyhow::{bail, ensure, Context, Result};
use aptos_crypto::HashValue;
use aptos_rest_client::{
    aptos_api_types::AptosErrorCode, error::RestError, AptosBaseUrl, Client, MoveModuleBytecode,
};
use aptos_types::{
    on_chain_config::Features,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{PersistedAuxiliaryInfo, Transaction, Version},
};
use aptos_validator_interface::{AptosValidatorInterface, RestDebuggerInterface};
use futures::{stream, StreamExt};
use mono_move_replay_common::{
    cli::rest_client,
    modules::{close_module_graph, head_framework, module_address_of},
};
use move_binary_format::CompiledModule;
use move_core_types::{account_address::AccountAddress, ident_str, language_storage::StructTag};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

/// Committed transactions are fetched in pages aligned to this size, so cached pages are shared
/// between dumps.
pub const PAGE_SIZE: u64 = 100;
/// The delays before each retry of a failed chain request: a page of transactions or auxiliary
/// info, or a lookup of the framework release. A transient failure (a rate limit) must not cost the
/// run.
#[cfg(not(test))]
const RETRY_DELAYS_SECS: [u64; 3] = [2, 4, 8];
/// No waiting in tests, whose failing chains fail every time.
#[cfg(test)]
const RETRY_DELAYS_SECS: [u64; 3] = [0, 0, 0];

/// The framework accounts. Their modules come from the framework set, never from a record.
pub const FRAMEWORK_ADDRESSES: [AccountAddress; 3] = [
    AccountAddress::ONE,
    AccountAddress::THREE,
    AccountAddress::FOUR,
];

/// A committed transaction as the chain recorded it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommittedTxn {
    pub version: Version,
    pub txn: Transaction,
    pub onchain: OnChainInfo,
    pub aux_info: PersistedAuxiliaryInfo,
}

/// What the importer reads from chain. All versions are ledger versions: the state a transaction
/// at version `v` runs against is the state at `v - 1`.
pub trait ChainSource {
    /// Every module at the framework accounts.
    fn framework_at(&self, version: Version) -> Result<BTreeMap<StateKey, StateValue>>;

    fn state_value_at(&self, key: &StateKey, version: Version) -> Result<Option<StateValue>>;

    /// A name for the source read, distinct across chains that share a chain id but are read from
    /// different endpoints (forks). Cached reads are scoped by it.
    fn identity(&self) -> Result<String>;

    /// The latest committed version.
    fn latest_version(&self) -> Result<Version>;

    /// The committed transactions in `[start, start + len)` for each range, each fetched or failed
    /// on its own so the ranges fetched can be kept.
    fn committed(&self, ranges: &[(Version, u64)]) -> Vec<Result<Vec<CommittedTxn>>>;
}

pub struct ImportConfig {
    pub legacy_dump: PathBuf,
    pub out: PathBuf,
    pub corpus_id: String,
    pub network: String,
    pub framework: FrameworkSource,
    pub shard_size: usize,
    /// Import at most this many records, from the lowest version.
    pub limit: Option<usize>,
    pub cache_dir: Option<PathBuf>,
    /// Whether to fetch the keys a replay reads beyond the dump (needs a chain).
    pub complete: bool,
    /// The limits each MonoMove replay during completion runs under.
    pub limits: V2Limits,
}

#[derive(Debug, Default, Serialize)]
pub struct ImportReport {
    pub imported: usize,
    /// Records left out, by reason.
    pub skipped: BTreeMap<String, usize>,
    pub with_onchain_status: usize,
    pub frameworks: usize,
    /// User modules added to complete a record's module closure.
    pub fetched_modules: usize,
    /// Keys the replays read beyond the dump, fetched and added as values and as absences.
    pub completed_values: usize,
    pub completed_absent: usize,
    /// Records whose recorded feature flags were dropped and, without state completion, not
    /// fetched from chain: they compare as incomplete.
    pub without_features: usize,
    /// Records whose MonoMove replay crashed during completion (see [`crate::isolated`]): kept,
    /// with only V1's reads completed, so the comparison reports the crash.
    pub v2_crashes: usize,
}

impl ImportReport {
    fn skip(&mut self, reason: &str) {
        *self.skipped.entry(reason.to_string()).or_default() += 1;
    }
}

/// Whether `inner` is `outer` or inside it, decided by the identity (device, inode) of the
/// directories on `inner`'s path rather than by their names, so that no symlink, firmlink or bind
/// mount hides it. `outer` must exist; `inner` is taken as far as it exists, as what does not exist
/// yet is plain names (see [`crate::corpus::ensure_no_inner_parent`]) under its existing part.
pub fn is_within(inner: &Path, outer: &Path) -> Result<bool> {
    use std::os::unix::fs::MetadataExt;
    let outer =
        std::fs::metadata(outer).with_context(|| format!("failed to inspect {:?}", outer))?;
    let inner = std::path::absolute(inner)?;
    let existing = inner
        .ancestors()
        .find(|ancestor| ancestor.exists())
        .context("a path has an existing ancestor")?
        .canonicalize()
        .with_context(|| format!("failed to resolve {:?}", inner))?;
    for ancestor in existing.ancestors() {
        let metadata = std::fs::metadata(ancestor)
            .with_context(|| format!("failed to inspect {:?}", ancestor))?;
        if (metadata.dev(), metadata.ino()) == (outer.dev(), outer.ino()) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The cache in `dir`, for reads of `chain`, once `out` (the corpus directory) exists. The cache
/// outlives runs, and a corpus directory holds only the corpus (and moves aside if the run stops),
/// so neither may be inside the other, decided on the directories themselves: the cache inside
/// `out` before the cache directory is created, so nothing is written there, and the rest once it
/// is. A refused run can leave the (empty) cache directory, which was asked for.
fn open_cache(dir: &Path, out: &Path, chain: &dyn ChainSource) -> Result<Cache> {
    crate::corpus::ensure_no_inner_parent(dir)?;
    let inside = format!(
        "--cache-dir {:?} and --out {:?} must not be inside one another",
        dir, out
    );
    ensure!(!is_within(dir, out)?, "{}", inside);
    // Scoped by chain, so one cache directory can serve imports from several networks.
    let cache = Cache::open(&dir.join(chain.identity()?))?;
    let root = cache.root().context("an open cache has a directory")?;
    ensure!(
        !is_within(root, out)? && !is_within(out, root)? && !is_within(out, dir)?,
        "{}",
        inside
    );
    Ok(cache)
}

pub fn import(
    config: &ImportConfig,
    chain: Option<&dyn ChainSource>,
) -> Result<(Manifest, ImportReport)> {
    if chain.is_none() {
        ensure!(
            config.framework == FrameworkSource::Head,
            "an offline import has no on-chain framework; use the head framework"
        );
    }
    let dump = LegacyDump::open(&config.legacy_dump)?;
    let era = dump.era()?;
    if era == LegacyEra::SourceOnly {
        bail!(
            "{:?} holds sources only, nothing to replay",
            config.legacy_dump
        );
    }
    let mut versions = dump.versions()?;
    if let Some(limit) = config.limit {
        versions.truncate(limit);
    }
    let origin = Origin::LegacyDump {
        source: config.legacy_dump.display().to_string(),
        era,
        framework: config.framework,
    };
    let overrides = OverrideConfig::for_origin(&origin);
    let features_key = StateKey::on_chain_config::<Features>()?;
    let mut writer = CorpusWriter::create(
        &config.out,
        &config.corpus_id,
        &config.network,
        origin,
        config.shard_size,
    )?;
    let cache = match (chain, &config.cache_dir) {
        (Some(chain), Some(dir)) => or_stop!(
            writer,
            open_cache(dir, &config.out, chain),
            "opening the cache"
        ),
        (Some(_), None) | (None, _) => Cache::none(),
    };
    let onchain_txns = match chain {
        Some(chain) => or_stop!(
            writer,
            fetch_committed(chain, &cache, &versions),
            "fetching the committed transactions"
        ),
        None => HashMap::new(),
    };
    let mut frameworks = Frameworks::default();
    let mut report = ImportReport::default();
    // The chain state is completed from, if any.
    let completing_from = chain.filter(|_| config.complete);
    for version in versions {
        let Some(state_version) = version.checked_sub(1) else {
            report.skip("genesis");
            continue;
        };
        let record = match dump.read(version) {
            Ok(record) => record,
            // Missing from the dump as well as undecodable: the error tells which.
            Err(err) => {
                eprintln!(
                    "version {}: skip: unreadable from the dump: {:#}",
                    version, err
                );
                report.skip("unreadable from the dump");
                continue;
            },
        };
        let committed = match chain {
            Some(_) => match onchain_txns.get(&version) {
                // The dump's transaction must be the one the chain committed at that version, or
                // the dump was taken from another network.
                Some(c) if c.txn == record.txn => Some(c),
                Some(_) => {
                    report.skip("transaction differs from chain");
                    continue;
                },
                None => {
                    report.skip("not on chain");
                    continue;
                },
            },
            None => None,
        };
        // Replaying it gas-free could loop for ever where gas stopped it on chain, and it would not
        // be comparable anyway.
        if committed.is_some_and(|c| hit_metering_limit_on_chain(&c.onchain.status)) {
            report.skip("hit a metering limit on chain");
            continue;
        }
        let (release, framework_modules) = match (config.framework, chain) {
            (FrameworkSource::Head, _) => frameworks.head(),
            (FrameworkSource::OnChain, Some(chain)) => {
                match frameworks.release(chain, &cache, &record.state, state_version) {
                    Ok(Some(framework)) => framework,
                    Ok(None) => {
                        report.skip("no 0x1 package registry");
                        continue;
                    },
                    // As in capture: a version whose release cannot be looked up (its state
                    // pruned, say) is skipped like any other that cannot be imported.
                    Err(err) => {
                        eprintln!("version {}: skip: {:#}", version, err);
                        report.skip("framework release lookup failed");
                        continue;
                    },
                }
            },
            (FrameworkSource::OnChain, None) => unreachable!("checked above"),
        };
        // The legacy VM read only the modules it loaded; MonoMove needs the whole closure. The
        // framework set provides the framework accounts' modules.
        let mut state = record.state;
        // The framework set owns its accounts' modules outright: its copy of a module wins over one the
        // dump recorded, and a module it lacks there is absent. In head mode the dump can hold older
        // `0x5` or `0x7` modules, some since removed or moved, which would otherwise mix two
        // frameworks.
        let owned: BTreeSet<AccountAddress> = framework_modules
            .keys()
            .filter_map(module_address_of)
            .collect();
        let is_owned =
            |key: &StateKey| module_address_of(key).is_some_and(|address| owned.contains(&address));
        let mut absent: BTreeSet<StateKey> = state
            .keys()
            .filter(|key| is_owned(key) && !framework_modules.contains_key(*key))
            .cloned()
            .collect();
        state.retain(|key, _| !is_owned(key));
        // The dump tool could override feature flags and record the overridden config as if read
        // from chain, and nothing in a dump tells whether it did, so the config is always dropped
        // and completion fetches the chain's. Without completion a replay reads it unobserved
        // and the record compares as incomplete, which the report counts.
        let dropped_features = state.remove(&features_key).is_some();
        // A failure with one record's closure (a failed chain read, a module that does not
        // deserialize) skips the record.
        // The framework set (on chain, or the head framework, which also covers `0x5` and `0x7`)
        // is complete with its closure: only modules outside it are walked, and a module it lacks at
        // its accounts is never fetched, so a dependency on one leaves the closure incomplete.
        let closure = match close_module_graph(
            &mut state,
            |id| framework_modules.contains_key(&StateKey::module_id(id)),
            |key| match (chain, is_owned(key)) {
                (Some(chain), false) => cache.state_at(chain, key, state_version),
                (Some(_), true) | (None, _) => Ok(None),
            },
        ) {
            Ok(closure) => closure,
            Err(err) => {
                eprintln!(
                    "version {}: skip: closing the module graph failed: {:#}",
                    version, err
                );
                report.skip("closing the module graph failed");
                continue;
            },
        };
        if !closure.missing.is_empty() {
            report.skip("incomplete module closure");
            continue;
        }
        let aux_info = committed.map_or(PersistedAuxiliaryInfo::None, |c| c.aux_info);
        // Legacy dumps recorded only reads that found a value. With a chain, the keys a replay
        // reads beyond them are fetched; offline, a replay that reads one reports the record as
        // incomplete.
        let mut v2_crashed = false;
        if let Some(chain) = completing_from {
            let completion = match completion::complete(
                &record.txn,
                aux_info,
                &framework_modules,
                &mut state,
                &mut absent,
                &overrides,
                &config.limits,
                |key| match is_owned(key) {
                    true => Ok(None),
                    false => cache.state_at(chain, key, state_version),
                },
            ) {
                Ok(completion) => completion,
                // A broken harness would fail every record alike, so it stops the import.
                Err(err) if isolated::is_harness_error(&err) => {
                    return Err(
                        writer.finish_after(err, &format!("completing version {}", version))
                    );
                },
                Err(err) => {
                    eprintln!(
                        "version {}: skip: completing the state failed: {:#}",
                        version, err
                    );
                    report.skip(v1::MeteringStop::skip_reason(
                        &err,
                        "completing the state failed",
                    ));
                    continue;
                },
            };
            if completion.incomplete {
                report.skip("state still incomplete after completing it");
                continue;
            }
            report.completed_values += completion.values;
            report.completed_absent += completion.absent;
            v2_crashed = completion.v2_crashed;
        }
        if committed.is_some() {
            report.with_onchain_status += 1;
        }
        report.fetched_modules += closure.fetched;
        report.v2_crashes += usize::from(v2_crashed);
        if dropped_features && completing_from.is_none() {
            report.without_features += 1;
        }
        let framework = or_stop!(
            writer,
            frameworks.add(&mut writer, release, &framework_modules),
            format!("importing version {}", version)
        );
        or_stop!(
            writer,
            writer.add(RecordInput {
                version,
                txn: record.txn,
                aux_info,
                state,
                framework: Some(framework),
                absent,
                onchain: committed.map(|c| c.onchain.clone()),
            }),
            format!("importing version {}", version)
        );
        report.imported += 1;
    }
    report.frameworks = frameworks.count();
    let manifest = writer.finish()?;
    Ok((manifest, report))
}

/// Fetches the committed transactions at `versions`, keyed by version; versions past the ledger
/// are left out. With a cache, whole pages aligned to [`PAGE_SIZE`] are fetched so later runs
/// reuse them; without one, only the contiguous runs of `versions`.
pub(crate) fn fetch_committed(
    chain: &dyn ChainSource,
    cache: &Cache,
    versions: &[Version],
) -> Result<HashMap<Version, CommittedTxn>> {
    let wanted: BTreeSet<Version> = versions.iter().copied().collect();
    let mut txns = HashMap::new();
    // Only the wanted transactions of each page are kept.
    let mut keep = |page: Vec<CommittedTxn>| {
        txns.extend(
            page.into_iter()
                .filter(|c| wanted.contains(&c.version))
                .map(|c| (c.version, c)),
        )
    };
    let missing = if cache.is_enabled() {
        let starts: BTreeSet<Version> = wanted.iter().map(|v| v / PAGE_SIZE * PAGE_SIZE).collect();
        let mut missing = vec![];
        for start in starts {
            match cache.get::<Vec<CommittedTxn>>(&page_path(start)) {
                Some(page) => keep(page),
                None => missing.push(start),
            }
        }
        if missing.is_empty() {
            vec![]
        } else {
            // The last page of the ledger is partial.
            let latest = chain.latest_version()?;
            missing
                .into_iter()
                .filter(|start| *start <= latest)
                .map(|start| (start, PAGE_SIZE.min((latest - start).saturating_add(1))))
                .collect()
        }
    } else {
        let latest = chain.latest_version()?;
        let wanted: BTreeSet<Version> = wanted.range(..=latest).copied().collect();
        contiguous_runs(&wanted, PAGE_SIZE)
    };
    if !missing.is_empty() {
        let fetched = chain.committed(&missing);
        ensure!(
            fetched.len() == missing.len(),
            "asked for {} ranges, got {}",
            missing.len(),
            fetched.len()
        );
        // Every range fetched is kept (and cached) before a failed one is reported, so a rerun
        // resumes from where this one stopped.
        let mut failure = None;
        for ((start, len), range) in missing.into_iter().zip(fetched) {
            let range = match range {
                Ok(range) => range,
                Err(err) => {
                    failure.get_or_insert(err);
                    continue;
                },
            };
            if range.len() as u64 != len {
                failure.get_or_insert(anyhow::anyhow!(
                    "asked for {} transactions from {}, got {}",
                    len,
                    start,
                    range.len()
                ));
                continue;
            }
            // Only a full page is final: the last page of the ledger still grows.
            if cache.is_enabled() && len == PAGE_SIZE {
                cache.put(&page_path(start), &range);
            }
            keep(range);
        }
        if let Some(err) = failure {
            return Err(err);
        }
    }
    Ok(txns)
}

/// The first version and the count still to fetch of `[start, start + len)` once `done` of it
/// are fetched.
fn next_page(start: Version, len: u64, done: usize) -> Result<(Version, u64)> {
    let done = u64::try_from(done)?;
    let next = start
        .checked_add(done)
        .with_context(|| format!("version overflow past {}", start))?;
    let remaining = len
        .checked_sub(done)
        .with_context(|| format!("fetched more than {} from {}", len, start))?;
    Ok((next, remaining))
}

/// `versions` as `(start, len)` runs of consecutive versions, each at most `max_len` long.
fn contiguous_runs(versions: &BTreeSet<Version>, max_len: u64) -> Vec<(Version, u64)> {
    let mut runs: Vec<(Version, u64)> = vec![];
    for &version in versions {
        match runs.last_mut() {
            Some((start, len)) if start.checked_add(*len) == Some(version) && *len < max_len => {
                *len += 1
            },
            _ => runs.push((version, 1)),
        }
    }
    runs
}

fn page_path(start: Version) -> String {
    format!("committed/{}.bcs", start)
}

/// On-chain framework releases, each fetched once.
#[derive(Default)]
pub(crate) struct Releases {
    /// Keyed by the hash of the framework accounts' package registries.
    modules: HashMap<HashValue, Arc<BTreeMap<StateKey, StateValue>>>,
}

impl Releases {
    /// The framework the transaction after `state_version` ran against. Every upgrade of a
    /// framework account rewrites that account's package registry, so the registries of `0x1`,
    /// `0x3` and `0x4` together identify the release: taken from `state` when it holds them,
    /// fetched otherwise. Returns the release id and its modules; `None` if `0x1` has no registry.
    pub(crate) fn at(
        &mut self,
        chain: &dyn ChainSource,
        cache: &Cache,
        state: &BTreeMap<StateKey, StateValue>,
        state_version: Version,
    ) -> Result<Option<(HashValue, Arc<BTreeMap<StateKey, StateValue>>)>> {
        release_id(chain, cache, state, state_version)?
            .map(|release| {
                Ok((
                    release,
                    self.modules_of(chain, cache, release, state_version)?,
                ))
            })
            .transpose()
    }

    /// [`Self::at`] for each of `state_versions`, which must be increasing, reading the registries
    /// only at the two ends of a run of versions, and within it only where they differ. A package
    /// registry changes only by an upgrade, which adds a package or raises a package's
    /// `upgrade_number`, so it never returns to an earlier value: registries equal at both ends of
    /// a run are equal throughout it. A capture window rarely spans an upgrade, so its releases
    /// usually take two lookups and one framework fetch in all.
    pub(crate) fn at_each(
        &mut self,
        chain: &dyn ChainSource,
        cache: &Cache,
        state_versions: &[Version],
    ) -> Vec<Result<Option<(HashValue, Arc<BTreeMap<StateKey, StateValue>>)>>> {
        // A failed lookup is the failure of the versions it stands for only: a run whose end failed
        // is split until the failure is the version's alone.
        let mut ids: Vec<std::result::Result<Option<HashValue>, String>> =
            vec![Ok(None); state_versions.len()];
        let mut looked_up = HashMap::new();
        let mut id_at = |index: usize| -> std::result::Result<Option<HashValue>, String> {
            let version = state_versions[index];
            looked_up
                .entry(version)
                .or_insert_with(|| {
                    release_id(chain, cache, &BTreeMap::new(), version)
                        .map_err(|err| format!("{:#}", err))
                })
                .clone()
        };
        // Runs of indexes to resolve, each by the release ids at its two ends.
        let mut runs = match state_versions.len() {
            0 => vec![],
            len => vec![(0, len - 1)],
        };
        while let Some((first, last)) = runs.pop() {
            let (first_id, last_id) = (id_at(first), id_at(last));
            if first_id.is_ok() && first_id == last_id {
                ids[first..=last].fill(first_id);
            } else if last - first <= 1 {
                ids[first] = first_id;
                ids[last] = last_id;
            } else {
                let middle = first + (last - first) / 2;
                runs.push((first, middle));
                runs.push((middle, last));
            }
        }
        let mut releases = Vec::with_capacity(ids.len());
        for (id, &state_version) in ids.into_iter().zip(state_versions) {
            releases.push(match id {
                Ok(Some(release)) => self
                    .modules_of(chain, cache, release, state_version)
                    .map(|modules| Some((release, modules))),
                Ok(None) => Ok(None),
                Err(err) => Err(anyhow::anyhow!("{}", err)),
            });
        }
        releases
    }

    /// The modules of `release`, fetched at `state_version` the first time it is needed.
    fn modules_of(
        &mut self,
        chain: &dyn ChainSource,
        cache: &Cache,
        release: HashValue,
        state_version: Version,
    ) -> Result<Arc<BTreeMap<StateKey, StateValue>>> {
        if let Some(modules) = self.modules.get(&release) {
            return Ok(modules.clone());
        }
        let modules = Arc::new(
            cache.get_or_fetch(&format!("frameworks/{}.bcs", release), || {
                chain.framework_at(state_version)
            })?,
        );
        self.modules.insert(release, modules.clone());
        Ok(modules)
    }
}

/// `f`, retried after each of [`RETRY_DELAYS_SECS`] while it fails, unless the node rejected the
/// request for good: retrying a pruned version only delays the failure.
fn retrying<T>(mut f: impl FnMut() -> Result<T>) -> Result<T> {
    let mut delays = RETRY_DELAYS_SECS.into_iter();
    loop {
        match f() {
            Ok(value) => return Ok(value),
            Err(err) => match delays.next() {
                Some(delay) if !is_rejected(&err) => std::thread::sleep(Duration::from_secs(delay)),
                Some(_) | None => return Err(err),
            },
        }
    }
}

/// [`retrying`] for an asynchronous `f`.
async fn retrying_async<T, F: std::future::Future<Output = Result<T>>>(
    mut f: impl FnMut() -> F,
) -> Result<T> {
    let mut delays = RETRY_DELAYS_SECS.into_iter();
    loop {
        match f().await {
            Ok(value) => return Ok(value),
            Err(err) => match delays.next() {
                Some(delay) if !is_rejected(&err) => {
                    tokio::time::sleep(Duration::from_secs(delay)).await
                },
                Some(_) | None => return Err(err),
            },
        }
    }
}

/// Whether `err` is a client error from the node other than a timeout, a rate limit, or a version the
/// node has not reached yet (a 404 `version_not_found` from a node behind the others).
fn is_rejected(err: &anyhow::Error) -> bool {
    let rejected = |status: u16| (400..500).contains(&status) && status != 408 && status != 429;
    match err.downcast_ref::<RestError>() {
        Some(RestError::Api(response)) => {
            !matches!(response.error.error_code, AptosErrorCode::VersionNotFound)
                && rejected(response.status_code.as_u16())
        },
        Some(RestError::Http(status, _)) => rejected(status.as_u16()),
        Some(
            RestError::Bcs(_)
            | RestError::Json(_)
            | RestError::UrlParse(_)
            | RestError::Timeout(_)
            | RestError::Unknown(_),
        )
        | None => false,
    }
}

/// The release the transaction after `state_version` ran against: the hash of the framework
/// accounts' package registries, taken from `state` when it holds them and fetched otherwise.
/// `None` if `0x1` has no registry.
fn release_id(
    chain: &dyn ChainSource,
    cache: &Cache,
    state: &BTreeMap<StateKey, StateValue>,
    state_version: Version,
) -> Result<Option<HashValue>> {
    let mut registries = vec![];
    for address in FRAMEWORK_ADDRESSES {
        let key = package_registry_key(address)?;
        let registry = match state.get(&key) {
            Some(value) => Some(value.clone()),
            None => cache.state_at(chain, &key, state_version)?,
        };
        registries.push(registry.as_ref().map(value_hash).transpose()?);
    }
    if registries[0].is_none() {
        return Ok(None);
    }
    Ok(Some(bcs_sha3(&registries, "package registries")?))
}

/// The frameworks records run against, each added to the corpus as a framework set the first time
/// a record that is written needs it, so a skipped record leaves no unreferenced set behind.
#[derive(Default)]
pub(crate) struct Frameworks {
    head: Option<Arc<BTreeMap<StateKey, StateValue>>>,
    releases: Releases,
    /// Framework set ids by release id, `None` being the head framework.
    sets: HashMap<Option<HashValue>, HashValue>,
}

impl Frameworks {
    /// The framework sets added: releases whose modules are identical share one.
    fn count(&self) -> usize {
        self.sets.values().collect::<BTreeSet<_>>().len()
    }

    /// The head framework, as a release id and its modules.
    fn head(&mut self) -> (Option<HashValue>, Arc<BTreeMap<StateKey, StateValue>>) {
        let modules = self
            .head
            .get_or_insert_with(|| Arc::new(head_framework()))
            .clone();
        (None, modules)
    }

    /// The on-chain release the record ran against, as a release id and its modules (see
    /// [`Releases::at`]); `None` if `0x1` has no registry.
    pub(crate) fn release(
        &mut self,
        chain: &dyn ChainSource,
        cache: &Cache,
        state: &BTreeMap<StateKey, StateValue>,
        state_version: Version,
    ) -> Result<Option<(Option<HashValue>, Arc<BTreeMap<StateKey, StateValue>>)>> {
        Ok(self
            .releases
            .at(chain, cache, state, state_version)?
            .map(|(release, modules)| (Some(release), modules)))
    }

    /// [`Self::release`] for each of `state_versions`, which must be increasing (see
    /// [`Releases::at_each`]).
    pub(crate) fn releases(
        &mut self,
        chain: &dyn ChainSource,
        cache: &Cache,
        state_versions: &[Version],
    ) -> Vec<Result<Option<(Option<HashValue>, Arc<BTreeMap<StateKey, StateValue>>)>>> {
        self.releases
            .at_each(chain, cache, state_versions)
            .into_iter()
            .map(|release| release.map(|release| release.map(|(id, modules)| (Some(id), modules))))
            .collect()
    }

    /// The framework set of `release`, added to the corpus the first time it is needed.
    pub(crate) fn add(
        &mut self,
        writer: &mut CorpusWriter,
        release: Option<HashValue>,
        modules: &BTreeMap<StateKey, StateValue>,
    ) -> Result<HashValue> {
        if let Some(framework) = self.sets.get(&release) {
            return Ok(*framework);
        }
        let framework = writer.add_framework(modules.clone())?;
        self.sets.insert(release, framework);
        Ok(framework)
    }
}

/// The key of `address`'s package registry.
fn package_registry_key(address: AccountAddress) -> Result<StateKey> {
    let tag = StructTag {
        address: AccountAddress::ONE,
        module: ident_str!("code").to_owned(),
        name: ident_str!("PackageRegistry").to_owned(),
        type_args: vec![],
    };
    StateKey::resource(&address, &tag).context("bad package registry key")
}

fn key_hash(key: &StateKey) -> Result<HashValue> {
    bcs_sha3(key, "state key")
}

/// An optional on-disk cache of chain reads, keyed by a relative path. It is best-effort: an entry
/// that cannot be read is fetched again, and one that cannot be written costs only a refetch on a
/// rerun, so neither skips a record nor stops the run; each is warned about once.
pub(crate) struct Cache {
    root: Option<PathBuf>,
}

impl Cache {
    pub(crate) fn none() -> Self {
        Self { root: None }
    }

    /// A cache under `dir`, for reads of the chain named `identity`, created lazily (see
    /// [`Self::open`] for the one a run uses).
    #[cfg(test)]
    pub(crate) fn scoped(dir: Option<PathBuf>, identity: &str) -> Self {
        Self {
            root: dir.map(|dir| dir.join(identity)),
        }
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.root.is_some()
    }

    fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    /// The cache in `dir`, created if need be and resolved, so that where it is is known before
    /// anything is written to it, and stays where it is.
    pub(crate) fn open(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("failed to create the cache directory {:?}", dir))?;
        let root = dir
            .canonicalize()
            .with_context(|| format!("failed to resolve the cache directory {:?}", dir))?;
        Ok(Self { root: Some(root) })
    }

    fn get<T: DeserializeOwned>(&self, path: &str) -> Option<T> {
        let root = self.root.as_ref()?;
        let path = root.join(path);
        if !path.exists() {
            return None;
        }
        let read = std::fs::read(&path)
            .with_context(|| format!("failed to read {:?}", path))
            .and_then(|bytes| {
                bcs::from_bytes(&bytes).with_context(|| format!("failed to decode {:?}", path))
            });
        match read {
            Ok(value) => Some(value),
            Err(err) => {
                static WARNED: std::sync::Once = std::sync::Once::new();
                WARNED.call_once(|| {
                    eprintln!(
                        "warning: a cache entry cannot be read, so it is fetched again ({:#})",
                        err
                    )
                });
                None
            },
        }
    }

    /// Stores `value` at `path`, best-effort.
    fn put<T: Serialize>(&self, path: &str, value: &T) {
        if let Err(err) = self.try_put(path, value) {
            static WARNED: std::sync::Once = std::sync::Once::new();
            WARNED.call_once(|| eprintln!("warning: the cache cannot be written ({:#})", err));
        }
    }

    fn try_put<T: Serialize>(&self, path: &str, value: &T) -> Result<()> {
        let Some(root) = &self.root else {
            return Ok(());
        };
        let path = root.join(path);
        let parent = path.parent().context("a cache entry has a directory")?;
        // Only into the cache: a directory in it that is a link out (to anything) is not written
        // through.
        std::fs::create_dir_all(root).with_context(|| format!("failed to create {:?}", root))?;
        ensure!(
            is_within(parent, root)?,
            "{:?} leads out of the cache {:?}",
            parent,
            root
        );
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {:?}", parent))?;
        // Write then rename, so an interrupted import never leaves a truncated entry. The temporary
        // file is created afresh (whatever an interrupted write left is removed first), so that it
        // is never written through a symlink, and removed if the write fails. Its name is the
        // process's, as processes may share the cache directory (a process writes it sequentially).
        let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
        let bytes = bcs::to_bytes(value).context("failed to serialize cache entry")?;
        match std::fs::remove_file(&tmp) {
            Err(err) if err.kind() != std::io::ErrorKind::NotFound => {
                return Err(err).with_context(|| format!("failed to remove {:?}", tmp));
            },
            Ok(()) | Err(_) => {},
        }
        let written = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .and_then(|mut file| file.write_all(&bytes))
            .and_then(|()| std::fs::rename(&tmp, &path));
        if let Err(err) = written {
            let _ = std::fs::remove_file(&tmp);
            return Err(err).with_context(|| format!("failed to write {:?}", path));
        }
        Ok(())
    }

    fn get_or_fetch<T: Serialize + DeserializeOwned>(
        &self,
        path: &str,
        fetch: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        if let Some(value) = self.get(path) {
            return Ok(value);
        }
        let value = fetch()?;
        self.put(path, &value);
        Ok(value)
    }

    /// `key`'s value at `version` on `chain`, through the cache: the one place the layout of
    /// cached state reads is decided.
    fn state_at(
        &self,
        chain: &dyn ChainSource,
        key: &StateKey,
        version: Version,
    ) -> Result<Option<StateValue>> {
        self.get_or_fetch(&format!("state/{}/{}", version, key_hash(key)?), || {
            chain.state_value_at(key, version)
        })
    }
}

/// [`ChainSource`] over the REST API. Pages of committed transactions are fetched concurrently.
pub struct RestChainSource {
    runtime: tokio::runtime::Runtime,
    client: Client,
    debugger: RestDebuggerInterface,
    concurrency: usize,
}

impl RestChainSource {
    pub fn new(
        base_url: AptosBaseUrl,
        api_key: Option<String>,
        concurrency: usize,
    ) -> Result<Self> {
        Self::from_client(rest_client(base_url, api_key)?, concurrency)
    }

    pub fn from_client(client: Client, concurrency: usize) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .context("failed to build tokio runtime")?;
        Ok(Self {
            runtime,
            debugger: RestDebuggerInterface::new(client.clone()),
            client,
            concurrency: concurrency.max(1),
        })
    }

    fn ledger(&self) -> Result<aptos_rest_client::State> {
        Ok(self
            .runtime
            .block_on(self.client.get_ledger_information())
            .context("failed to fetch the ledger info")?
            .into_inner())
    }

    /// One page of auxiliary infos from `start`, retried since a transient failure must not cost
    /// the range.
    async fn aux_infos(&self, start: Version, len: u64) -> Result<Vec<PersistedAuxiliaryInfo>> {
        retrying_async(|| async move {
            let batch = self
                .client
                .get_persisted_auxiliary_infos(start, len)
                .await?;
            ensure!(!batch.is_empty(), "no auxiliary infos from {}", start);
            Ok(batch)
        })
        .await
        .with_context(|| format!("failed to fetch the auxiliary info from {}", start))
    }

    /// The committed transactions in `[start, start + len)`. Fetched with the client rather than
    /// the validator interface, which silently substitutes `None` when the auxiliary info cannot
    /// be fetched; a missing index changes what natives such as
    /// `monotonically_increasing_counter` return, so here it is an error.
    async fn committed_range(&self, start: Version, len: u64) -> Result<Vec<CommittedTxn>> {
        let mut txns = Vec::with_capacity(len as usize);
        while (txns.len() as u64) < len {
            let (next, remaining) = next_page(start, len, txns.len())?;
            let limit = u16::try_from(remaining)?;
            // Retried like the auxiliary info: a transient failure (a rate limit, or a node that has no
            // page yet) must not cost the run.
            let batch = retrying_async(|| async move {
                let batch = self
                    .client
                    .get_transactions_bcs(Some(next), Some(limit))
                    .await?
                    .into_inner();
                ensure!(!batch.is_empty(), "no transactions from {}", next);
                Ok(batch)
            })
            .await
            .with_context(|| format!("failed to fetch transactions from {}", next))?;
            txns.extend(batch);
        }
        // Paged like the transactions, since a node caps the page size.
        let mut aux_infos = Vec::with_capacity(len as usize);
        while (aux_infos.len() as u64) < len {
            let (next, remaining) = next_page(start, len, aux_infos.len())?;
            let batch = self.aux_infos(next, remaining).await?;
            ensure!(
                batch.len() as u64 <= remaining,
                "asked for {} auxiliary infos from {}, got {}",
                remaining,
                next,
                batch.len()
            );
            aux_infos.extend(batch);
        }
        txns.into_iter()
            .zip(aux_infos)
            .zip(start..)
            .map(|((txn, aux_info), version)| {
                // The auxiliary info is matched by position.
                ensure!(
                    txn.version == version,
                    "expected version {}, got {}",
                    version,
                    txn.version
                );
                Ok(CommittedTxn {
                    version,
                    onchain: OnChainInfo {
                        status: txn.info.status().clone(),
                        gas_used: txn.info.gas_used(),
                    },
                    txn: txn.transaction,
                    aux_info,
                })
            })
            .collect()
    }

    /// Every module at the framework accounts at `version`, read once.
    fn list_framework(&self, version: Version) -> Result<BTreeMap<StateKey, StateValue>> {
        self.runtime.block_on(async {
            let mut modules = BTreeMap::new();
            for address in FRAMEWORK_ADDRESSES {
                // The JSON listing, because the BCS one is a map whose keys are not in BCS
                // canonical order, which BCS decoding rejects.
                let listed = match self
                    .client
                    .paginate_with_cursor::<MoveModuleBytecode>(
                        &format!("accounts/{}/modules", address.to_hex()),
                        1000,
                        Some(version),
                    )
                    .await
                {
                    Ok(response) => response.into_inner(),
                    // `0x4` did not exist in early mainnet. Only a missing account is skipped: a 404
                    // for the version (a node behind it) would silently drop the account's modules.
                    Err(RestError::Api(err))
                        if err.status_code.as_u16() == 404
                            && matches!(err.error.error_code, AptosErrorCode::AccountNotFound) =>
                    {
                        continue
                    },
                    Err(err) => {
                        return Err(err).with_context(|| {
                            format!("failed to list the modules of {} at {}", address, version)
                        })
                    },
                };
                for module in listed {
                    let code = module.bytecode.0;
                    let id = CompiledModule::deserialize(&code)
                        .with_context(|| format!("undecodable module at {}", address))?
                        .self_id();
                    modules.insert(
                        StateKey::module_id(&id),
                        StateValue::new_legacy(code.into()),
                    );
                }
            }
            ensure!(!modules.is_empty(), "no framework modules at {}", version);
            Ok(modules)
        })
    }
}

impl ChainSource for RestChainSource {
    /// Retried: a transient failure (a rate limit, a node behind) must not cost the run.
    fn framework_at(&self, version: Version) -> Result<BTreeMap<StateKey, StateValue>> {
        retrying(|| self.list_framework(version))
    }

    /// Retried, like every read from the node.
    fn state_value_at(&self, key: &StateKey, version: Version) -> Result<Option<StateValue>> {
        retrying(|| {
            self.runtime
                .block_on(self.debugger.get_state_value_by_version(key, version))
        })
    }

    /// The chain id and the endpoint: deterministic, and distinct for forks read from different
    /// endpoints. What a node serves cannot identify a chain more reliably (genesis may be pruned,
    /// or served by only some backends), so a chain reset behind the same endpoint needs a new
    /// cache directory.
    fn identity(&self) -> Result<String> {
        let chain_id = self.ledger()?.chain_id;
        let endpoint = HashValue::sha3_256_of(self.client.path_prefix_string().as_bytes());
        Ok(format!("chain-{}-{}", chain_id, &endpoint.to_hex()[..16]))
    }

    fn latest_version(&self) -> Result<Version> {
        Ok(self.ledger()?.version)
    }

    fn committed(&self, ranges: &[(Version, u64)]) -> Vec<Result<Vec<CommittedTxn>>> {
        self.runtime.block_on(async {
            let mut fetched: Vec<(usize, Result<Vec<CommittedTxn>>)> = stream::iter(
                ranges.iter().enumerate(),
            )
            .map(|(i, &(start, len))| async move { (i, self.committed_range(start, len).await) })
            .buffer_unordered(self.concurrency)
            .collect()
            .await;
            fetched.sort_by_key(|(i, _)| *i);
            fetched.into_iter().map(|(_, range)| range).collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::Corpus;
    use aptos_comparison_testing_dump_format::{DataManager, PackageInfo, TxnIndex};
    use aptos_types::transaction::ExecutionStatus;
    use move_binary_format::{
        file_format::empty_module_with_dependencies_and_friends_at_addr,
        file_format_common::VERSION_DEFAULT,
    };
    use std::{cell::RefCell, path::Path};

    fn user_address() -> AccountAddress {
        AccountAddress::from_hex_literal("0xa").expect("valid address")
    }

    /// A user module at `0xa` depending on other `0xa` modules.
    fn user_module(name: &str, deps: &[&str]) -> (StateKey, StateValue) {
        let mut module = empty_module_with_dependencies_and_friends_at_addr(
            user_address(),
            name,
            deps.iter().copied(),
            [],
        );
        module.version = VERSION_DEFAULT;
        let mut bytes = vec![];
        module.serialize(&mut bytes).expect("serialize module");
        (
            StateKey::module(
                &user_address(),
                &move_core_types::identifier::Identifier::new(name).expect("valid name"),
            ),
            StateValue::new_legacy(bytes.into()),
        )
    }

    fn framework(tag: u8) -> BTreeMap<StateKey, StateValue> {
        [(
            StateKey::module(&AccountAddress::ONE, ident_str!("coin")),
            StateValue::new_legacy(vec![tag].into()),
        )]
        .into_iter()
        .collect()
    }

    fn registry(tag: u8) -> (StateKey, StateValue) {
        (
            package_registry_key(AccountAddress::ONE).expect("registry key"),
            StateValue::new_legacy(vec![tag; 8].into()),
        )
    }

    fn txn(version: Version) -> Transaction {
        Transaction::StateCheckpoint(HashValue::sha3_256_of(&version.to_be_bytes()))
    }

    /// Writes a legacy dump with the given records, with the legacy tool's own writer.
    fn write_dump(root: &Path, records: Vec<(Version, Vec<(StateKey, StateValue)>)>) {
        let data = DataManager::new_with_dir_creation(root);
        for (version, state) in records {
            data.dump_txn_index(version, &TxnIndex {
                version,
                package_info: PackageInfo {
                    address: user_address(),
                    package_name: "pkg".to_string(),
                    upgrade_number: Some(1),
                },
                txn: txn(version),
            });
            data.dump_state_data(version, &state.into_iter().collect());
        }
    }

    #[derive(Default)]
    struct FakeChain {
        /// Framework by the ledger version from which it is live.
        frameworks: Vec<(Version, BTreeMap<StateKey, StateValue>)>,
        modules: BTreeMap<StateKey, StateValue>,
        /// Transactions the chain holds, overriding the default `txn(version)`.
        txns: HashMap<Version, Transaction>,
        framework_calls: RefCell<usize>,
        page_calls: RefCell<usize>,
        fail: bool,
        /// The latest version, if not unbounded.
        latest: Option<Version>,
        /// The start of a range that fails to fetch.
        failing_range: Option<Version>,
    }

    impl ChainSource for FakeChain {
        fn framework_at(&self, version: Version) -> Result<BTreeMap<StateKey, StateValue>> {
            ensure!(!self.fail, "chain unavailable");
            *self.framework_calls.borrow_mut() += 1;
            Ok(self
                .frameworks
                .iter()
                .rev()
                .find(|(from, _)| *from <= version)
                .expect("a framework")
                .1
                .clone())
        }

        fn state_value_at(&self, key: &StateKey, _: Version) -> Result<Option<StateValue>> {
            ensure!(!self.fail, "chain unavailable");
            Ok(self.modules.get(key).cloned())
        }

        fn identity(&self) -> Result<String> {
            Ok("chain-1".to_string())
        }

        fn latest_version(&self) -> Result<Version> {
            ensure!(!self.fail, "chain unavailable");
            Ok(self.latest.unwrap_or(Version::MAX))
        }

        fn committed(&self, ranges: &[(Version, u64)]) -> Vec<Result<Vec<CommittedTxn>>> {
            *self.page_calls.borrow_mut() += ranges.len();
            ranges
                .iter()
                .map(|&(start, len)| {
                    ensure!(!self.fail, "chain unavailable");
                    ensure!(self.failing_range != Some(start), "range unavailable");
                    Ok((start..start + len)
                        .map(|version| CommittedTxn {
                            version,
                            txn: self.txns.get(&version).cloned().unwrap_or(txn(version)),
                            onchain: OnChainInfo {
                                status: ExecutionStatus::Success,
                                gas_used: version,
                            },
                            aux_info: PersistedAuxiliaryInfo::V1 {
                                transaction_index: version as u32,
                            },
                        })
                        .collect())
                })
                .collect()
        }
    }

    fn config(dump: &Path, out: &Path, framework: FrameworkSource) -> ImportConfig {
        ImportConfig {
            legacy_dump: dump.to_path_buf(),
            out: out.to_path_buf(),
            corpus_id: "test".to_string(),
            network: "mainnet".to_string(),
            framework,
            shard_size: 2,
            limit: None,
            cache_dir: None,
            // The synthetic records cannot be replayed.
            complete: false,
            limits: V2Limits::default(),
        }
    }

    fn states(corpus: &Corpus) -> BTreeMap<Version, HashMap<StateKey, StateValue>> {
        let mut states = BTreeMap::new();
        for n in 0..corpus.manifest().shards.len() {
            let shard = corpus.load_shard(n).expect("load shard");
            for record in &shard.records {
                states.insert(
                    record.version,
                    corpus.record_state(&shard, record).expect("state"),
                );
            }
        }
        states
    }

    #[test]
    fn online_import_fills_framework_status_and_modules() {
        let dump = tempfile::tempdir().expect("tempdir");
        let out = tempfile::tempdir().expect("tempdir");
        let (a_key, a) = user_module("a", &["b"]);
        let (b_key, b) = user_module("b", &[]);
        write_dump(dump.path(), vec![
            (10, vec![registry(1), (a_key.clone(), a)]),
            (11, vec![registry(1)]),
            (250, vec![registry(2)]),
        ]);
        let chain = FakeChain {
            frameworks: vec![(0, framework(1)), (200, framework(2))],
            modules: [(b_key.clone(), b.clone())].into_iter().collect(),
            ..FakeChain::default()
        };

        let (manifest, report) = import(
            &config(dump.path(), out.path(), FrameworkSource::OnChain),
            Some(&chain),
        )
        .expect("import");
        assert_eq!(report.imported, 3);
        assert_eq!(report.with_onchain_status, 3);
        assert_eq!(report.fetched_modules, 1);
        assert_eq!(report.frameworks, 2);
        assert_eq!(manifest.num_frameworks, 2);
        // One fetch per registry value, one page per 100 versions.
        assert_eq!(*chain.framework_calls.borrow(), 2);
        assert_eq!(*chain.page_calls.borrow(), 2);

        let corpus = Corpus::open(out.path()).expect("open corpus");
        let states = states(&corpus);
        let coin = StateKey::module(&AccountAddress::ONE, ident_str!("coin"));
        assert_eq!(states[&10].get(&b_key), Some(&b));
        assert_eq!(states[&11].get(&coin), framework(1).get(&coin));
        assert_eq!(states[&250].get(&coin), framework(2).get(&coin));
        let shard = corpus.load_shard(0).expect("load shard");
        assert_eq!(
            shard.records[0].onchain.as_ref().map(|o| o.gas_used),
            Some(10)
        );
        assert_eq!(shard.records[0].aux_info, PersistedAuxiliaryInfo::V1 {
            transaction_index: 10
        });
    }

    #[test]
    fn upgrade_of_0x3_alone_is_a_new_framework() {
        let dump = tempfile::tempdir().expect("tempdir");
        let out = tempfile::tempdir().expect("tempdir");
        let token_registry = |tag: u8| {
            (
                package_registry_key(AccountAddress::THREE).expect("registry key"),
                StateValue::new_legacy(vec![tag; 8].into()),
            )
        };
        // The same 0x1 registry, but 0x3 upgraded between the two records.
        write_dump(dump.path(), vec![
            (10, vec![registry(1), token_registry(1)]),
            (20, vec![registry(1), token_registry(2)]),
        ]);
        let chain = FakeChain {
            frameworks: vec![(0, framework(1)), (15, framework(2))],
            ..FakeChain::default()
        };
        let (manifest, report) = import(
            &config(dump.path(), out.path(), FrameworkSource::OnChain),
            Some(&chain),
        )
        .expect("import");
        assert_eq!(report.imported, 2);
        assert_eq!(manifest.num_frameworks, 2);
        assert_eq!(*chain.framework_calls.borrow(), 2);
    }

    #[test]
    fn record_that_differs_from_chain_is_skipped() {
        let dump = tempfile::tempdir().expect("tempdir");
        let out = tempfile::tempdir().expect("tempdir");
        write_dump(dump.path(), vec![
            (10, vec![registry(1)]),
            (11, vec![registry(1)]),
        ]);
        let chain = FakeChain {
            frameworks: vec![(0, framework(1))],
            txns: [(11, txn(99))].into_iter().collect(),
            ..FakeChain::default()
        };
        let (_, report) = import(
            &config(dump.path(), out.path(), FrameworkSource::OnChain),
            Some(&chain),
        )
        .expect("import");
        assert_eq!(report.imported, 1);
        assert_eq!(
            report.skipped.get("transaction differs from chain"),
            Some(&1)
        );
    }

    #[test]
    fn offline_import_uses_head_and_skips_incomplete_closures() {
        let dump = tempfile::tempdir().expect("tempdir");
        let out = tempfile::tempdir().expect("tempdir");
        let (a_key, a) = user_module("a", &["b"]);
        write_dump(dump.path(), vec![
            (10, vec![(a_key, a)]),
            (11, vec![registry(1)]),
        ]);
        assert!(import(
            &config(dump.path(), out.path(), FrameworkSource::OnChain),
            None
        )
        .is_err());

        let (manifest, report) = import(
            &config(dump.path(), out.path(), FrameworkSource::Head),
            None,
        )
        .expect("import");
        assert_eq!(report.imported, 1);
        assert_eq!(report.skipped.get("incomplete module closure"), Some(&1));
        assert_eq!(manifest.num_frameworks, 1);
        let corpus = Corpus::open(out.path()).expect("open corpus");
        let shard = corpus.load_shard(0).expect("load shard");
        assert_eq!(shard.records[0].onchain, None);
        let state = corpus
            .record_state(&shard, &shard.records[0])
            .expect("state");
        assert!(state.len() > 100, "the head framework is attached");
    }

    #[test]
    fn the_framework_set_owns_its_accounts_modules() {
        let dump = tempfile::tempdir().expect("tempdir");
        let out = tempfile::tempdir().expect("tempdir");
        let (a_key, a) = user_module("a", &[]);
        // A `0x7` module the dump recorded but the head framework no longer has.
        let stale = StateKey::module(&AccountAddress::SEVEN, ident_str!("removed_in_head"));
        write_dump(dump.path(), vec![(10, vec![
            (a_key, a),
            (stale.clone(), StateValue::new_legacy(vec![0].into())),
        ])]);
        let (_, report) = import(
            &config(dump.path(), out.path(), FrameworkSource::Head),
            None,
        )
        .expect("import");
        assert_eq!(report.imported, 1);
        let corpus = Corpus::open(out.path()).expect("open corpus");
        let shard = corpus.load_shard(0).expect("load shard");
        let state = corpus
            .record_state(&shard, &shard.records[0])
            .expect("state");
        assert!(!state.contains_key(&stale), "the framework set wins");
        assert!(
            shard.records[0].absent.contains(&stale),
            "and it lacks the module"
        );
    }

    #[test]
    fn cached_reads_make_a_rerun_independent_of_the_chain() {
        let dump = tempfile::tempdir().expect("tempdir");
        let cache = tempfile::tempdir().expect("tempdir");
        let (a_key, a) = user_module("a", &["b"]);
        let (b_key, b) = user_module("b", &[]);
        write_dump(dump.path(), vec![(10, vec![registry(1), (a_key, a)])]);
        let chain = FakeChain {
            frameworks: vec![(0, framework(1))],
            modules: [(b_key, b)].into_iter().collect(),
            ..FakeChain::default()
        };
        let mut first = None;
        for chain in [&chain, &FakeChain {
            fail: true,
            ..FakeChain::default()
        }] {
            let out = tempfile::tempdir().expect("tempdir");
            let mut config = config(dump.path(), out.path(), FrameworkSource::OnChain);
            config.cache_dir = Some(cache.path().to_path_buf());
            let (_, report) = import(&config, Some(chain)).expect("import");
            assert_eq!(report.imported, 1);
            let states = states(&Corpus::open(out.path()).expect("open corpus"));
            match &first {
                None => first = Some(states),
                Some(first) => assert_eq!(&states, first),
            }
        }
    }

    #[test]
    fn the_cache_and_the_corpus_are_never_inside_one_another() {
        let dump = tempfile::tempdir().expect("tempdir");
        write_dump(dump.path(), vec![(10, vec![registry(1)])]);
        let dir = tempfile::tempdir().expect("tempdir");
        let refused = |out: &Path, cache: PathBuf, reason: &str| {
            let mut config = config(dump.path(), out, FrameworkSource::OnChain);
            config.cache_dir = Some(cache);
            let err = import(&config, Some(&FakeChain::default())).expect_err("refused");
            assert!(format!("{:#}", err).contains(reason), "{err:#}");
            // Nothing was written: the claimed directory is removed again.
            assert!(!out.exists(), "{:?} is left", out);
        };
        let inside = "must not be inside one another";
        let out = dir.path().join("corpus");
        // The cache inside `--out`, named directly or through a symlink.
        refused(&out, out.join("cache"), inside);
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(dir.path(), &link).expect("symlink");
        refused(&out, link.join("corpus").join("cache"), inside);
        // Through a symlink that dangles until `--out` is claimed: decided after the claim.
        let late = dir.path().join("late");
        std::os::unix::fs::symlink(&out, &late).expect("symlink");
        refused(&out, late.join("cache"), inside);
        // `--out` inside the cache directory.
        let cache = dir.path().join("cache");
        refused(&cache.join("corpus"), cache.clone(), inside);
        // The chain's cache directory a symlink to `--out`.
        let other = dir.path().join("other");
        std::fs::create_dir(&other).expect("other");
        std::os::unix::fs::symlink(&out, other.join("chain-1")).expect("symlink");
        refused(&out, other, inside);
        // The chain's cache directory a symlink to a directory `--out` is inside.
        let real = dir.path().join("real");
        std::fs::create_dir(&real).expect("real");
        let third = dir.path().join("third");
        std::fs::create_dir(&third).expect("third");
        std::os::unix::fs::symlink(&real, third.join("chain-1")).expect("symlink");
        refused(&real.join("committed"), third, inside);
        // A `..` after a name is refused, as for `--out`, and creates nothing.
        refused(
            &out,
            dir.path().join("x").join("..").join("cache"),
            "has a `..`",
        );
        assert!(!dir.path().join("x").exists());
        // A cache directory that is a symlink to nothing cannot be created: refused, not silently
        // left unused.
        let dangling = dir.path().join("dangling");
        std::os::unix::fs::symlink(dir.path().join("gone"), &dangling).expect("symlink");
        refused(&out, dangling, "failed to create the cache directory");
    }

    #[test]
    fn an_unreadable_cache_entry_is_fetched_again() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cache = Cache::scoped(Some(dir.path().to_path_buf()), "chain-1");
        let entry = dir.path().join("chain-1").join("state").join("1");
        std::fs::create_dir_all(&entry).expect("entry dir");
        std::fs::write(entry.join("a"), b"").expect("truncated entry");
        assert_eq!(cache.get::<u64>("state/1/a"), None);
        assert_eq!(
            cache.get_or_fetch("state/1/a", || Ok(7u64)).expect("fetch"),
            7
        );
        assert_eq!(cache.get::<u64>("state/1/a"), Some(7));
    }

    #[test]
    fn a_cache_that_cannot_be_written_skips_no_record() {
        let dump = tempfile::tempdir().expect("tempdir");
        let (a_key, a) = user_module("a", &["b"]);
        let (b_key, b) = user_module("b", &[]);
        write_dump(dump.path(), vec![(10, vec![registry(1), (a_key, a)])]);
        let chain = FakeChain {
            frameworks: vec![(0, framework(1))],
            modules: [(b_key, b)].into_iter().collect(),
            ..FakeChain::default()
        };
        // Files where the cache's directories should be: no entry can be written, even by root.
        let dir = tempfile::tempdir().expect("tempdir");
        let cache = dir.path().join("cache");
        let chain_dir = cache.join("chain-1");
        std::fs::create_dir_all(&chain_dir).expect("cache");
        for name in ["committed", "state", "frameworks"] {
            std::fs::write(chain_dir.join(name), b"not a directory").expect("file");
        }
        let out = dir.path().join("corpus");
        let mut config = config(dump.path(), &out, FrameworkSource::OnChain);
        config.cache_dir = Some(cache);
        let (_, report) = import(&config, Some(&chain)).expect("import");
        assert_eq!(report.imported, 1);
        assert!(Corpus::open(&out).is_ok());
    }

    #[test]
    fn only_failures_that_may_pass_are_retried() {
        let api_error = |status: u16, error_code: AptosErrorCode| -> anyhow::Error {
            RestError::from((
                aptos_rest_client::aptos_api_types::AptosError {
                    message: String::new(),
                    error_code,
                    vm_error_code: None,
                },
                None,
                reqwest::StatusCode::from_u16(status).expect("status"),
            ))
            .into()
        };
        // As the API reports them: a pruned version, a bad request, a node behind the version, a rate
        // limit and an unavailable node.
        for (status, error_code, attempts) in [
            (410, AptosErrorCode::VersionPruned, 1),
            (400, AptosErrorCode::InvalidInput, 1),
            (404, AptosErrorCode::VersionNotFound, 4),
            (429, AptosErrorCode::WebFrameworkError, 4),
            (503, AptosErrorCode::InternalError, 4),
        ] {
            let mut calls = 0;
            let result: Result<()> = retrying(|| {
                calls += 1;
                Err(api_error(status, error_code)).context("reading state")
            });
            assert!(result.is_err());
            assert_eq!(calls, attempts, "status {}", status);
        }
    }

    #[test]
    fn a_cache_directory_linked_out_is_not_written_through() {
        let dir = tempfile::tempdir().expect("tempdir");
        let victim = dir.path().join("victim");
        std::fs::create_dir_all(&victim).expect("victim");
        std::fs::write(victim.join("0.bcs"), b"keep").expect("victim file");
        let root = dir.path().join("cache").join("chain-1");
        std::fs::create_dir_all(&root).expect("root");
        std::os::unix::fs::symlink(&victim, root.join("committed")).expect("symlink");
        let cache = Cache::scoped(Some(dir.path().join("cache")), "chain-1");
        assert!(cache.try_put("committed/0.bcs", &1u64).is_err());
        assert_eq!(
            std::fs::read(victim.join("0.bcs")).expect("victim file"),
            b"keep"
        );
    }

    #[test]
    fn a_cache_entry_is_never_written_through_a_symlink() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cache = Cache::scoped(Some(dir.path().join("cache")), "chain-1");
        let victim = dir.path().join("victim");
        std::fs::write(&victim, b"keep").expect("victim");
        let entries = dir.path().join("cache").join("chain-1").join("committed");
        std::fs::create_dir_all(&entries).expect("entries");
        std::os::unix::fs::symlink(
            &victim,
            entries.join(format!("0.{}.tmp", std::process::id())),
        )
        .expect("symlink");
        cache.try_put("committed/0.bcs", &1u64).expect("put");
        assert_eq!(std::fs::read(&victim).expect("victim"), b"keep");
        assert_eq!(cache.get::<u64>("committed/0.bcs"), Some(1));
    }

    /// A chain whose `0x1` registry changes at `upgrade_at`, counting registry reads.
    struct UpgradingChain {
        upgrade_at: Version,
        registry_reads: RefCell<usize>,
        /// State before this version fails to read, as on a node that pruned it.
        pruned_below: Version,
    }

    impl ChainSource for UpgradingChain {
        fn framework_at(&self, version: Version) -> Result<BTreeMap<StateKey, StateValue>> {
            Ok(framework(u8::from(version >= self.upgrade_at)))
        }

        fn state_value_at(&self, key: &StateKey, version: Version) -> Result<Option<StateValue>> {
            *self.registry_reads.borrow_mut() += 1;
            ensure!(version >= self.pruned_below, "version {} pruned", version);
            let (registry_key, _) = registry(0);
            Ok((*key == registry_key).then(|| registry(u8::from(version >= self.upgrade_at)).1))
        }

        fn identity(&self) -> Result<String> {
            Ok("chain-1".to_string())
        }

        fn latest_version(&self) -> Result<Version> {
            Ok(Version::MAX)
        }

        fn committed(&self, _: &[(Version, u64)]) -> Vec<Result<Vec<CommittedTxn>>> {
            vec![]
        }
    }

    #[test]
    fn a_failed_release_lookup_fails_only_its_versions() {
        let versions: Vec<Version> = (100..200).collect();
        let chain = UpgradingChain {
            upgrade_at: 1_000,
            registry_reads: RefCell::new(0),
            pruned_below: 103,
        };
        let releases = Releases::default().at_each(&chain, &Cache::none(), &versions);
        for (release, version) in releases.iter().zip(&versions) {
            assert_eq!(release.is_err(), *version < 103, "{}", version);
        }
        let first = releases[3]
            .as_ref()
            .expect("a lookup")
            .as_ref()
            .expect("a release")
            .0;
        assert!(releases[3..]
            .iter()
            .all(|release| matches!(release, Ok(Some((id, _))) if *id == first)));
    }

    #[test]
    fn releases_are_resolved_from_the_ends_of_runs() {
        let versions: Vec<Version> = (100..200).collect();
        // No upgrade in the window: one lookup at each end (three registries each), whatever its
        // length.
        let chain = UpgradingChain {
            upgrade_at: 1_000,
            registry_reads: RefCell::new(0),
            pruned_below: 0,
        };
        let releases = Releases::default()
            .at_each(&chain, &Cache::none(), &versions)
            .into_iter()
            .collect::<Result<Vec<_>>>()
            .expect("releases");
        assert_eq!(*chain.registry_reads.borrow(), 6);
        let first = releases[0].as_ref().expect("a release").0;
        assert!(releases
            .iter()
            .all(|release| release.as_ref().map(|(id, _)| *id) == Some(first)));
        // An upgrade inside it: each version gets the release it ran against, as a lookup at every
        // version would give, for a few more lookups.
        let chain = UpgradingChain {
            upgrade_at: 137,
            registry_reads: RefCell::new(0),
            pruned_below: 0,
        };
        let mut releases = Releases::default();
        let resolved = releases
            .at_each(&chain, &Cache::none(), &versions)
            .into_iter()
            .collect::<Result<Vec<_>>>()
            .expect("releases");
        for (release, &version) in resolved.iter().zip(&versions) {
            let expected = releases
                .at(&chain, &Cache::none(), &BTreeMap::new(), version)
                .expect("release")
                .map(|(id, _)| id);
            assert_eq!(release.as_ref().map(|(id, _)| *id), expected, "{}", version);
        }
        let (before, after) = (&resolved[36], &resolved[37]);
        assert_ne!(
            before.as_ref().map(|(id, _)| *id),
            after.as_ref().map(|(id, _)| *id)
        );
    }

    #[test]
    fn pages_fetched_before_a_failed_one_are_cached() {
        let cache_dir = tempfile::tempdir().expect("tempdir");
        let cache = Cache::scoped(Some(cache_dir.path().to_path_buf()), "chain-1");
        let chain = FakeChain {
            failing_range: Some(100),
            ..FakeChain::default()
        };
        assert!(fetch_committed(&chain, &cache, &[10, 110, 210]).is_err());
        let cached = |start| cache.get::<Vec<CommittedTxn>>(&page_path(start)).is_some();
        assert!(cached(0) && !cached(100) && cached(200));
    }

    #[test]
    fn last_partial_page_is_clamped_and_not_cached() {
        let cache_dir = tempfile::tempdir().expect("tempdir");
        let cache = Cache::scoped(Some(cache_dir.path().to_path_buf()), "chain-1");
        let chain = FakeChain {
            latest: Some(150),
            ..FakeChain::default()
        };
        let committed = fetch_committed(&chain, &cache, &[120, 160]).expect("fetch");
        // 160 is past the ledger; 120 comes from a 51-transaction page that is not final.
        assert_eq!(committed.keys().copied().collect::<Vec<_>>(), vec![120]);
        assert!(cache.get::<Vec<CommittedTxn>>(&page_path(100)).is_none());
    }

    #[test]
    fn without_a_cache_only_the_runs_of_wanted_versions_are_fetched() {
        let chain = FakeChain::default();
        let committed =
            fetch_committed(&chain, &Cache::none(), &[5, 6, 7, 50, 300]).expect("fetch");
        assert_eq!(committed.len(), 5);
        assert_eq!(*chain.page_calls.borrow(), 3);
        assert_eq!(
            contiguous_runs(&[5, 6, 7, 50, 300].into_iter().collect(), 2),
            vec![(5, 2), (7, 1), (50, 1), (300, 1)]
        );
    }

    #[test]
    fn cache_is_scoped_by_chain() {
        let dir = tempfile::tempdir().expect("tempdir");
        let fork_a = Cache::scoped(Some(dir.path().to_path_buf()), "chain-1-a");
        let fork_b = Cache::scoped(Some(dir.path().to_path_buf()), "chain-1-b");
        fork_a.try_put("committed/0.bcs", &1u64).expect("put");
        assert_eq!(fork_a.get::<u64>("committed/0.bcs"), Some(1));
        assert_eq!(fork_b.get::<u64>("committed/0.bcs"), None);
    }
}
