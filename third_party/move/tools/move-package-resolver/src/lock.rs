// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use git2::Oid;
use move_package_cache::{
    CanonicalGitIdentity, CanonicalNodeIdentity, PackageCache, PackageCacheListener,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{btree_map, BTreeMap},
    fs,
    path::Path,
};
use url::Url;

/// Represents the package lock, which stores resolved identities of git branches and network versions.
/// This ensures reproducible builds by pinning dependencies to specific commits or network versions.
#[derive(Serialize, Deserialize)]
pub struct PackageLock {
    // git_identity (stringified) -> commit_id
    git: BTreeMap<String, String>,

    // node_identity (stringified) -> version
    on_chain: BTreeMap<String, u64>,
}

impl PackageLock {
    /// Creates a new, empty [`PackageLock`].
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            git: BTreeMap::new(),
            on_chain: BTreeMap::new(),
        }
    }

    /// Saves the current state of the package lock to a file in TOML format.
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let toml = toml::to_string_pretty(self)?;
        fs::write(path, toml)?;
        Ok(())
    }

    /// Loads the package lock from a file if it exists, or returns a new empty lock if not found.
    pub fn load_from_file_or_empty(path: impl AsRef<Path>) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(contents) => Ok(toml::from_str(&contents)?),
            Err(err) => match err.kind() {
                std::io::ErrorKind::NotFound => Ok(Self::new()),
                _ => Err(err.into()),
            },
        }
    }

    /// Resolves and pins a git revision.
    ///
    /// - If the given git URL and branch/rev combo is already recorded in the lock,
    /// returns the pinned commit hash.
    /// - Otherwise, queries the remote, records the result,
    /// and returns the resolved commit hash.
    pub async fn resolve_git_revision<L>(
        &mut self,
        package_cache: &PackageCache<L>,
        git_url: &Url,
        rev: &str,
    ) -> Result<Oid>
    where
        L: PackageCacheListener,
    {
        let git_identity = CanonicalGitIdentity::new(git_url)?;

        let repo_loc_and_rev = format!("{}@{}", git_identity, rev);

        let res = match self.git.entry(repo_loc_and_rev) {
            btree_map::Entry::Occupied(entry) => entry.get().clone(),
            btree_map::Entry::Vacant(entry) => {
                let oid = package_cache.resolve_git_revision(git_url, rev).await?;
                entry.insert(oid.to_string()).clone()
            },
        };

        Ok(Oid::from_str(&res)?)
    }

    /// Resolves and pins the network version for the given URL.
    ///
    /// - If the version is already recorded in the lock, returns the pinned version.
    /// - Otherwise, queries the network, records the result in the lock, and returns the resolved version.
    pub async fn resolve_network_version(&mut self, fullnode_url: &Url) -> Result<u64> {
        let node_identity = CanonicalNodeIdentity::new(fullnode_url)?;

        let res = match self.on_chain.entry(node_identity.to_string()) {
            btree_map::Entry::Occupied(entry) => *entry.get(),
            btree_map::Entry::Vacant(entry) => {
                let client = velor_rest_client::Client::new(fullnode_url.clone());
                let version = client.get_ledger_information().await?.into_inner().version;

                entry.insert(version);

                version
            },
        };

        Ok(res)
    }
}
