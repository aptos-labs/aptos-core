// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The on-disk corpus: captured transactions plus the state each one reads, stored so that values
//! shared across transactions are kept once.
//!
//! ```text
//! <corpus>/
//!   manifest.json    # identity, origin, shard index
//!   modules.pack     # every distinct module value, plus the framework sets
//!   shards/<n>.bcs   # a batch of records plus the distinct non-module values they read
//! ```
//!
//! Module values are content-addressed across the whole corpus. A framework set (the modules at
//! `0x1`, `0x3` and `0x4` for one framework release) is stored once and referenced by id, so a
//! record does not list every framework module it may load.
//! Non-module values are pooled per shard: hot values such as `0x1::code::PackageRegistry` are
//! read by nearly every transaction, and pooling them keeps each shard self-contained.
//!
//! A value is identified by the SHA3-256 of its full BCS serialization, metadata included, since
//! metadata is part of what the VM sees. The state is stored exactly as read from chain: overrides
//! are applied at replay time, so a corpus survives changes to the override set.
//!
//! A published corpus is immutable: the writer refuses a directory that already has a manifest.

use anyhow::{bail, ensure, Context, Result};
use aptos_crypto::HashValue;
use aptos_types::{
    state_store::{
        state_key::{inner::StateKeyInner, StateKey},
        state_value::StateValue,
    },
    transaction::{ExecutionStatus, PersistedAuxiliaryInfo, Transaction, Version},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::{Component, Path, PathBuf},
};

pub const FORMAT_VERSION: u32 = 2;
pub const MANIFEST_FILE: &str = "manifest.json";
pub const MODULES_FILE: &str = "modules.pack";
pub const SHARDS_DIR: &str = "shards";

/// The SHA3-256 of `value`'s BCS serialization; `what` names it in an error.
pub(crate) fn bcs_sha3<T: Serialize + ?Sized>(value: &T, what: &str) -> Result<HashValue> {
    let bytes = bcs::to_bytes(value).with_context(|| format!("failed to serialize {}", what))?;
    Ok(HashValue::sha3_256_of(&bytes))
}

/// The content hash of a state value: SHA3-256 of its full BCS serialization.
pub fn value_hash(value: &StateValue) -> Result<HashValue> {
    bcs_sha3(value, "state value")
}

/// Whether the key holds a module (and so belongs in the module pack).
pub fn is_module_key(key: &StateKey) -> bool {
    matches!(key.inner(), StateKeyInner::AccessPath(ap) if ap.is_code())
}

/// What the chain recorded for the transaction. Only the status is comparable at replay time:
/// the replay runs gas-free and without state metadata, so its write set never matches chain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnChainInfo {
    pub status: ExecutionStatus,
    pub gas_used: u64,
}

/// One transaction and the state it reads, with values stored by reference.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TxnRecord {
    pub version: Version,
    pub txn: Transaction,
    pub aux_info: PersistedAuxiliaryInfo,
    /// Non-module reads, as indices into the shard's value pool. Sorted by key.
    pub state: Vec<(StateKey, u32)>,
    /// Module reads, as hashes into the corpus module pack. Sorted by key.
    pub modules: Vec<(StateKey, HashValue)>,
    /// A framework set in the module pack, layered under `modules` and `state`.
    pub framework: Option<HashValue>,
    /// Keys read and found absent, so a replay that reads them can trust the absence. A key
    /// neither here nor in the state was never observed. Sorted.
    pub absent: Vec<StateKey>,
    /// `None` until filled in, e.g. for records imported from a source that did not store it.
    pub onchain: Option<OnChainInfo>,
}

/// A batch of records and the distinct non-module values they read.
#[derive(Debug, Serialize, Deserialize)]
pub struct Shard {
    pub values: Vec<StateValue>,
    pub records: Vec<TxnRecord>,
}

/// Which framework the records of a legacy dump are paired with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameworkSource {
    /// The on-chain framework at each record's version.
    OnChain,
    /// The framework this binary was built with.
    Head,
}

/// Where a corpus came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Origin {
    /// Captured from chain by this tool.
    Capture,
    /// Converted from a dump written by `aptos-e2e-comparison-testing`.
    LegacyDump {
        source: String,
        era: crate::legacy::LegacyEra,
        framework: FrameworkSource,
    },
}

/// The module pack file: distinct module values and the framework sets that reference them.
#[derive(Debug, Default, Serialize, Deserialize)]
struct ModulePack {
    modules: Vec<StateValue>,
    /// Each set is sorted by key; its id is the SHA3-256 of its BCS serialization.
    frameworks: Vec<Vec<(StateKey, HashValue)>>,
}

