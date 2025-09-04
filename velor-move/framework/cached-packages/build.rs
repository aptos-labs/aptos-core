// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use velor_framework::ReleaseTarget;
use std::{env::current_dir, path::PathBuf};

fn main() -> Result<()> {
    // Set the below variable to skip the building step. This might be useful if the build
    // is broken so it can be debugged with the old outdated artifacts.
    if std::env::var("SKIP_FRAMEWORK_BUILD").is_err() {
        let current_dir = current_dir().expect("Should be able to get current dir");
        // Get the previous directory
        let mut prev_dir = current_dir;
        prev_dir.pop();
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir
                .join("velor-experimental")
                .join("sources")
                .display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir
                .join("velor-experimental")
                .join("Move.toml")
                .display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir
                .join("velor-token-objects")
                .join("Move.toml")
                .display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir
                .join("velor-token-objects")
                .join("sources")
                .display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir.join("velor-token").join("sources").display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir.join("velor-token").join("Move.toml").display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir
                .join("velor-token-objects")
                .join("sources")
                .display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir
                .join("velor-token-objects")
                .join("Move.toml")
                .display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir.join("velor-framework").join("sources").display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir.join("velor-framework").join("Move.toml").display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir.join("velor-stdlib").join("sources").display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir.join("velor-stdlib").join("Move.toml").display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir.join("move-stdlib").join("sources").display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir.join("move-stdlib").join("Move.toml").display()
        );

        let path =
            PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR defined")).join("head.mrb");

        ReleaseTarget::Head
            .create_release(true, Some(path))
            .context("Failed to create release")?;
    }

    Ok(())
}
