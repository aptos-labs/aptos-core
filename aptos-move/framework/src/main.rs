// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use aptos_framework::{ReleaseOptions, ReleaseTarget};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[clap(name = "aptos-framework", author, version, propagate_version = true)]
enum Commands {
    /// Creates an Aptos framework release for the specified target.
    Release(StandardRelease),
    /// Allows to create a custom release package,
    Custom(CustomRelease),
    /// Rebuilds cached packages (head.mrb and SDK builder files).
    UpdateCachedPackages,
}

fn main() {
    let cmd: Commands = Commands::parse();
    let result = match cmd {
        Commands::Release(release) => release.execute(),
        Commands::Custom(custom) => custom.execute(),
        Commands::UpdateCachedPackages => update_cached_packages(),
    };
    if let Err(e) = result {
        eprintln!("error: {:#}", e);
        std::process::exit(1)
    }
}

// ========================
// Custom Release

#[derive(Debug, Parser)]
struct CustomRelease {
    #[clap(flatten)]
    options: ReleaseOptions,
}

impl CustomRelease {
    fn execute(self) -> anyhow::Result<()> {
        self.options.create_release()
    }
}

// ========================
// Standard Release

#[derive(Debug, Parser)]
struct StandardRelease {
    /// The release target. One of head, devnet, testnet, or mainnet. Notice the type
    /// of target determines what packages are included in the release. For example,
    /// some packages may be available in testnet, but aren't in mainnet.
    #[clap(long, default_value_t = ReleaseTarget::Head)]
    target: ReleaseTarget,

    /// Remove the source code from the release package to shrink its size.
    #[clap(long)]
    without_source_code: bool,
}

impl StandardRelease {
    fn execute(self) -> anyhow::Result<()> {
        self.target.create_release(!self.without_source_code, None)
    }
}

fn update_cached_packages() -> anyhow::Result<()> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = crate_dir.join("cached-packages/src/head.mrb");
    ReleaseTarget::Head.create_release(true, Some(output))
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Commands::command().debug_assert()
}