fn framework_id(set: &[(StateKey, HashValue)]) -> Result<HashValue> {
    bcs_sha3(set, "framework set")
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShardInfo {
    /// Path relative to the corpus root.
    pub file: String,
    pub num_records: usize,
    pub first_version: Version,
    pub last_version: Version,
    /// SHA3-256 of the shard file, checked on load.
    pub sha3: HashValue,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub format_version: u32,
    pub corpus_id: String,
    pub network: String,
    pub origin: Origin,
    pub num_modules: usize,
    pub num_frameworks: usize,
    pub shards: Vec<ShardInfo>,
}

impl Manifest {
    pub fn num_records(&self) -> usize {
        self.shards.iter().map(|s| s.num_records).sum()
    }
}

/// A transaction to add to a corpus, with its read set as plain values.
pub struct RecordInput {
    pub version: Version,
    pub txn: Transaction,
    pub aux_info: PersistedAuxiliaryInfo,
    pub state: BTreeMap<StateKey, StateValue>,
    /// A framework set previously added with [`CorpusWriter::add_framework`].
    pub framework: Option<HashValue>,
    /// Keys read and found absent; empty when the source did not record them.
    pub absent: BTreeSet<StateKey>,
    pub onchain: Option<OnChainInfo>,
}

/// Accumulates one shard's records, pooling their non-module values.
#[derive(Default)]
struct ShardBuilder {
    values: Vec<StateValue>,
    index: HashMap<HashValue, u32>,
    records: Vec<TxnRecord>,
}

impl ShardBuilder {
    fn intern(&mut self, value: StateValue) -> Result<u32> {
        let hash = value_hash(&value)?;
        if let Some(idx) = self.index.get(&hash) {
            return Ok(*idx);
        }
        let idx = u32::try_from(self.values.len()).context("shard value pool overflow")?;
        self.values.push(value);
        self.index.insert(hash, idx);
        Ok(idx)
    }
}

/// Writes a new corpus. Records must be added in increasing version order.
pub struct CorpusWriter {
    root: PathBuf,
    corpus_id: String,
    network: String,
    origin: Origin,
    shard_size: usize,
    modules: BTreeMap<HashValue, StateValue>,
    frameworks: BTreeMap<HashValue, Vec<(StateKey, HashValue)>>,
    pending: ShardBuilder,
    shards: Vec<ShardInfo>,
    last_version: Option<Version>,
    /// The directories this run created, outermost first: `root` and its missing parents, or none
    /// if it claimed an empty directory that was there.
    created: Vec<PathBuf>,
}

/// Refuses a directory `path` with a `..` after a name: creating its missing parents would create
/// a directory just to leave it (`a/missing/../b` creates `a/missing`), and where it leads can
/// change once they exist. A `..` may only lead the path.
pub fn ensure_no_inner_parent(path: &Path) -> Result<()> {
    let mut components = path.components().skip_while(|c| {
        matches!(
            c,
            Component::Prefix(_) | Component::RootDir | Component::CurDir | Component::ParentDir
        )
    });
    ensure!(
        components.all(|c| matches!(c, Component::Normal(_))),
        "{:?} has a `..` after a directory name; name the directory without one",
        path
    );
    Ok(())
}

/// Makes `root` a directory the corpus has to itself: creates it, or accepts it if it is an empty
/// directory (not a symlink). Anything else is refused: a published corpus, a directory holding
/// other files, a symlink, or `.` naming one of them; so is a path with a `..` after a name. So a
/// corpus never overwrites another, and if its run stops, moving its directory moves only its own.
/// Creating is atomic, so that check cannot be raced; an empty directory that was there is taken as
/// the run's, so nothing else may write into it while the run lasts. Missing parents are created
/// one at a time, and the directories created are returned, outermost first, so that exactly
/// those can be removed again; if claiming fails, they are.
fn claim_dir(root: &Path) -> Result<Vec<PathBuf>> {
    ensure_no_inner_parent(root)?;
    let mut created = vec![];
    let parents: Vec<&Path> = root
        .ancestors()
        .skip(1)
        .take_while(|dir| !dir.as_os_str().is_empty() && std::fs::symlink_metadata(dir).is_err())
        .collect();
    for dir in parents.into_iter().rev().chain([root]) {
        match std::fs::create_dir(dir) {
            Ok(()) => created.push(dir.to_path_buf()),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists && dir == root => {},
            Err(err) => {
                remove_created(&created);
                return Err(err).with_context(|| format!("failed to create corpus dir {:?}", dir));
            },
        }
    }
    if created.last().is_some_and(|dir| dir == root) {
        return Ok(created);
    }
    // `root` was there: it must be an empty directory.
    let metadata =
        std::fs::symlink_metadata(root).with_context(|| format!("failed to inspect {:?}", root))?;
    let empty = metadata.is_dir()
        && std::fs::read_dir(root)
            .with_context(|| format!("failed to list {:?}", root))?
            .next()
            .is_none();
    ensure!(
        empty,
        "{:?} already exists and is not an empty directory; a corpus is written to a new one (a \
         published corpus is immutable)",
        root
    );
    Ok(created)
}

/// Removes the directories a run `created` (outermost first), innermost first, as long as they are
/// empty: only empty directories are removed, so nothing else can be. Returns whether the
/// innermost is gone.
fn remove_created(created: &[PathBuf]) -> bool {
    for (n, dir) in created.iter().rev().enumerate() {
        if std::fs::remove_dir(dir).is_err() {
            return n > 0;
        }
    }
    true
}

/// The file in a corpus a stopped run leaves, saying why: the corpus holds only the records written
/// before the run stopped.
pub const STOPPED_FILE: &str = "STOPPED";

/// A free sibling of `root` to keep a stopped run's records in: `<name>.partial`, or
/// `<name>.partial.<n>` if a run stopped there before. Named from `root`'s own name, so that
/// `out/` and `.` work too.
fn partial_path(root: &Path) -> PathBuf {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let name = root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "corpus".to_string());
    let parent = root.parent().unwrap_or(Path::new("."));
    let mut candidate = parent.join(format!("{}.partial", name));
    let mut n = 1;
    // `symlink_metadata`, so that a dangling symlink counts as taken.
    while std::fs::symlink_metadata(&candidate).is_ok() {
        candidate = parent.join(format!("{}.partial.{}", name, n));
        n += 1;
    }
    candidate
}

