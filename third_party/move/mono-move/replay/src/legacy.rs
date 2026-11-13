// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Reads dumps written by `aptos-e2e-comparison-testing dump` (format in
//! `aptos-comparison-testing-dump-format`), so the transactions collected with that tool can be
//! converted into a corpus.
//!
//! What a dump does not hold: the framework modules (`0x1`, `0x3`, `0x4` reads were never
//! recorded), reads that found no value, the on-chain status, and the auxiliary info.

use crate::txn::txn_kind;
use anyhow::{Context, Result};
use aptos_comparison_testing_dump_format::{
    DataManager, PackageInfo, APTOS_COMMONS, INDEX_FILE, STATE_DATA,
};
use aptos_types::{
    access_path::Path as AccessPathKind,
    state_store::{
        state_key::{inner::StateKeyInner, StateKey},
        state_value::StateValue,
    },
    transaction::{Transaction, Version},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
};

/// Added to `aptos-commons` by the same tool change (aptos-core `9ad8d18c86`) that started
/// executing against the cloned framework, so its presence marks the later era.
const COMPILED_FRAMEWORK_MARKER: &str = "aptos-experimental";

/// Which framework the dump executed against, which decides the framework a replay must supply.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyEra {
    /// The on-chain framework at each transaction's version, read live and not recorded.
    OnChainFramework,
    /// The framework compiled from `aptos-commons` (git `main` at dump time; the commit is not
    /// recorded). Feature flags may also have been overridden in the recorded state.
    CompiledFramework,
    /// Sources only: the transactions were not executed and no state was recorded.
    SourceOnly,
}

pub struct LegacyRecord {
    pub version: Version,
    pub package: PackageInfo,
    pub txn: Transaction,
    pub state: BTreeMap<StateKey, StateValue>,
}

pub struct LegacyDump {
    root: PathBuf,
    data: DataManager,
}

impl LegacyDump {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let data = DataManager::open_read_only(&root)?;
        Ok(Self { root, data })
    }

    pub fn era(&self) -> Result<LegacyEra> {
        if self.versions()?.is_empty() {
            return Ok(LegacyEra::SourceOnly);
        }
        // Without `aptos-commons` both eras read the framework from chain.
        if self
            .root
            .join(APTOS_COMMONS)
            .join(COMPILED_FRAMEWORK_MARKER)
            .exists()
        {
            Ok(LegacyEra::CompiledFramework)
        } else {
            Ok(LegacyEra::OnChainFramework)
        }
    }

    pub fn versions(&self) -> Result<Vec<Version>> {
        self.data.txn_index_versions()
    }

    pub fn read(&self, version: Version) -> Result<LegacyRecord> {
        let index = self
            .data
            .try_get_txn_index(version)?
            .with_context(|| format!("version {} is not in the txn index", version))?;
        Ok(LegacyRecord {
            version: index.version,
            package: index.package_info,
            txn: index.txn,
            state: self.data.try_get_state(version)?,
        })
    }

    /// Versions with a state file, whether or not they are indexed.
    fn state_file_versions(&self) -> Result<HashSet<Version>> {
        let dir = self.root.join(STATE_DATA);
        if !dir.exists() {
            return Ok(HashSet::new());
        }
        let mut versions = HashSet::new();
        for entry in std::fs::read_dir(&dir).with_context(|| format!("failed to list {:?}", dir))? {
            let name = entry.context("failed to list state files")?.file_name();
            if let Some(version) = name
                .to_str()
                .and_then(|n| n.strip_suffix("_state"))
                .and_then(|n| n.parse().ok())
            {
                versions.insert(version);
            }
        }
        Ok(versions)
    }

    fn index_file_len(&self) -> Result<usize> {
        let path = self.root.join(INDEX_FILE);
        if !path.exists() {
            return Ok(0);
        }
        let text =
            std::fs::read_to_string(&path).with_context(|| format!("failed to read {:?}", path))?;
        Ok(text.lines().filter(|l| !l.trim().is_empty()).count())
    }

    /// Decodes every record and summarizes what the dump holds. `shard_size` sizes the
    /// per-shard value pooling estimate.
    pub fn survey(&self, shard_size: usize) -> Result<Survey> {
        let versions = self.versions()?;
        let state_files = self.state_file_versions()?;
        let mut survey = Survey {
            era: Some(self.era()?),
            indexed: versions.len(),
            index_file_lines: self.index_file_len()?,
            state_files: state_files.len(),
            first_version: versions.first().copied(),
            last_version: versions.last().copied(),
            ..Survey::default()
        };
        let indexed: HashSet<_> = versions.iter().copied().collect();
        survey.indexed_without_state = indexed.difference(&state_files).count();
        survey.state_without_index = state_files.difference(&indexed).count();

        let mut modules = HashSet::new();
        let mut shard_values = HashSet::new();
        for (i, version) in versions.into_iter().enumerate() {
            if i % shard_size.max(1) == 0 {
                shard_values.clear();
            }
            let record = match self.read(version) {
                Ok(record) => record,
                Err(err) => {
                    survey.decode_errors += 1;
                    survey.first_error.get_or_insert(format!("{:#}", err));
                    continue;
                },
            };
            survey.decoded += 1;
            *survey
                .txn_kinds
                .entry(txn_kind(&record.txn).to_string())
                .or_default() += 1;
            *survey
                .packages
                .entry(format!(
                    "{}@{}",
                    record.package.package_name,
                    record.package.address.short_str_lossless()
                ))
                .or_default() += 1;
            for (key, value) in &record.state {
                *survey
                    .key_kinds
                    .entry(key_kind(key).to_string())
                    .or_default() += 1;
                let len = value.bytes().len() as u64;
                survey.value_bytes += len;
                let hash = crate::corpus::value_hash(value)?;
                if crate::corpus::is_module_key(key) {
                    if modules.insert(hash) {
                        survey.module_bytes += len;
                    }
                } else if shard_values.insert(hash) {
                    survey.pooled_value_bytes += len;
                }
            }
        }
        survey.distinct_modules = modules.len();
        Ok(survey)
    }
}

/// What a legacy dump holds, from decoding every record.
#[derive(Debug, Default, Serialize)]
pub struct Survey {
    pub era: Option<LegacyEra>,
    pub first_version: Option<Version>,
    pub last_version: Option<Version>,
    pub indexed: usize,
    pub index_file_lines: usize,
    pub state_files: usize,
    pub indexed_without_state: usize,
    pub state_without_index: usize,
    pub decoded: usize,
    pub decode_errors: usize,
    pub first_error: Option<String>,
    pub txn_kinds: BTreeMap<String, usize>,
    pub packages: BTreeMap<String, usize>,
    pub key_kinds: BTreeMap<String, usize>,
    pub distinct_modules: usize,
    /// Bytes of all recorded values, as stored in the dump.
    pub value_bytes: u64,
    /// Bytes of distinct modules across the dump.
    pub module_bytes: u64,
    /// Bytes of non-module values after pooling per shard.
    pub pooled_value_bytes: u64,
}

fn key_kind(key: &StateKey) -> &'static str {
    match key.inner() {
        StateKeyInner::AccessPath(ap) => match ap.get_path() {
            AccessPathKind::Code(_) if key.is_aptos_code() => "module/framework",
            AccessPathKind::Code(_) => "module/user",
            AccessPathKind::Resource(_) => "resource",
            AccessPathKind::ResourceGroup(_) => "resource_group",
        },
        StateKeyInner::TableItem { .. } => "table_item",
        StateKeyInner::Raw(_) => "raw",
        StateKeyInner::TradingNative(_) => "trading_native",
    }
}
