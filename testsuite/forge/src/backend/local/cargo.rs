// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::Result;
use anyhow::{bail, Context};
use aptos_logger::info;
use serde::Deserialize;
use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::NamedTempFile;

#[derive(Deserialize)]
pub struct Metadata {
    pub target_directory: PathBuf,
    pub workspace_root: PathBuf,
}

/// at _forge_ compile time, decide what kind of build we will use for `aptos-node`
pub fn use_release() -> bool {
    // option_env!("LOCAL_SWARM_NODE_RELEASE").is_some()
    true
}

/// at _forge_ compile time, decide to build `aptos-node` only for consensus perf tests
pub fn build_consensus_only_node() -> bool {
    option_env!("CONSENSUS_ONLY_PERF_TEST").is_some()
}

/// at forge _run_ time, compile `aptos-node` without indexer
pub fn build_aptos_node_without_indexer() -> bool {
    std::env::var("FORGE_BUILD_WITHOUT_INDEXER").is_ok()
}

pub fn metadata() -> Result<Metadata> {
    let output = Command::new("cargo")
        .arg("metadata")
        .arg("--no-deps")
        .arg("--format-version=1")
        .output()
        .context("Failed to query cargo metadata")?;

    serde_json::from_slice(&output.stdout).map_err(Into::into)
}

/// Get the aptos node binary from the current working directory
pub fn get_aptos_node_binary_from_worktree() -> Result<(String, PathBuf)> {
    let metadata = metadata()?;
    let mut revision = git_rev_parse(&metadata, "HEAD")?;
    if git_is_worktree_dirty()? {
        revision.push_str("-dirty");
    }

    let bin_path = cargo_build_aptos_node(&metadata.workspace_root, &metadata.target_directory)?;

    Ok((revision, bin_path))
}

/// This function will attempt to build the aptos-node binary at an arbitrary revision.
/// Using the `target/forge` as a working directory it will do the following:
///     1. Look for a binary named `aptos-node--<revision>`, if it already exists return it
///     2. If the binary doesn't exist check out the revision to `target/forge/revision` by doing
///        `git archive --format=tar <revision> | tar x`
///     3. Using the `target/forge/target` directory as a cargo artifact directory, build the
///        binary and then move it to `target/forge/aptos-node--<revision>`
pub fn get_aptos_node_binary_at_revision(revision: &str) -> Result<(String, PathBuf)> {
    let metadata = metadata()?;
    let forge_directory = metadata.target_directory.join("forge");
    let revision = git_rev_parse(&metadata, format!("{}^{{commit}}", revision))?;
    let checkout_dir = forge_directory.join(&revision);
    let forge_target_directory = forge_directory.join("target");
    let aptos_node_bin = forge_directory.join(format!(
        "aptos-node--{}{}",
        revision,
        env::consts::EXE_SUFFIX
    ));

    if aptos_node_bin.exists() {
        return Ok((revision, aptos_node_bin));
    }

    fs::create_dir_all(&forge_target_directory)?;

    checkout_revision(&metadata, &revision, &checkout_dir)?;

    fs::rename(
        cargo_build_aptos_node(&checkout_dir, &forge_target_directory)?,
        &aptos_node_bin,
    )?;

    let _ = fs::remove_dir_all(&checkout_dir);

    Ok((revision, aptos_node_bin))
}

fn git_rev_parse<R: AsRef<str>>(metadata: &Metadata, rev: R) -> Result<String> {
    let rev = rev.as_ref();
    let output = Command::new("git")
        .current_dir(&metadata.workspace_root)
        .arg("rev-parse")
        .arg(rev)
        .output()
        .context("Failed to parse revision")?;
    if output.status.success() {
        String::from_utf8(output.stdout)
            .map(|s| s.trim().to_owned())
            .map_err(Into::into)
    } else {
        bail!("Failed to parse revision: {}", rev);
    }
}

// Determine if the worktree is dirty
fn git_is_worktree_dirty() -> Result<bool> {
    Command::new("git")
        .args(["diff-index", "--name-only", "HEAD", "--"])
        .output()
        .context("Failed to determine if the worktree is dirty")
        .map(|output| !output.stdout.is_empty())
}

