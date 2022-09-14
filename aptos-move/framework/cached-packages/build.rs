// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use framework::ReleaseTarget;
use std::env::current_dir;
use std::path::PathBuf;

fn main() {
    // Set the below variable to skip the building step. This might be useful if the build
    // is broken so it can be debugged with the old outdated artifacts.
    if std::env::var("SKIP_FRAMEWORK_BUILD").is_err() {
        let current_dir = current_dir().expect("Should be able to get current dir");
        // Get the previous directory
        let mut prev_dir = current_dir;
        prev_dir.pop();
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
        ReleaseTarget::Head
            .create_release(Some(
                PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR defined")).join("head.mrb"),
            ))
            .expect("release build failed");
    }
}
