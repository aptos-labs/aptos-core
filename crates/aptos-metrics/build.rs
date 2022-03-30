// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::{env, path::Path, process::Command};

const GIT_INDEX: &str = "../../.git/index";

/// Save revision info to environment variable
fn main() {
    if Path::new(GIT_INDEX).exists() {
        println!("cargo:rerun-if-changed={}", GIT_INDEX);
    }
    if env::var("GIT_REV").is_err() {
        let output = Command::new("git")
            .args(&["rev-parse", "--short", "HEAD"])
            .output()
            .unwrap();
        let git_rev = String::from_utf8(output.stdout).unwrap();
        println!("cargo:rustc-env=GIT_REV={}", git_rev);
    }
}
