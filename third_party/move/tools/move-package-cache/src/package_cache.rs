// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    canonical,
    file_lock::FileLock,
    listener::{EmptyPackageCacheListener, PackageCacheListener},
};
use anyhow::{bail, Result};
use aptos_framework::natives::code::PackageRegistry;
use futures::future;
use git2::{
    build::RepoBuilder, FetchOptions, ObjectType, Oid, RemoteCallbacks, Repository, TreeWalkResult,
};
use move_core_types::account_address::AccountAddress;
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};
use url::Url;

/// Removes a directory if it exists, ignoring "directory not found" errors.
fn remove_dir_if_exists(path: &Path) -> std::io::Result<()> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

/// A Move package cache that manages Git repositories and on-chain packages.
///
/// The cache provides async APIs for fetching and storing remote dependencies.
/// Safe concurrent access is providedthrough file-based locking.
pub struct PackageCache<L> {
    root: PathBuf,
    listener: L,
}

/// An opened Git repository with an active file lock.
///
/// This serves two purposes:
/// 1. Provides access to the contents of an opened repo
/// 2. Holds a file lock that prevents other package cache instances from accessing
///    the repository while this `ActiveRepository` is still alive
pub struct ActiveRepository {
    repo: Repository,

    #[allow(dead_code)]
    lock: FileLock,
}

impl PackageCache<EmptyPackageCacheListener> {
    /// Creates a new package cache with a no-op listener.
    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        Self::new_with_listener(root, EmptyPackageCacheListener)
    }
}

impl<L> PackageCache<L> {
    /// Creates a new package cache with a custom listener.
    pub fn new_with_listener(root: impl AsRef<Path>, listener: L) -> Result<Self> {
        let root = root.as_ref().to_owned();

        fs::create_dir_all(&root)?;

        Ok(PackageCache { root, listener })
    }

    /// Clones or updates a Git repository, ensuring it is available locally with up-to-date data.
    ///
    /// Returns an `ActiveRepository` object. This can be used to access the contents of the repo, and while
    /// is still alive, a lock is held to prevent other package cache instances to access the repo.
    async fn clone_or_update_git_repo(&self, git_url: &Url) -> Result<ActiveRepository>
    where
        L: PackageCacheListener,
    {
        let repo_name = canonical::canonical_repo_name_from_url(git_url);
        let repos_path = self.root.join("git").join("repos");
        let repo_path = repos_path.join(&repo_name);

        // First, acquire a file lock to ensure exclusive write access to the cached repo.
        let lock_path = repo_path.with_extension("lock");

        fs::create_dir_all(&repos_path)?;
        let file_lock =
            FileLock::lock_with_alert_on_wait(&lock_path, Duration::from_millis(1000), || {
                self.listener.on_file_lock_wait(&lock_path);
            })
            .await?;

        // Next, ensure that we have an up-to-date clone of the repo locally.
        // - If the repo does not exist, clone it.
        // - If the repo already exists, update it.
        let mut cbs = RemoteCallbacks::new();
        let mut received = 0;
        cbs.transfer_progress(move |stats| {
            let received_new = stats.received_objects();

            if received_new != received {
                received = received_new;

                self.listener.on_repo_receive_object(
                    git_url.as_str(),
                    stats.received_objects(),
                    stats.total_objects(),
                );
            }

            true
        });

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(cbs);

        let repo = if repo_path.exists() {
            self.listener.on_repo_update_start(git_url.as_str());

            let repo = Repository::open_bare(&repo_path)?;
            {
                let mut remote = repo.find_remote("origin")?;
                remote.fetch(
                    &["refs/heads/*:refs/remotes/origin/*"],
                    Some(&mut fetch_options),
                    None,
                )?;
            }

            self.listener.on_repo_update_complete(git_url.as_str());

            repo
        } else {
            let mut repo_builder = RepoBuilder::new();
            repo_builder.fetch_options(fetch_options);
            repo_builder.bare(true);

            self.listener.on_repo_clone_start(git_url.as_str());
            let repo = repo_builder.clone(git_url.as_str(), &repo_path)?;
            self.listener.on_repo_clone_complete(git_url.as_str());

            repo
        };

        Ok(ActiveRepository {
            repo,
            lock: file_lock,
        })
    }

    /// Resolves a Git revision string to a specific commit id.
    ///
    /// This will clone the repo if it is not already cached, or update it if it is.
    pub async fn resolve_git_revision(&self, git_url: &Url, rev: &str) -> Result<Oid>
    where
        L: PackageCacheListener,
    {
        let repo = self.clone_or_update_git_repo(git_url).await?;

        let obj = repo.repo.revparse_single(&format!("origin/{}", rev))?;
        let oid = obj.id();

        Ok(oid)
    }

