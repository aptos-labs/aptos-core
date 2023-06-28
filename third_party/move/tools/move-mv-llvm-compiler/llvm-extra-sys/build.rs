// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Context};
use std::{path::PathBuf, process::Command};

fn main() -> anyhow::Result<()> {
    // Get the path to llvm-config from the llvm-sys crate
    let llvm_config_path = std::env::var("DEP_LLVM_15_CONFIG_PATH")
        .context("DEP_LLVM_15_CONFIG_PATH not set")
        .context("this probably means the llvm-sys build failed")?;
    let llvm_config_path = PathBuf::from(llvm_config_path);
    let llvm_config = LlvmConfig::new(llvm_config_path);
    let llvm_include_dir = llvm_config.include_dir()?;
    let cxxflags = llvm_config.cxxflags()?;
    let cxxflags = split_flags(&cxxflags);

    let mut cc = cc::Build::new();

    for flag in cxxflags {
        cc.flag(&flag);
    }

    cc.cpp(true)
        .warnings(false)
        .include(llvm_include_dir)
        .file("src/llvm-extra.cpp")
        .compile("llvm-extra");

    println!("cargo:rerun-if-changed=src/llvm-extra.cpp");

    Ok(())
}

fn split_flags(flags: &str) -> Vec<String> {
    flags
        .split_ascii_whitespace()
        .map(|slice| slice.to_string())
        .collect()
}

struct LlvmConfig {
    path: PathBuf,
}

impl LlvmConfig {
    fn new(path: PathBuf) -> LlvmConfig {
        LlvmConfig { path }
    }

    fn include_dir(&self) -> anyhow::Result<PathBuf> {
        let out = Command::new(&self.path).arg("--includedir").output()?;

        if !out.status.success() {
            bail!("llvm-config returned non-zero exit code");
        }

        Ok(PathBuf::from(String::from_utf8(out.stdout)?.trim()))
    }

    fn cxxflags(&self) -> anyhow::Result<String> {
        let out = Command::new(&self.path).arg("--cxxflags").output()?;

        if !out.status.success() {
            bail!("llvm-config returned non-zero exit code");
        }

        Ok(String::from_utf8(out.stdout)?)
    }
}
