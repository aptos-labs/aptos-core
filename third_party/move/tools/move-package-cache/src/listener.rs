// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_framework::natives::code::PackageMetadata;
use move_core_types::account_address::AccountAddress;
use std::path::Path;

/// Trait for subscribing to package cache events.
///
/// This allows clients to receive notifications about various operations happening
/// in the package cache, such as repository cloning, package downloads, and file
/// lock acquisitions. Useful for providing visual feedback to users, debugging,
/// and collecting telemetry data.
///
/// Visual feedback is particularly important to reassure users that the tool is
/// actively working and not stalled during long-running operations.
#[allow(unused_variables)]
pub trait PackageCacheListener: Sync {
    fn on_file_lock_wait(&self, lock_path: &Path) {}
    fn on_file_lock_acquired(&self, lock_path: &Path) {}

    fn on_repo_update_start(&self, repo_url: &str) {}
    fn on_repo_update_complete(&self, repo_url: &str) {}
    fn on_repo_clone_start(&self, repo_url: &str) {}
    fn on_repo_clone_complete(&self, repo_url: &str) {}
    fn on_repo_receive_object(&self, repo_url: &str, received: usize, total: usize) {}

    fn on_repo_checkout(&self, repo_url: &str, commit_id: &[u8]) {}

    fn on_bytecode_package_download_start(&self, address: AccountAddress, package_name: &str) {}
    fn on_bytecode_package_receive_metadata(
        &self,
        address: AccountAddress,
        package_metadata: &PackageMetadata,
    ) {
    }
    fn on_bytecode_package_receive_module(
        &self,
        address: AccountAddress,
        package_name: &str,
        module_name: &str,
    ) {
    }
    fn on_bytecode_package_download_complete(&self, address: AccountAddress, package_name: &str) {}
}

/// A no-op implementation of `PackageCacheListener`.
///
/// This can be used when no event notifications are needed.
pub struct EmptyPackageCacheListener;

impl PackageCacheListener for EmptyPackageCacheListener {}

/// A debug implementation of `PackageCacheListener` that prints all events to stdout,
/// used mainly for debugging and development.
pub struct DebugPackageCacheListener;

impl PackageCacheListener for DebugPackageCacheListener {
    fn on_file_lock_wait(&self, lock_path: &Path) {
        println!("waiting for repo lock {}", lock_path.display())
    }

    fn on_file_lock_acquired(&self, lock_path: &Path) {
        println!("lock acquired {}", lock_path.display())
    }

    fn on_repo_clone_start(&self, repo_url: &str) {
        println!("cloning {}", repo_url)
    }

    fn on_repo_clone_complete(&self, repo_url: &str) {
        println!("cloned {}", repo_url)
    }

    fn on_repo_update_start(&self, repo_url: &str) {
        println!("updating {}", repo_url)
    }

    fn on_repo_update_complete(&self, repo_url: &str) {
        println!("updated {}", repo_url)
    }

    fn on_repo_receive_object(&self, repo_url: &str, received: usize, total: usize) {
        println!("repo {}, received {}/{} objects", repo_url, received, total);
    }

    fn on_repo_checkout(&self, repo_url: &str, commit_id: &[u8]) {
        println!("checking out {}@{}", repo_url, hex::encode(commit_id))
    }

    fn on_bytecode_package_download_start(&self, address: AccountAddress, package_name: &str) {
        println!("downloading bytecode package {}::{}", address, package_name)
    }

    fn on_bytecode_package_receive_metadata(
        &self,
        address: AccountAddress,
        package_metadata: &PackageMetadata,
    ) {
        println!(
            "received metadata for package {}::{}",
            address, package_metadata.name
        );
        println!("  upgrade number: {}", package_metadata.upgrade_number);
        println!(
            "  upgrade policy: {}",
            package_metadata.upgrade_policy.policy
        );
        println!("  modules:");
        for module in &package_metadata.modules {
            println!("    {}", module.name);
        }
    }

    fn on_bytecode_package_receive_module(
        &self,
        address: AccountAddress,
        package_name: &str,
        module_name: &str,
    ) {
        println!(
            "downloaded bytecode module {}::{}::{}",
            address, package_name, module_name
        )
    }

    fn on_bytecode_package_download_complete(&self, address: AccountAddress, package_name: &str) {
        println!("downloaded bytecode package {}::{}", address, package_name)
    }
}
