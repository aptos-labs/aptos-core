// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{BUILD_PROFILE, PATH_CRATE_ROOT};
use anyhow::{bail, Result};
use once_cell::sync::Lazy;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

const RUN_APTOS_P2P: &str = "run-aptos-p2p";

static PATH_BIN_RUN_APTOS_P2P: Lazy<PathBuf> = Lazy::new(|| {
    PATH_CRATE_ROOT
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target")
        .join(BUILD_PROFILE)
        .join(RUN_APTOS_P2P)
});

fn run_aptos_p2p() -> Result<()> {
    println!("Profiling Aptos VM...");

    let log_path = Path::join(&PATH_CRATE_ROOT, "p2p.log");
    let annotation_path = Path::join(&PATH_CRATE_ROOT, "p2p.txt");

    crate::valgrind::profile_with_valgrind(
        [&*PATH_BIN_RUN_APTOS_P2P],
        &[],
        log_path,
        annotation_path,
    )?;

    Ok(())
}

fn build_binaries() -> Result<()> {
    let status = Command::new("cargo")
        .arg("build")
        .arg("--profile")
        .arg(BUILD_PROFILE)
        .arg("--features")
        .arg("move-vm-types/force-inline move-vm-runtime/force-inline")
        .arg("-p")
        .arg("aptos-vm-profiling")
        .arg("--bin")
        .arg(RUN_APTOS_P2P)
        .status()?;

    if !status.success() {
        bail!("Failed to compile {}", RUN_APTOS_P2P);
    }

    Ok(())
}

pub fn run() -> Result<()> {
    build_binaries()?;
    run_aptos_p2p()?;

    Ok(())
}