impl CorpusWriter {
    /// Finishes the corpus with the records written so far, for a run stopped by `err` while
    /// `what`, marks it with a [`STOPPED_FILE`], and moves it to `<dir>.partial`: the work done is
    /// kept, the marker and the name say the run did not finish, and `--out` is free for a run that
    /// does. The directory is the corpus's own (see [`claim_dir`]), so moving it moves nothing else.
    /// If it cannot be marked, the records are still written but the manifest is withheld, so that
    /// they cannot pass for a complete corpus. If the pending records cannot be written, they are
    /// given up, so that the shards written before them are still published. Returns `err`, saying
    /// where the records are.
    pub fn finish_after(mut self, err: anyhow::Error, what: &str) -> anyhow::Error {
        let root = self.root.clone();
        // A run that stopped before writing anything (its directory holds only the empty `shards`)
        // leaves nothing: the directories it created are removed, or the one it claimed is emptied
        // again, so that `--out` is as it was. Only empty directories are removed, so nothing else
        // can be.
        let untouched = self.shards.is_empty()
            && self.pending.records.is_empty()
            && std::fs::read_dir(&root).is_ok_and(|entries| {
                entries
                    .map(|entry| entry.map(|entry| entry.file_name()))
                    .collect::<std::io::Result<Vec<_>>>()
                    .is_ok_and(|names| names == [SHARDS_DIR])
            });
        let left = untouched
            && std::fs::remove_dir(root.join(SHARDS_DIR)).is_ok()
            && remove_created(&self.created);
        if left {
            return err.context(format!(
                "{}; the run stopped before writing any record, and left {:?} as it was",
                what, root
            ));
        }
        // The pending records first, so that the marker can say if they were given up.
        let lost = match self.flush_shard() {
            Ok(()) => String::new(),
            Err(flush_err) => {
                let lost = std::mem::take(&mut self.pending).records.len();
                format!(
                    "; the last {} records could not be written: {:#}",
                    lost, flush_err
                )
            },
        };
        let marked = std::fs::write(
            root.join(STOPPED_FILE),
            format!("the run stopped while {}: {:#}{}\n", what, err, lost),
        );
        let written = match marked {
            Ok(()) => self.publish(),
            Err(_) => self.write_records(),
        };
        // Even if writing fails, the directory is moved aside below, so that `--out` is free.
        let kept_records = match (&written, &marked) {
            (Ok(manifest), Ok(())) => {
                format!(
                    "the {} records written so far were kept",
                    manifest.num_records()
                )
            },
            (Ok(manifest), Err(mark_err)) => format!(
                "the {} records written so far were kept, unpublished since the corpus could not \
                 be marked as stopped ({})",
                manifest.num_records(),
                mark_err
            ),
            (Err(write_err), _) => format!(
                "writing the records failed too ({:#}); what was written is",
                write_err
            ),
        };
        // The working directory (e.g. `--out .` while empty) cannot be moved: it stays, marked.
        let in_use = std::env::current_dir()
            .is_ok_and(|cwd| root.canonicalize().is_ok_and(|real| cwd.starts_with(real)));
        let partial = partial_path(&root);
        let kept = if in_use {
            format!("{:?}", root)
        } else {
            match std::fs::rename(&root, &partial) {
                Ok(()) => format!("{:?}", partial),
                Err(rename_err) => format!(
                    "{:?} (moving it to {:?} failed: {})",
                    root, partial, rename_err
                ),
            }
        };
        err.context(format!(
            "{}; the run stopped, and {} in {}{}",
            what, kept_records, kept, lost
        ))
    }

    pub fn create(
        root: impl AsRef<Path>,
        corpus_id: impl Into<String>,
        network: impl Into<String>,
        origin: Origin,
        shard_size: usize,
    ) -> Result<Self> {
        ensure!(shard_size > 0, "shard size must be at least 1");
        // Normalized, so that `out/` or `out/.` name `out` itself: a trailing separator would make
        // a symlink look like its target.
        let root: PathBuf = root.as_ref().components().collect();
        let created = claim_dir(&root)?;
        if let Err(err) = std::fs::create_dir(root.join(SHARDS_DIR)) {
            remove_created(&created);
            return Err(err).with_context(|| format!("failed to create corpus dir {:?}", root));
        }
        Ok(Self {
            root,
            corpus_id: corpus_id.into(),
            network: network.into(),
            origin,
            shard_size,
            modules: BTreeMap::new(),
            frameworks: BTreeMap::new(),
            pending: ShardBuilder::default(),
            shards: vec![],
            last_version: None,
            created,
        })
    }

    /// Adds a framework set and returns its id, for [`RecordInput::framework`]. Adding the same set
    /// twice is a no-op.
    pub fn add_framework(&mut self, modules: BTreeMap<StateKey, StateValue>) -> Result<HashValue> {
        let mut set = Vec::with_capacity(modules.len());
        for (key, value) in modules {
            ensure!(
                is_module_key(&key),
                "{:?} in a framework set is not a module",
                key
            );
            let hash = value_hash(&value)?;
            self.modules.entry(hash).or_insert(value);
            set.push((key, hash));
        }
        let id = framework_id(&set)?;
        self.frameworks.entry(id).or_insert(set);
        Ok(id)
    }

