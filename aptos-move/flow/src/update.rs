// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{anyhow, Context, Result};
use clap::Args;
use self_update::{backends::github::Update, errors::Error as UpdateError};

const TAG_PREFIX: &str = "move-flow-";

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

pub fn run(args: &UpdateArgs) -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");
    let mut builder = Update::configure();
    builder
        .repo_owner(&args.repo_owner)
        .repo_name(&args.repo_name)
        .bin_name("move-flow")
        .tag_prefix(TAG_PREFIX)
        .current_version(current_version)
        .no_confirm(true);

    // Only releases tagged with `TAG_PREFIX` and newer than the current version are
    // returned, newest first.
    let newer_releases = builder
        .build()
        .context("failed to configure updater")?
        .get_newer_releases()
        .context("failed to fetch releases from GitHub")?;

    let Some(latest) = newer_releases.latest() else {
        println!("Already up to date (v{current_version})");
        return Ok(());
    };
    let latest_version = latest.version();

    if args.check {
        println!("Update available: v{latest_version}");
        return Ok(());
    }

    builder
        .release_tag(format!("{TAG_PREFIX}v{latest_version}"))
        .build()
        .context("failed to configure updater")?
        .update()
        .map_err(|e| match e {
            UpdateError::InstallPathNotWritable { .. } => {
                anyhow!("cannot replace binary: permission denied (try sudo)")
            },
            UpdateError::NoReleaseFound { .. } => anyhow!(
                "no release asset for the current platform — download manually from \
                 https://github.com/{}/{}/releases",
                args.repo_owner,
                args.repo_name
            ),
            e => anyhow!("update failed: {e:#}"),
        })?;

    println!("Updated move-flow v{current_version} → v{latest_version}");
    Ok(())
}
