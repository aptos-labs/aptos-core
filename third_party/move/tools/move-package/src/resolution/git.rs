// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::source_package::parsed_manifest::PackageName;
use anyhow::bail;
use std::process::{Command, Stdio};

pub(crate) fn confirm_git_available() -> anyhow::Result<()> {
    match Command::new("git").arg("--version").output() {
        Ok(_) => Ok(()),
        Err(e) => {
            if let std::io::ErrorKind::NotFound = e.kind() {
                bail!(
                    "git was not found, confirm you have git installed and it is on your PATH. \
                    Alternatively, skip with --skip-fetch-latest-git-deps"
                );
            } else {
                bail!(
                    "Unexpected error occurred when checking for presence of `git`: {:#}",
                    e
                );
            }
        },
    }
}

pub(crate) fn clone(url: &str, target_path: &str, dep_name: PackageName) -> anyhow::Result<()> {
    let status = Command::new("git")
        .args(["clone", url, target_path])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| {
            anyhow::anyhow!("Failed to clone Git repository for package '{}'", dep_name)
        })?;
    if !status.success() {
        return Err(anyhow::anyhow!(
            "Failed to clone Git repository for package '{}' | Exit status: {}",
            dep_name,
            status
        ));
    }
    Ok(())
}

pub(crate) fn shallow_clone(
    url: &str,
    target_path: &str,
    dep_name: PackageName,
) -> anyhow::Result<()> {
    let status = Command::new("git")
        .args(["clone", url, "--depth", "1", target_path])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| {
            anyhow::anyhow!("Failed to clone Git repository for package '{}'", dep_name)
        })?;
    if !status.success() {
        return Err(anyhow::anyhow!(
            "Failed to clone Git repository for package '{}' | Exit status: {}",
            dep_name,
            status
        ));
    }
    Ok(())
}

pub(crate) fn checkout(repo_path: &str, rev: &str, dep_name: PackageName) -> anyhow::Result<()> {
    let status = Command::new("git")
        .args(["-C", repo_path, "checkout", rev])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| {
            anyhow::anyhow!(
                "Failed to checkout Git reference '{}' for package '{}'",
                rev,
                dep_name
            )
        })?;
    if !status.success() {
        return Err(anyhow::anyhow!(
            "Failed to checkout Git reference '{}' for package '{}' | Exit status: {}",
            rev,
            dep_name,
            status
        ));
    }
    Ok(())
}

pub(crate) fn fetch_origin(repo_path: &str, dep_name: PackageName) -> anyhow::Result<()> {
    let status = Command::new("git")
        .args([
            "-C",
            repo_path,
            "fetch",
            "origin",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| {
            anyhow::anyhow!(
                "Failed to fetch latest Git state for package '{}', to skip set --skip-fetch-latest-git-deps",
                dep_name
            )
        })?;
    if !status.success() {
        return Err(anyhow::anyhow!(
            "Failed to fetch to latest Git state for package '{}', to skip set --skip-fetch-latest-git-deps | Exit status: {}",
            dep_name,
            status
        ));
    }
    Ok(())
}

pub(crate) fn shallow_fetch_origin(
    repo_path: &str,
    rev: &str,
    dep_name: PackageName,
) -> anyhow::Result<()> {
    let status = Command::new("git")
        .args([
            "-C",
            repo_path,
            "fetch",
            "--depth",
            "1",
            "origin",
            rev,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| {
            anyhow::anyhow!(
                "Failed to fetch latest Git state for package '{}', to skip set --skip-fetch-latest-git-deps",
                dep_name
            )
        })?;
    if !status.success() {
        return Err(anyhow::anyhow!(
            "Failed to fetch to latest Git state for package '{}', to skip set --skip-fetch-latest-git-deps | Exit status: {}",
            dep_name,
            status
        ));
    }
    Ok(())
}

pub(crate) fn reset_hard(repo_path: &str, rev: &str, dep_name: PackageName) -> anyhow::Result<()> {
    let status = Command::new("git")
        .args([
            "-C",
            repo_path,
            "reset",
            "--hard",
            &format!("origin/{}", rev)
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| {
            anyhow::anyhow!(
                "Failed to reset to latest Git state '{}' for package '{}', to skip set --skip-fetch-latest-git-deps",
                rev,
                dep_name
            )
        })?;
    if !status.success() {
        return Err(anyhow::anyhow!(
            "Failed to reset to latest Git state '{}' for package '{}', to skip set --skip-fetch-latest-git-deps | Exit status: {}",
            rev,
            dep_name,
            status
        ));
    }
    Ok(())
}

pub(crate) fn switch_to_fetched_rev(repo_path: &str, dep_name: PackageName) -> anyhow::Result<()> {
    let status = Command::new("git")
        .args([
            "-C",
            repo_path,
            "reset",
            "--hard",
            "FETCH_HEAD",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| {
            anyhow::anyhow!(
                "Failed to reset to FETCH_HEAD for package '{}', to skip set --skip-fetch-latest-git-deps",
                dep_name
            )
        })?;
    if !status.success() {
        return Err(anyhow::anyhow!(
            "Failed to reset to FETCH_HEAD for package '{}', to skip set --skip-fetch-latest-git-deps | Exit status: {}",
            dep_name,
            status
        ));
    }
    Ok(())
}

pub(crate) fn find_rev(repo_path: &str, rev: &str) -> anyhow::Result<String> {
    let output = Command::new("git")
        .args(["-C", repo_path, "rev-parse", "--verify", rev])
        .output()?;
    let status = output.status;
    if !status.success() {
        return Err(anyhow::anyhow!("Exit status: {}", status));
    }
    String::from_utf8(output.stdout)
        .map_err(|_| anyhow::anyhow!("Stdout contains non-UTF8 symbols"))
}

pub(crate) fn find_tag(repo_path: &str, rev: &str) -> anyhow::Result<String> {
    let output = Command::new("git")
        .args(["-C", repo_path, "tag", "--list", rev])
        .output()?;
    let status = output.status;
    if !status.success() {
        return Err(anyhow::anyhow!("Exit status: {}", status));
    }
    String::from_utf8(output.stdout)
        .map_err(|_| anyhow::anyhow!("Stdout contains non-UTF8 symbols"))
}