    pub fn add(&mut self, input: RecordInput) -> Result<()> {
        if let Some(framework) = input.framework {
            ensure!(
                self.frameworks.contains_key(&framework),
                "version {}: framework set {} was not added",
                input.version,
                framework
            );
        }
        if let Some(last) = self.last_version {
            ensure!(
                input.version > last,
                "records must be added in increasing version order: {} after {}",
                input.version,
                last
            );
        }
        self.last_version = Some(input.version);

        let mut state = vec![];
        let mut modules = vec![];
        // `BTreeMap` iteration keeps both lists sorted by key.
        for (key, value) in input.state {
            if is_module_key(&key) {
                let hash = value_hash(&value)?;
                self.modules.entry(hash).or_insert(value);
                modules.push((key, hash));
            } else {
                let idx = self.pending.intern(value)?;
                state.push((key, idx));
            }
        }
        self.pending.records.push(TxnRecord {
            version: input.version,
            txn: input.txn,
            aux_info: input.aux_info,
            state,
            modules,
            framework: input.framework,
            absent: input.absent.into_iter().collect(),
            onchain: input.onchain,
        });
        if self.pending.records.len() >= self.shard_size {
            self.flush_shard()?;
        }
        Ok(())
    }

    /// Writes the pending records as the next shard. They stay pending until it is written, so a
    /// failed write loses nothing: a later flush (e.g. by `finish_after`) writes them again.
    fn flush_shard(&mut self) -> Result<()> {
        let builder = &self.pending;
        let (Some(first), Some(last)) = (builder.records.first(), builder.records.last()) else {
            return Ok(());
        };
        let (first_version, last_version) = (first.version, last.version);
        let num_records = builder.records.len();
        // The bytes of a `Shard`: BCS encodes a struct as its fields in order.
        let bytes = bcs::to_bytes(&(&builder.values, &builder.records))
            .context("failed to serialize shard")?;
        let file = format!("{}/{}.bcs", SHARDS_DIR, self.shards.len());
        std::fs::write(self.root.join(&file), &bytes)
            .with_context(|| format!("failed to write shard {}", file))?;
        self.pending = ShardBuilder::default();
        self.shards.push(ShardInfo {
            file,
            num_records,
            first_version,
            last_version,
            sha3: HashValue::sha3_256_of(&bytes),
        });
        Ok(())
    }

    /// Writes the last shard, the module pack and the manifest. The manifest goes last, so a
    /// directory with a manifest always holds a complete corpus. If writing fails, the run stops
    /// as with [`Self::finish_after`], so that the records written are kept and `--out` is free.
    pub fn finish(mut self) -> Result<Manifest> {
        match self.publish() {
            Ok(manifest) => Ok(manifest),
            Err(err) => Err(self.finish_after(err, "finishing the corpus")),
        }
    }

    /// Writes the records and the manifest that publishes them.
    fn publish(&mut self) -> Result<Manifest> {
        let manifest = self.write_records()?;
        let json = serde_json::to_vec_pretty(&manifest).context("failed to serialize manifest")?;
        std::fs::write(self.root.join(MANIFEST_FILE), json).context("failed to write manifest")?;
        Ok(manifest)
    }

    /// Writes the shards and the module pack, and returns the manifest that would publish them,
    /// without writing it: without a manifest, the directory is not a corpus. The module pack goes
    /// first, so that the shards written before a failure here stay readable. Nothing is consumed,
    /// so a failed write can be tried again.
    fn write_records(&mut self) -> Result<Manifest> {
        let modules: Vec<_> = self.modules.values().collect();
        let frameworks: Vec<_> = self.frameworks.values().collect();
        let (num_modules, num_frameworks) = (modules.len(), frameworks.len());
        // The bytes of a `ModulePack`: BCS encodes a struct as its fields in order.
        let bytes =
            bcs::to_bytes(&(modules, frameworks)).context("failed to serialize module pack")?;
        std::fs::write(self.root.join(MODULES_FILE), bytes).context("failed to write modules")?;
        self.flush_shard()?;
        Ok(Manifest {
            format_version: FORMAT_VERSION,
            corpus_id: self.corpus_id.clone(),
            network: self.network.clone(),
            origin: self.origin.clone(),
            num_modules,
            num_frameworks,
            shards: self.shards.clone(),
        })
    }
}

/// A corpus opened for reading. Holds the manifest and the module pack; shards load on demand.
pub struct Corpus {
    root: PathBuf,
    manifest: Manifest,
    modules: HashMap<HashValue, StateValue>,
    frameworks: HashMap<HashValue, Vec<(StateKey, HashValue)>>,
}

impl Corpus {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let json = std::fs::read(root.join(MANIFEST_FILE))
            .with_context(|| format!("failed to read manifest in {:?}", root))?;
        let manifest: Manifest =
            serde_json::from_slice(&json).context("failed to decode manifest")?;
        ensure!(
            manifest.format_version == FORMAT_VERSION,
            "unsupported corpus format version {} (expected {})",
            manifest.format_version,
            FORMAT_VERSION
        );
        // Shards live in the corpus's shard directory, which the guards on outputs check, never
        // elsewhere.
        for shard in &manifest.shards {
            let components: Vec<_> = Path::new(&shard.file).components().collect();
            ensure!(
                matches!(
                    components.as_slice(),
                    [Component::Normal(dir), Component::Normal(_)] if *dir == SHARDS_DIR
                ),
                "shard {:?} is not a file in the corpus's {} directory",
                shard.file,
                SHARDS_DIR
            );
        }
        let bytes = std::fs::read(root.join(MODULES_FILE)).context("failed to read modules")?;
        let pack: ModulePack = bcs::from_bytes(&bytes).context("failed to decode module pack")?;
        let mut modules = HashMap::with_capacity(pack.modules.len());
        for value in pack.modules {
            modules.insert(value_hash(&value)?, value);
        }
        let mut frameworks = HashMap::with_capacity(pack.frameworks.len());
        for set in pack.frameworks {
            if let Some((key, hash)) = set.iter().find(|(_, hash)| !modules.contains_key(hash)) {
                bail!("framework module {:?} ({}) is not in the pack", key, hash);
            }
            frameworks.insert(framework_id(&set)?, set);
        }
        ensure!(
            modules.len() == manifest.num_modules && frameworks.len() == manifest.num_frameworks,
            "module pack holds {} modules and {} frameworks, manifest says {} and {}",
            modules.len(),
            frameworks.len(),
            manifest.num_modules,
            manifest.num_frameworks
        );
        Ok(Self {
            root,
            manifest,
            modules,
            frameworks,
        })
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn load_shard(&self, n: usize) -> Result<Shard> {
        let info = self
            .manifest
            .shards
            .get(n)
            .with_context(|| format!("no shard {} in corpus {}", n, self.manifest.corpus_id))?;
        let bytes = std::fs::read(self.root.join(&info.file))
            .with_context(|| format!("failed to read shard {}", info.file))?;
        ensure!(
            HashValue::sha3_256_of(&bytes) == info.sha3,
            "shard {} does not match its manifest hash",
            info.file
        );
        let shard: Shard = bcs::from_bytes(&bytes)
            .with_context(|| format!("failed to decode shard {}", info.file))?;
        ensure!(
            shard.records.len() == info.num_records,
            "shard {} holds {} records, manifest says {}",
            info.file,
            shard.records.len(),
            info.num_records
        );
        Ok(shard)
    }

