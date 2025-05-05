// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{BUILD_PROFILE, PATH_CRATE_ROOT};
use anyhow::{bail, Result};
use aptos_transaction_simulation::GENESIS_CHANGE_SET_HEAD;
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

    let genesis_blob = bcs::to_bytes(GENESIS_CHANGE_SET_HEAD.write_set())?;

    let log_path = Path::join(&PATH_CRATE_ROOT, "p2p.log");
    let annotation_path = Path::join(&PATH_CRATE_ROOT, "p2p.txt");

    crate::valgrind::profile_with_valgrind(
        [&*PATH_BIN_RUN_APTOS_P2P],
        &genesis_blob,
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
