// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{Context, Result};
use aptos_framework::ReleaseTarget;
use std::{env::current_dir, path::PathBuf};

fn main() -> Result<()> {
    // Set the below variable to skip the building step. This might be useful if the build
    // is broken so it can be debugged with the old outdated artifacts.
    if std::env::var("SKIP_FRAMEWORK_BUILD").is_err() {
        let current_dir = current_dir().expect("Should be able to get current dir");
        // Get the previous directory
        let mut prev_dir = current_dir;
        prev_dir.pop();

        for package in [
            "aptos-experimental",
            "aptos-trading",
            "aptos-token-objects",
            "aptos-token",
            "aptos-framework",
            "aptos-stdlib",
            "move-stdlib",
        ] {
            println!(
                "cargo:rerun-if-changed={}",
                prev_dir.join(package).join("sources").display()
            );
            println!(
                "cargo:rerun-if-changed={}",
                prev_dir.join(package).join("Move.toml").display()
            );
        }

        let path =
            PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR defined")).join("head.mrb");

        ReleaseTarget::Head
            .create_release(true, Some(path))
            .context("Failed to create release")?;
    }

    Ok(())
}