    /// The full state of `record`, which must come from `shard`: its framework set, overlaid with
    /// what the record itself read.
    pub fn record_state(
        &self,
        shard: &Shard,
        record: &TxnRecord,
    ) -> Result<HashMap<StateKey, StateValue>> {
        let framework_len = record
            .framework
            .as_ref()
            .and_then(|id| self.frameworks.get(id))
            .map_or(0, Vec::len);
        let mut state =
            HashMap::with_capacity(record.state.len() + record.modules.len() + framework_len);
        if let Some(id) = &record.framework {
            let set = self.frameworks.get(id).with_context(|| {
                format!(
                    "version {}: framework {} not in the pack",
                    record.version, id
                )
            })?;
            for (key, hash) in set {
                // Every set member was checked against the pack on open.
                state.insert(key.clone(), self.modules[hash].clone());
            }
        }
        for (key, idx) in &record.state {
            let value = shard.values.get(*idx as usize).with_context(|| {
                format!(
                    "version {}: value index {} out of range",
                    record.version, idx
                )
            })?;
            state.insert(key.clone(), value.clone());
        }
        for (key, hash) in &record.modules {
            let value = self.modules.get(hash).with_context(|| {
                format!(
                    "version {}: module {} not in the pack",
                    record.version, hash
                )
            })?;
            state.insert(key.clone(), value.clone());
        }
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_types::{
        on_chain_config::CurrentTimeMicroseconds, state_store::state_value::StateValueMetadata,
    };
    use move_core_types::{
        account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
    };

    fn txn() -> Transaction {
        Transaction::StateCheckpoint(HashValue::zero())
    }

    fn module_key(name: &str) -> StateKey {
        StateKey::module_id(&ModuleId::new(
            AccountAddress::ONE,
            Identifier::new(name).expect("valid identifier"),
        ))
    }

    fn input(version: Version, state: Vec<(StateKey, StateValue)>) -> RecordInput {
        RecordInput {
            version,
            txn: txn(),
            aux_info: PersistedAuxiliaryInfo::None,
            state: state.into_iter().collect(),
            framework: None,
            absent: BTreeSet::new(),
            onchain: Some(OnChainInfo {
                status: ExecutionStatus::Success,
                gas_used: 7,
            }),
        }
    }

    fn origin() -> Origin {
        Origin::Capture
    }

    /// Asserts that `create` refuses `out` as not a directory of the corpus's own.
    fn assert_refused(out: impl AsRef<Path>) {
        let out = out.as_ref();
        let err = CorpusWriter::create(out, "test", "mainnet", origin(), 10)
            .err()
            .unwrap_or_else(|| panic!("{:?} was accepted", out));
        assert!(
            format!("{:#}", err).contains("is not an empty directory"),
            "{:?}: {err:#}",
            out
        );
    }

    #[test]
    fn out_is_a_new_or_empty_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        // A published corpus, a directory holding other files or a marker, and a symlink, even to
        // an empty directory, are refused, and left as they were.
        let published = dir.path().join("published");
        CorpusWriter::create(&published, "test", "mainnet", origin(), 10)
            .expect("create writer")
            .finish()
            .expect("finish");
        let crowded = dir.path().join("crowded");
        std::fs::create_dir(&crowded).expect("crowded");
        std::fs::write(crowded.join("theirs"), b"keep").expect("theirs");
        let marked = dir.path().join("marked");
        std::fs::create_dir_all(marked.join(STOPPED_FILE)).expect("marked");
        let empty = dir.path().join("empty");
        std::fs::create_dir(&empty).expect("empty");
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&empty, &link).expect("symlink");
        for out in [&published, &crowded, &marked, &link] {
            assert_refused(out);
        }
        assert_refused(format!("{}/", link.display()));
        // A `..` after a name is refused before anything is created, which would otherwise create
        // `published/missing`.
        for out in [
            published.join("missing").join(".."),
            published.join("missing").join("..").join("x"),
            dir.path().join("missing").join("..").join("published"),
        ] {
            let err = CorpusWriter::create(&out, "test", "mainnet", origin(), 10)
                .err()
                .unwrap_or_else(|| panic!("{:?} was accepted", out));
            assert!(format!("{:#}", err).contains("has a `..`"), "{err:#}");
        }
        assert!(!published.join("missing").exists());
        assert!(!dir.path().join("missing").exists());
        assert!(std::fs::symlink_metadata(&link).expect("link").is_symlink());
        assert!(crowded.join("theirs").exists());
        // An empty directory, and a new one under missing parents, are accepted.
        CorpusWriter::create(&empty, "test", "mainnet", origin(), 10)
            .expect("create writer")
            .finish()
            .expect("finish");
        let nested = dir.path().join("a").join("b");
        CorpusWriter::create(&nested, "test", "mainnet", origin(), 10)
            .expect("create writer")
            .finish()
            .expect("finish");
        assert!(Corpus::open(&nested).is_ok());
    }

    #[test]
    fn a_stopped_run_keeps_its_records_apart() {
        let dir = tempfile::tempdir().expect("tempdir");
        let out = dir.path().join("corpus");
        let mut writer =
            CorpusWriter::create(&out, "test", "mainnet", origin(), 10).expect("create writer");
        writer.add(input(1, vec![])).expect("add record");
        let err = writer.finish_after(anyhow::anyhow!("harness broke"), "capturing version 2");
        assert!(format!("{:#}", err).contains("harness broke"), "{err:#}");
        // The records are kept, under a name that says the run stopped; the directory is free.
        let partial = dir.path().join("corpus.partial");
        assert_eq!(
            Corpus::open(&partial)
                .expect("open")
                .manifest()
                .num_records(),
            1
        );
        assert!(
            partial.join(STOPPED_FILE).is_file(),
            "the corpus is not marked as stopped"
        );
        assert!(!out.exists());
        // A second stopped run into the same directory keeps its records apart from the first;
        // `out/.` names `out` itself.
        let mut writer = CorpusWriter::create(
            format!("{}/.", out.display()),
            "test",
            "mainnet",
            origin(),
            10,
        )
        .expect("create writer");
        writer.add(input(2, vec![])).expect("add record");
        let _ = writer.finish_after(anyhow::anyhow!("harness broke"), "capturing version 3");
        let second = Corpus::open(dir.path().join("corpus.partial.1")).expect("open");
        assert_eq!(second.manifest().num_records(), 1);
        assert!(!out.exists());
    }

    #[test]
    fn a_run_stopped_before_any_record_leaves_out_as_it_was() {
        let dir = tempfile::tempdir().expect("tempdir");
        // A directory the run created is removed.
        let out = dir.path().join("corpus");
        let writer =
            CorpusWriter::create(&out, "test", "mainnet", origin(), 10).expect("create writer");
        let err = writer.finish_after(anyhow::anyhow!("no chain"), "fetching transactions");
        assert!(
            format!("{:#}", err).contains("before writing any record"),
            "{err:#}"
        );
        assert!(!out.exists());
        assert!(!dir.path().join("corpus.partial").exists());
        // An empty directory that was there is emptied again, and kept.
        std::fs::create_dir(&out).expect("empty out");
        let writer =
            CorpusWriter::create(&out, "test", "mainnet", origin(), 10).expect("create writer");
        let _ = writer.finish_after(anyhow::anyhow!("no chain"), "fetching transactions");
        assert_eq!(std::fs::read_dir(&out).expect("out").count(), 0);
        // The missing parents the run created are removed too.
        let nested = dir.path().join("runs").join("today").join("corpus");
        let writer =
            CorpusWriter::create(&nested, "test", "mainnet", origin(), 10).expect("create writer");
        let _ = writer.finish_after(anyhow::anyhow!("no chain"), "fetching transactions");
        assert!(!dir.path().join("runs").exists());
        // A run that wrote anything at all is not taken as untouched: one whose failed finish left only
        // its module pack is marked and moved aside.
        let writer =
            CorpusWriter::create(&out, "test", "mainnet", origin(), 10).expect("create writer");
        std::fs::write(out.join(MODULES_FILE), b"").expect("module pack");
        let err = writer.finish_after(anyhow::anyhow!("no space"), "finishing the corpus");
        assert!(!format!("{:#}", err).contains("as it was"), "{err:#}");
        let partial = dir.path().join("corpus.partial");
        assert!(partial.join(STOPPED_FILE).is_file());
        assert!(partial.join(MODULES_FILE).is_file());
    }

    #[test]
    fn a_failed_claim_leaves_no_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Some parents are created, then one that cannot be (a name longer than any file system takes):
        // the corpus directory itself, or a parent of it. Those created are removed again.
        let long = "x".repeat(5000);
        for out in [
            dir.path().join("runs").join("today").join(&long),
            dir.path().join("runs").join(&long).join("corpus"),
        ] {
            assert!(CorpusWriter::create(&out, "test", "mainnet", origin(), 10).is_err());
            assert!(!dir.path().join("runs").exists(), "{:?}", out);
        }
    }

    #[test]
    fn a_failed_shard_write_loses_no_record() {
        let dir = tempfile::tempdir().expect("tempdir");
        let out = dir.path().join("corpus");
        let mut writer =
            CorpusWriter::create(&out, "test", "mainnet", origin(), 2).expect("create writer");
        writer.add(input(1, vec![])).expect("add record");
        // The first shard cannot be written: its records stay pending.
        let blocker = out.join(SHARDS_DIR).join("0.bcs");
        std::fs::create_dir(&blocker).expect("block the shard");
        assert!(writer.add(input(2, vec![])).is_err());
        std::fs::remove_dir(&blocker).expect("unblock the shard");
        let err = writer.finish_after(anyhow::anyhow!("disk hiccup"), "capturing version 2");
        assert!(format!("{:#}", err).contains("the 2 records"), "{err:#}");
        let corpus = Corpus::open(dir.path().join("corpus.partial")).expect("open");
        assert_eq!(corpus.manifest().num_records(), 2);
    }

    #[test]
    fn a_stopped_run_that_cannot_write_still_frees_out() {
        let dir = tempfile::tempdir().expect("tempdir");
        let out = dir.path().join("corpus");
        // A dangling symlink holds the first name a stopped run would move to.
        std::os::unix::fs::symlink(dir.path().join("gone"), dir.path().join("corpus.partial"))
            .expect("symlink");
        let mut writer =
            CorpusWriter::create(&out, "test", "mainnet", origin(), 1).expect("create writer");
        writer.add(input(1, vec![])).expect("add record");
        // The last shard cannot be written: it is given up, and the shard before it is published.
        std::fs::create_dir(out.join(SHARDS_DIR).join("1.bcs")).expect("block the shard");
        assert!(writer.add(input(2, vec![])).is_err());
        let err = writer.finish_after(anyhow::anyhow!("harness broke"), "capturing version 3");
        assert!(
            format!("{:#}", err).contains("the last 1 records could not be written"),
            "{err:#}"
        );
        let partial = dir.path().join("corpus.partial.1");
        let corpus = Corpus::open(&partial).expect("open");
        assert_eq!(corpus.manifest().num_records(), 1);
        assert!(partial.join(STOPPED_FILE).is_file());
        assert!(!out.exists());
        // If the module pack cannot be written, nothing is published, but `--out` is still free.
        let mut writer =
            CorpusWriter::create(&out, "test", "mainnet", origin(), 10).expect("create writer");
        writer.add(input(1, vec![])).expect("add record");
        std::fs::create_dir(out.join(MODULES_FILE)).expect("block the module pack");
        let err = writer.finish_after(anyhow::anyhow!("harness broke"), "capturing version 2");
        assert!(format!("{:#}", err).contains("failed too"), "{err:#}");
        let partial = dir.path().join("corpus.partial.2");
        assert!(partial.join(STOPPED_FILE).is_file());
        assert!(!partial.join(MANIFEST_FILE).exists());
        assert!(!out.exists());
    }

    #[test]
    fn a_failed_finish_stops_the_run() {
        let dir = tempfile::tempdir().expect("tempdir");
        let out = dir.path().join("corpus");
        let mut writer =
            CorpusWriter::create(&out, "test", "mainnet", origin(), 1).expect("create writer");
        writer.add(input(1, vec![])).expect("add record");
        writer.add(input(2, vec![])).expect("add record");
        // The last shard cannot be written: the shards before it are published, marked.
        std::fs::create_dir(out.join(SHARDS_DIR).join("2.bcs")).expect("block the shard");
        writer.add(input(3, vec![])).expect_err("blocked");
        let err = writer.finish().expect_err("blocked");
        assert!(
            format!("{:#}", err).contains("finishing the corpus"),
            "{err:#}"
        );
        let partial = dir.path().join("corpus.partial");
        assert_eq!(
            Corpus::open(&partial)
                .expect("open")
                .manifest()
                .num_records(),
            2
        );
        let marker = std::fs::read_to_string(partial.join(STOPPED_FILE)).expect("marker");
        assert!(
            marker.contains("the last 1 records could not be written"),
            "{marker}"
        );
        assert!(!out.exists());
    }

    #[test]
    fn an_unmarked_stopped_run_keeps_its_records_unpublished() {
        let dir = tempfile::tempdir().expect("tempdir");
        let out = dir.path().join("corpus");
        let mut writer =
            CorpusWriter::create(&out, "test", "mainnet", origin(), 10).expect("create writer");
        writer.add(input(1, vec![])).expect("add record");
        // Something takes the marker's name after the corpus was created.
        std::fs::create_dir(out.join(STOPPED_FILE)).expect("block the marker");
        let err = writer.finish_after(anyhow::anyhow!("harness broke"), "capturing version 2");
        assert!(format!("{:#}", err).contains("unpublished"), "{err:#}");
        // The records are written, but with no manifest they cannot pass for a corpus.
        let partial = dir.path().join("corpus.partial");
        assert!(partial.join(MODULES_FILE).is_file());
        assert_eq!(
            std::fs::read_dir(partial.join(SHARDS_DIR))
                .expect("shards")
                .count(),
            1
        );
        assert!(!partial.join(MANIFEST_FILE).exists());
        assert!(Corpus::open(&partial).is_err());
        assert!(!out.exists());
    }

    #[test]
    fn round_trip_dedups_values_and_modules() {
        let dir = tempfile::tempdir().expect("tempdir");
        let shared = StateValue::new_legacy(vec![1; 64].into());
        let module = StateValue::new_legacy(vec![2; 32].into());
        let records = vec![
            input(10, vec![
                (StateKey::raw(b"a"), shared.clone()),
                (StateKey::raw(b"b"), StateValue::new_legacy(vec![3].into())),
                (module_key("m"), module.clone()),
            ]),
            input(11, vec![
                (StateKey::raw(b"a"), shared.clone()),
                (module_key("m"), module.clone()),
            ]),
            input(12, vec![(StateKey::raw(b"c"), shared.clone())]),
        ];
        let expected: Vec<_> = records.iter().map(|r| r.state.clone()).collect();

        let mut writer = CorpusWriter::create(dir.path(), "test", "mainnet", origin(), 2)
            .expect("create writer");
        for record in records {
            writer.add(record).expect("add record");
        }
        let manifest = writer.finish().expect("finish");
        assert_eq!(manifest.shards.len(), 2);
        assert_eq!(manifest.num_records(), 3);
        assert_eq!(manifest.num_modules, 1);
        assert_eq!(
            (
                manifest.shards[0].first_version,
                manifest.shards[0].last_version
            ),
            (10, 11)
        );

        let corpus = Corpus::open(dir.path()).expect("open corpus");
        assert_eq!(corpus.manifest(), &manifest);
        let mut loaded = vec![];
        for n in 0..manifest.shards.len() {
            let shard = corpus.load_shard(n).expect("load shard");
            for record in &shard.records {
                let state = corpus.record_state(&shard, record).expect("resolve state");
                loaded.push(state.into_iter().collect::<BTreeMap<_, _>>());
            }
            if n == 0 {
                // `shared` is stored once for both records of the first shard.
                assert_eq!(shard.values.len(), 2);
                assert_eq!(shard.records[0].modules.len(), 1);
                assert_eq!(
                    shard.records[0].onchain.as_ref().map(|o| o.gas_used),
                    Some(7)
                );
            }
        }
        assert_eq!(loaded, expected);
    }

    #[test]
    fn framework_set_is_layered_under_record_reads() {
        let dir = tempfile::tempdir().expect("tempdir");
        let old = StateValue::new_legacy(vec![1].into());
        let new = StateValue::new_legacy(vec![2].into());
        let mut writer = CorpusWriter::create(dir.path(), "test", "mainnet", origin(), 10)
            .expect("create writer");
        let framework = writer
            .add_framework(
                [
                    (module_key("a"), old.clone()),
                    (module_key("b"), old.clone()),
                ]
                .into_iter()
                .collect(),
            )
            .expect("add framework");
        // Adding the same set again yields the same id.
        assert_eq!(
            writer
                .add_framework(
                    [
                        (module_key("a"), old.clone()),
                        (module_key("b"), old.clone())
                    ]
                    .into_iter()
                    .collect()
                )
                .expect("add framework again"),
            framework
        );
        let mut record = input(1, vec![(module_key("b"), new.clone())]);
        record.framework = Some(framework);
        record.absent = [StateKey::raw(b"gone")].into_iter().collect();
        writer.add(record).expect("add record");
        let manifest = writer.finish().expect("finish");
        assert_eq!((manifest.num_modules, manifest.num_frameworks), (2, 1));

        let corpus = Corpus::open(dir.path()).expect("open corpus");
        let shard = corpus.load_shard(0).expect("load shard");
        let state = corpus
            .record_state(&shard, &shard.records[0])
            .expect("resolve state");
        assert_eq!(state.get(&module_key("a")), Some(&old));
        // The record's own read wins over the framework set.
        assert_eq!(state.get(&module_key("b")), Some(&new));
        assert_eq!(shard.records[0].absent, vec![StateKey::raw(b"gone")]);
    }

    #[test]
    fn unknown_framework_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut writer = CorpusWriter::create(dir.path(), "test", "mainnet", origin(), 10)
            .expect("create writer");
        let mut record = input(1, vec![]);
        record.framework = Some(HashValue::zero());
        assert!(writer.add(record).is_err());
    }

    #[test]
    fn metadata_is_part_of_value_identity() {
        let plain = StateValue::new_legacy(vec![1].into());
        let with_metadata = StateValue::new_with_metadata(
            vec![1].into(),
            StateValueMetadata::new(5, 0, &CurrentTimeMicroseconds { microseconds: 1 }),
        );
        assert_ne!(
            value_hash(&plain).expect("hash"),
            value_hash(&with_metadata).expect("hash")
        );
    }

    #[test]
    fn existing_corpus_is_not_overwritten() {
        let dir = tempfile::tempdir().expect("tempdir");
        CorpusWriter::create(dir.path(), "test", "mainnet", origin(), 10)
            .expect("create writer")
            .finish()
            .expect("finish");
        assert!(CorpusWriter::create(dir.path(), "test", "mainnet", origin(), 10).is_err());
    }

    #[test]
    fn out_of_order_versions_are_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut writer = CorpusWriter::create(dir.path(), "test", "mainnet", origin(), 10)
            .expect("create writer");
        writer.add(input(5, vec![])).expect("add first");
        assert!(writer.add(input(5, vec![])).is_err());
    }

    #[test]
    fn corrupted_shard_is_detected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut writer = CorpusWriter::create(dir.path(), "test", "mainnet", origin(), 10)
            .expect("create writer");
        writer
            .add(input(1, vec![(
                StateKey::raw(b"a"),
                StateValue::new_legacy(vec![1].into()),
            )]))
            .expect("add");
        let manifest = writer.finish().expect("finish");
        let shard_path = dir.path().join(&manifest.shards[0].file);
        let mut bytes = std::fs::read(&shard_path).expect("read shard");
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF;
        std::fs::write(&shard_path, bytes).expect("write shard");
        let corpus = Corpus::open(dir.path()).expect("open corpus");
        assert!(corpus.load_shard(0).is_err());
    }

    #[test]
    fn a_shard_outside_the_corpus_is_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut writer = CorpusWriter::create(dir.path(), "test", "mainnet", origin(), 10)
            .expect("create writer");
        writer
            .add(input(1, vec![(
                StateKey::raw(b"a"),
                StateValue::new_legacy(vec![1].into()),
            )]))
            .expect("add");
        let mut manifest = writer.finish().expect("finish");
        for file in [
            "/tmp/data.jsonl",
            "shards/../../data.jsonl",
            "elsewhere.bcs",
        ] {
            manifest.shards[0].file = file.to_string();
            std::fs::write(
                dir.path().join(MANIFEST_FILE),
                serde_json::to_vec(&manifest).expect("encode"),
            )
            .expect("write manifest");
            assert!(Corpus::open(dir.path()).is_err(), "{}", file);
        }
    }
}
