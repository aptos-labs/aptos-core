// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use aptos_framework::ReleaseTarget;
use std::{env::current_dir, fs, path::PathBuf};

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
                .join("aptos-experimental")
                .join("sources")
                .display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir
                .join("aptos-experimental")
                .join("Move.toml")
                .display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir
                .join("aptos-token-objects")
                .join("Move.toml")
                .display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir
                .join("aptos-token-objects")
                .join("sources")
                .display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir.join("aptos-token").join("sources").display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir.join("aptos-token").join("Move.toml").display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir
                .join("aptos-token-objects")
                .join("sources")
                .display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir
                .join("aptos-token-objects")
                .join("Move.toml")
                .display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir.join("aptos-framework").join("sources").display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir.join("aptos-framework").join("Move.toml").display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir.join("aptos-stdlib").join("sources").display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir.join("aptos-stdlib").join("Move.toml").display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir.join("move-stdlib").join("sources").display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            prev_dir.join("move-stdlib").join("Move.toml").display()
        );

        rebuild_framework()?;
    }

    Ok(())
}

/// For debug builds, specify `APTOS_FRAMEWORK_BUILD_PATH` at compile time to override framework's `head.mrb` path.
fn rebuild_framework() -> Result<()> {
    let default_head_mrb_path =
        PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR defined")).join("head.mrb");

    // need to use option_env! instead of `std::env::var()` to trigger recompilation step
    let custom_path_from_env = option_env!("APTOS_FRAMEWORK_BUILD_PATH");
    // target path is only override-able for debug builds
    let custom_framework_mrb_path = cfg!(debug_assertions)
        .then_some(custom_path_from_env)
        .flatten()
        .map(PathBuf::from);

    let target_path = match custom_framework_mrb_path {
        Some(custom_framework_mrb_path) => {
            // we need to create dummy file at `default_head_mrb_path` for later `include_bytes!` to succeed
            if !default_head_mrb_path.exists() {
                fs::File::create(default_head_mrb_path).expect("OUT_DIR should be writeable");
            }
            println!(
                "cargo::warning=APTOS_FRAMEWORK_BUILD_PATH is set, target framework path = {}",
                custom_framework_mrb_path.display()
            );
            if custom_framework_mrb_path.exists() {
                println!("cargo::warning=file already exists, skipping build");
                return Ok(());
            }
            custom_framework_mrb_path
        },
        None => default_head_mrb_path,
    };

    ReleaseTarget::Head
        .create_release(true, Some(target_path))
        .context("Failed to create release")
}