/// Attempt to query the local git repository's remotes for the one that points to the upstream
/// aptos-labs/aptos-core repository, falling back to "origin" if unable to locate the remote
pub fn git_get_upstream_remote() -> Result<String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(
            "git remote -v | grep \"https://github.com/aptos-labs/aptos-core.* (fetch)\" | cut -f1",
        )
        .output()
        .context("Failed to get upstream remote")?;

    if output.status.success() {
        let remote = String::from_utf8(output.stdout).map(|s| s.trim().to_owned())?;

        // If its empty, fall back to "origin"
        if remote.is_empty() {
            Ok("origin".into())
        } else {
            Ok(remote)
        }
    } else {
        Ok("origin".into())
    }
}

pub fn git_merge_base<R: AsRef<str>>(rev: R) -> Result<String> {
    let rev = rev.as_ref();
    let output = Command::new("git")
        .arg("merge-base")
        .arg("HEAD")
        .arg(rev)
        .output()
        .context("Failed to find merge base")?;
    if output.status.success() {
        String::from_utf8(output.stdout)
            .map(|s| s.trim().to_owned())
            .map_err(Into::into)
    } else {
        bail!("Failed to find merge base between: {} and HEAD", rev);
    }
}

pub fn cargo_build_common_args() -> Vec<&'static str> {
    let mut args = if build_aptos_node_without_indexer() {
        vec!["build", "--features=failpoints,smoke-test"]
    } else {
        vec!["build", "--features=failpoints,indexer,smoke-test"]
    };
    if build_consensus_only_node() {
        args.push("--features=consensus-only-perf-test");
    }
    if use_release() {
        args.push("--release");
    }
    args
}

fn cargo_build_aptos_node<D, T>(directory: D, target_directory: T) -> Result<PathBuf>
where
    D: AsRef<Path>,
    T: AsRef<Path>,
{
    let target_directory = target_directory.as_ref();
    let directory = directory.as_ref();

    let mut args = cargo_build_common_args();
    // build the aptos-node package directly to avoid feature unification issues
    args.push("--package=aptos-node");

    // #[cfg(feature = "sim-types")]
    // args.append(&mut vec!["--features", "sim-types"]);

    // #[cfg(feature = "force-aptos-types")]
    // args.append(&mut vec!["--features", "force-aptos-types"]);
    //
    // #[cfg(feature = "inject-delays")]
    // args.append(&mut vec!["--features", "inject-delays"]);
    //
    // #[cfg(feature = "inject-drops")]
    // args.append(&mut vec!["--features", "inject-drops"]);

    info!("Compiling with cargo args: {:?}", args);
    let output = Command::new("cargo")
        .current_dir(directory)
        .env("CARGO_TARGET_DIR", target_directory)
        .args(&args)
        .output()
        .context("Failed to build aptos-node")?;

    if output.status.success() {
        let bin_path = target_directory.join(format!(
            "{}/{}{}",
            if use_release() { "release" } else { "debug" },
            "aptos-node",
            env::consts::EXE_SUFFIX
        ));
        if !bin_path.exists() {
            bail!(
                "Can't find binary aptos-node at expected path {:?}",
                bin_path
            );
        }
        info!("Local swarm node binary path: {:?}", bin_path);
        Ok(bin_path)
    } else {
        io::stderr().write_all(&output.stderr)?;

        bail!(
            "Failed to build aptos-node: 'cd {} && CARGO_TARGET_DIR={} cargo build --bin=aptos-node",
            directory.display(),
            target_directory.display(),
        );
    }
}

fn checkout_revision(metadata: &Metadata, revision: &str, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;

    let archive_file = NamedTempFile::new()?.into_temp_path();

    let output = Command::new("git")
        .current_dir(&metadata.workspace_root)
        .arg("archive")
        .arg("--format=tar")
        .arg("--output")
        .arg(&archive_file)
        .arg(revision)
        .output()
        .context("Failed to run git archive")?;
    if !output.status.success() {
        bail!("Failed to run git archive");
    }

    let output = Command::new("tar")
        .current_dir(to)
        .arg("xf")
        .arg(&archive_file)
        .output()
        .context("Failed to run tar")?;

    if !output.status.success() {
        bail!("Failed to run tar");
    }

    Ok(())
}