    /// Checks out a commit of a Git repository and returns the path to the checkout.
    ///
    /// A checkout is an immutable snapshot of a repository at a specific commit.
    /// If a checkout already exists, the existing path is returned.
    pub async fn checkout_git_repo(&self, git_url: &Url, oid: Oid) -> Result<PathBuf>
    where
        L: PackageCacheListener,
    {
        let repo_name = canonical::canonical_repo_name_from_url(git_url);
        let checkouts_path = self.root.join("git").join("checkouts");

        // Check if a checkout already exists for this commit.
        let checkout_path = checkouts_path.join(format!("{}@{}", repo_name, oid));
        if checkout_path.exists() {
            return Ok(checkout_path);
        }

        // Checkout does not exist -- need to create one.
        //
        // However before we do that, we need to make sure the repo is cloned to the local
        // file system and updated.
        let repo = self.clone_or_update_git_repo(git_url).await?;

        // Acquire a file lock to ensure exclusive write access to the checkout.
        let lock_path = checkout_path.with_extension("lock");

        fs::create_dir_all(&checkouts_path)?;
        let _file_lock =
            FileLock::lock_with_alert_on_wait(&lock_path, Duration::from_millis(1000), || {
                self.listener.on_file_lock_wait(&lock_path);
            })
            .await?;

        self.listener
            .on_repo_checkout(git_url.as_str(), oid.as_bytes());

        // Create the files from the commit.
        //
        // The files stored into a temporary directory, and then the temporary directory
        // is atomically renamed/moved to the destination.
        //
        // This is to ensure we only expose complete checkouts.
        let temp = tempfile::tempdir_in(&checkouts_path)?;

        let commit = repo.repo.find_commit(oid)?;
        let tree = commit.tree()?;

        tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
            let name = entry.name().unwrap_or("");
            let full_path = temp.path().join(format!("{}{}", root, name));

            match entry.kind() {
                Some(ObjectType::Blob) => {
                    let blob = repo.repo.find_blob(entry.id()).unwrap();
                    fs::create_dir_all(full_path.parent().unwrap()).unwrap();
                    let mut file = File::create(&full_path).unwrap();
                    file.write_all(blob.content()).unwrap();
                },
                Some(ObjectType::Tree) => (),
                _ => {},
            }

            TreeWalkResult::Ok
        })?;

        remove_dir_if_exists(&checkout_path)?;
        fs::rename(temp.into_path(), &checkout_path)?;

        Ok(checkout_path)
    }

    /// Fetches an on-chain package from the specified network and version and stores it locally.
    /// Returns the path to the cached package.
    ///
    /// The cached package currently only contains the bytecode modules, but may be extended with
    /// additional metadata in the future.
    pub async fn fetch_on_chain_package(
        &self,
        fullnode_url: Url,
        network_version: u64,
        address: AccountAddress,
        package_name: &str,
    ) -> Result<PathBuf>
    where
        L: PackageCacheListener,
    {
        let on_chain_packages_path = self.root.join("on-chain");

        let canonical_name = canonical::canonical_on_chain_package_name(
            &fullnode_url,
            network_version,
            address,
            package_name,
        );

        let cached_package_path = on_chain_packages_path.join(&canonical_name);

        // If the package directory already exists, assume it has been cached.
        if cached_package_path.exists() {
            // TODO: In the future, consider verifying data integrity,
            //       e.g. hash of metadata or full contents.
            return Ok(cached_package_path);
        }

        // Package directory does not exist -- need to download the package and cache it.
        //
        // First, acquire a lock to ensure exclusive write access to this package.
        let lock_path = cached_package_path.with_extension("lock");

        fs::create_dir_all(&on_chain_packages_path)?;
        let _file_lock =
            FileLock::lock_with_alert_on_wait(&lock_path, Duration::from_millis(1000), || {
                self.listener.on_file_lock_wait(&lock_path);
            })
            .await?;

        self.listener.on_file_lock_acquired(&lock_path);

        // After acquiring the lock, re-check if the package was already cached by another process.
        if cached_package_path.exists() {
            return Ok(cached_package_path);
        }

        // Fetch the on-chain package registry at the specified ledger version and look-up the
        // package by name.
        self.listener
            .on_bytecode_package_download_start(address, package_name);

        let client = aptos_rest_client::Client::new(fullnode_url.clone());

        let package_registry = client
            .get_account_resource_at_version_bcs::<PackageRegistry>(
                address,
                "0x1::code::PackageRegistry",
                network_version,
            )
            .await?
            .into_inner();

        let package = match package_registry
            .packages
            .iter()
            .find(|package_metadata| package_metadata.name == package_name)
        {
            Some(package) => package,
            None => bail!(
                "package not found: {}//{}::{}",
                fullnode_url,
                address,
                package_name
            ),
        };

        self.listener
            .on_bytecode_package_receive_metadata(address, package);

        // Download all modules of the package concurrently.
        //
        // The downloaded files are first saved into a temporary directory, and then
        // the temporary directory is atomically renamed/moved to the destination.
        // This is to ensure we only expose complete downloads.
        let temp = tempfile::tempdir_in(&on_chain_packages_path)?;

        let fetch_futures = package.modules.iter().map(|module| {
            let client = client.clone();
            let temp_path = temp.path().to_owned();
            let package_name = package_name.to_string();
            let module_name = module.name.clone();

            async move {
                let module_bytes = client
                    .get_account_module_bcs_at_version(address, &module_name, network_version)
                    .await?
                    .into_inner();

                let module_file_path = temp_path.join(&module_name).with_extension("mv");

                // Use blocking file write in spawn_blocking to avoid blocking the async runtime
                tokio::task::spawn_blocking(move || {
                    fs::create_dir_all(module_file_path.parent().unwrap())?;
                    let mut file = File::create(&module_file_path)?;
                    file.write_all(&module_bytes)?;
                    Ok::<(), std::io::Error>(())
                })
                .await??;

                // Notify listener after writing
                self.listener.on_bytecode_package_receive_module(
                    address,
                    &package_name,
                    &module_name,
                );
                Ok::<(), anyhow::Error>(())
            }
        });

        future::try_join_all(fetch_futures).await?;

        remove_dir_if_exists(&cached_package_path)?;
        fs::rename(temp.into_path(), cached_package_path)?;

        self.listener
            .on_bytecode_package_download_complete(address, package_name);

        Ok(PathBuf::new())
    }
}
