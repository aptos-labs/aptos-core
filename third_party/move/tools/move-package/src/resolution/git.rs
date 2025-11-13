use crate::source_package::parsed_manifest::PackageName;
use anyhow::bail;
use std::process::{Command, ExitStatus, Stdio};

pub(crate) fn fetch_new_dependency(
    git_url: &str,
    git_path: &str,
    git_rev: &str,
    dep_name: PackageName,
) -> anyhow::Result<()> {
    deep_clone_repo(git_url, git_path, dep_name)?;
    checkout_rev(git_path, git_rev, dep_name)?;
    Ok(())
}

pub(crate) fn fetch_new_dependency_shallow(
    git_url: &str,
    git_path: &str,
    git_rev: &str,
    dep_name: PackageName,
) -> anyhow::Result<()> {
    shallow_clone_repo(git_url, git_path, dep_name)?;

    shallow_fetch_latest_origin_rev(git_path, git_rev, dep_name)?;
    switch_to_fetched_rev(git_path, git_rev, dep_name)?;

    Ok(())
}

pub(crate) fn update_dependency(
    git_path: &str,
    git_rev: &str,
    dep_name: PackageName,
) -> anyhow::Result<()> {
    let status = fetch_latest_origin_rev(git_path, dep_name)?;
    if !status.success() {
        return Err(anyhow::anyhow!(
                            "Failed to fetch to latest Git state for package '{}', to skip set --skip-fetch-latest-git-deps | Exit status: {}",
                            dep_name,
                        status
                        ));
    }
    let status = Command::new("git")
        .args([
            "-C",
            git_path,
            "reset",
            "--hard",
            &format!("origin/{}", git_rev)
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| {
            anyhow::anyhow!(
                            "Failed to reset to latest Git state '{}' for package '{}', to skip set --skip-fetch-latest-git-deps",
                            git_rev,
                            dep_name
                        )
        })?;
    if !status.success() {
        return Err(anyhow::anyhow!(
                            "Failed to reset to latest Git state '{}' for package '{}', to skip set --skip-fetch-latest-git-deps | Exit status: {}",
                            git_rev,
                            dep_name,
                        status
                        ));
    }
    Ok(())
}

pub(crate) fn update_dependency_shallow(
    git_path: &str,
    git_rev: &str,
    dep_name: PackageName,
) -> anyhow::Result<()> {
    // If the current folder exists, do a fetch and reset to ensure that the branch
    // is up to date
    // NOTE: this means that you must run the package system with a working network connection
    let status = shallow_fetch_latest_origin_rev(git_path, git_rev, dep_name)?;
    if !status.success() {
        return Err(anyhow::anyhow!(
                            "Failed to fetch to latest Git state for package '{}', to skip set --skip-fetch-latest-git-deps | Exit status: {}",
                            dep_name,
                        status
                        ));
    }
    let status = switch_to_fetched_rev(git_path, git_rev, dep_name)?;
    if !status.success() {
        return Err(anyhow::anyhow!(
                            "Failed to reset to latest Git state '{}' for package '{}', to skip set --skip-fetch-latest-git-deps | Exit status: {}",
                            git_rev,
                            dep_name,
                        status
                        ));
    }
    Ok(())
}

// old version of initial git clone
fn deep_clone_repo(url: &str, repo_path: &str, dep_name: PackageName) -> anyhow::Result<()> {
    Command::new("git")
        .args(["clone", url, repo_path])
        .output()
        .map_err(|_| {
            anyhow::anyhow!("Failed to clone Git repository for package '{}'", dep_name)
        })?;
    Ok(())
}

fn shallow_clone_repo(url: &str, repo_path: &str, package_name: PackageName) -> anyhow::Result<()> {
    Command::new("git")
        .args(["clone", "--depth", "1", url, repo_path])
        .output()
        .map_err(|_| {
            anyhow::anyhow!(
                "Failed to clone Git repository for package '{}'",
                package_name
            )
        })?;
    Ok(())
}

// old version of initial git checkout
fn checkout_rev(repo_path: &str, git_rev: &str, dep_name: PackageName) -> anyhow::Result<()> {
    Command::new("git")
        .args(["-C", repo_path, "checkout", git_rev])
        .output()
        .map_err(|_| {
            anyhow::anyhow!(
                "Failed to checkout Git reference '{}' for package '{}'",
                git_rev,
                dep_name
            )
        })?;
    Ok(())
}

// old version of git fetch
fn fetch_latest_origin_rev(repo_path: &str, dep_name: PackageName) -> anyhow::Result<ExitStatus> {
    let mut cmd = Command::new("git");
    cmd.args(["-C", repo_path, "fetch", "origin"])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    cmd
        .status()
        .map_err(|_| {
            anyhow::anyhow!(
                "Failed to fetch latest Git state for package '{}', to skip set --skip-fetch-latest-git-deps",
                dep_name
            )
        })
}

fn switch_to_fetched_rev(
    repo_path: &str,
    git_rev: &str,
    dep_name: PackageName,
) -> anyhow::Result<ExitStatus> {
    let mut cmd = Command::new("git");
    cmd.args(["reset", "--hard", "FETCH_HEAD"])
        .current_dir(repo_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let output = cmd
        .output()
        .map_err(|_| {
            anyhow::anyhow!(
                "Failed to reset to latest Git state '{}' for package '{}', to skip set --skip-fetch-latest-git-deps",
                git_rev,
                dep_name
            )
        })?;
    Ok(output.status)
}

fn shallow_fetch_latest_origin_rev(
    repo_path: &str,
    git_rev: &str,
    dep_name: PackageName,
) -> anyhow::Result<ExitStatus> {
    let status = Command::new("git")
        .args(["fetch", "--depth", "1", "origin", git_rev])
        .current_dir(repo_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| {
            anyhow::anyhow!(
                "Failed to checkout Git reference '{}' for package '{}'",
                git_rev,
                dep_name
            )
        })?;
    Ok(status)
}

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

pub(crate) fn get_existing_rev(repo_path: &str, git_rev: &str) -> anyhow::Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", git_rev])
        .current_dir(repo_path)
        .output()?;
    let stdout = String::from_utf8(output.stdout)?;
    Ok(stdout.trim().to_string())
}

pub(crate) fn get_existing_tag(repo_path: &str, git_rev: &str) -> anyhow::Result<String> {
    let output = Command::new("git")
        .args(["tag", "--list", git_rev])
        .current_dir(repo_path)
        .output()?;
    let stdout = String::from_utf8(output.stdout)?;
    Ok(stdout.trim().to_string())
}
