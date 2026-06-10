// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Reads the per-transaction files produced by `aptos-replay-benchmark`.
//!
//! For each replayed version `V`, the download script writes two files into the
//! data directory:
//!   - `<V>_txns` — `bcs(Vec<TransactionBlock>)` (one block, one transaction);
//!   - `<V>_inputs` — `bcs(Vec<ReadSet>)` (one read-set, the complete captured
//!     state for that transaction, including all modules it touches).
//!
//! The `ReadSet` and `TransactionBlock` types are reused directly from
//! `aptos-replay-benchmark` (no copy), so the read-set is the same complete one
//! the benchmark captures via its `get_state_slot` hook.

use anyhow::{anyhow, Context, Result};
use aptos_types::{
    replay::{ReadSet, TransactionBlock},
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{PersistedAuxiliaryInfo, Transaction},
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const TXNS_SUFFIX: &str = "_txns";
const INPUTS_SUFFIX: &str = "_inputs";

/// Read-only handle over a directory of replay-benchmark per-version files.
pub struct Dump {
    root: PathBuf,
}

impl Dump {
    /// Opens the dump at `root`.
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            root: root.as_ref().to_path_buf(),
        })
    }

    /// All dumped versions, derived from the `<V>_txns` filenames, sorted.
    pub fn versions(&self) -> Result<Vec<u64>> {
        let mut versions = Vec::new();
        for entry in
            fs::read_dir(&self.root).with_context(|| format!("reading {}", self.root.display()))?
        {
            let name = entry?.file_name();
            if let Some(version) = name.to_str().and_then(|n| n.strip_suffix(TXNS_SUFFIX)) {
                versions.push(
                    version
                        .parse::<u64>()
                        .with_context(|| format!("parsing version from {name:?}"))?,
                );
            }
        }
        versions.sort_unstable();
        Ok(versions)
    }

    /// The complete captured input state for `version`.
    pub fn state(&self, version: u64) -> Result<BTreeMap<StateKey, StateValue>> {
        let path = self.root.join(format!("{version}{INPUTS_SUFFIX}"));
        let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        let read_sets: Vec<ReadSet> = bcs::from_bytes(&bytes)
            .with_context(|| format!("decoding read-sets for version {version}"))?;
        let read_set = read_sets
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("empty read-set file for version {version}"))?;
        Ok(read_set.into_data().into_iter().collect())
    }

    /// The transaction recorded for `version`, or `None` if no `<V>_txns` file
    /// is present.
    pub fn transaction(&self, version: u64) -> Result<Option<Transaction>> {
        let path = self.root.join(format!("{version}{TXNS_SUFFIX}"));
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        let blocks: Vec<TransactionBlock> = bcs::from_bytes(&bytes)
            .with_context(|| format!("decoding transactions for version {version}"))?;
        let txn = blocks
            .into_iter()
            .next()
            .and_then(|block| block.transactions.into_iter().next());
        Ok(txn)
    }

    /// The persisted auxiliary info (block transaction index) recorded for
    /// `version`, or `None` if absent. Needed so the monotonic transaction
    /// counter native matches the on-chain value during replay.
    pub fn aux_info(&self, version: u64) -> Result<PersistedAuxiliaryInfo> {
        let path = self.root.join(format!("{version}{TXNS_SUFFIX}"));
        let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        let blocks: Vec<TransactionBlock> = bcs::from_bytes(&bytes)
            .with_context(|| format!("decoding transactions for version {version}"))?;
        Ok(blocks
            .into_iter()
            .next()
            .and_then(|block| block.persisted_auxiliary_infos.into_iter().next())
            .unwrap_or(PersistedAuxiliaryInfo::None))
    }
}
