// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{anyhow, Context, Result};
use clap::Args;
use self_update::{
    backends::github::{ReleaseList, Update},
    version::bump_is_greater,
};

#[derive(Args, Debug, Clone)]
pub struct UpdateArgs {
    /// Check for updates without downloading.
    #[arg(long)]
    pub check: bool,

    #[arg(long, default_value = "aptos-labs", hide = true)]
    pub repo_owner: String,

    #[arg(long, default_value = "aptos-ai", hide = true)]
    pub repo_name: String,
}

/// `bump_is_greater` only handles `x.y.z`; strip suffixes it can't parse.
pub(crate) fn strip_prerelease(v: &str) -> &str {
    if let Some(stem) = v.strip_suffix(".beta").or_else(|| v.strip_suffix(".rc")) {
        stem
    } else {
        v
    }
}

pub fn run(args: &UpdateArgs) -> Result<()> {
    let releases = ReleaseList::configure()
        .repo_owner(&args.repo_owner)
        .repo_name(&args.repo_name)
        .build()
        .context("failed to configure release list")?
        .fetch()
        .context("failed to fetch releases from GitHub")?;

    let latest = releases
        .into_iter()
        .find(|r| r.version.starts_with("move-flow-v"))
        .ok_or_else(|| {
            anyhow!(
                "no move-flow release found in {}/{}",
                args.repo_owner,
                args.repo_name
            )
        })?;

    let latest_version = &latest.version["move-flow-v".len()..];
    let current_version = env!("CARGO_PKG_VERSION");
    let needs_update = bump_is_greater(current_version, strip_prerelease(latest_version))
        .context("failed to compare versions")?;

    if args.check {
        if needs_update {
            println!("Update available: v{latest_version}");
        } else {
            println!("Already up to date (v{current_version})");
        }
        return Ok(());
    }

    if !needs_update {
        println!("Already up to date (v{current_version})");
        return Ok(());
    }

    Update::configure()
        .repo_owner(&args.repo_owner)
        .repo_name(&args.repo_name)
        .bin_name("move-flow")
        .current_version(current_version)
        .target_version_tag(&latest.version)
        .no_confirm(true)
        .build()
        .context("failed to configure updater")?
        .update()
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("permission denied") || msg.contains("Access is denied") {
                anyhow!("cannot replace binary: permission denied (try sudo)")
            } else if msg.contains("could not find binary") || msg.contains("no asset") {
                anyhow!(
                    "no release asset for the current platform — download manually from \
                     https://github.com/{}/{}/releases",
                    args.repo_owner,
                    args.repo_name
                )
            } else {
                anyhow!("update failed: {e:#}")
            }
        })?;

    println!("Updated move-flow v{current_version} → v{latest_version}");
    Ok(())
}
