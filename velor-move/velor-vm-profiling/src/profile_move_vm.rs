// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{BUILD_PROFILE, PATH_CRATE_ROOT};
use anyhow::{bail, Result};
use once_cell::sync::Lazy;
use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};

const RUN_MOVE: &str = "run-move";

static PATH_BIN_RUN_MOVE: Lazy<PathBuf> = Lazy::new(|| {
    PATH_CRATE_ROOT
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target")
        .join(BUILD_PROFILE)
        .join(RUN_MOVE)
});

/// Profile the Move VM using callgrind and convert the output into an easy-to-read form using
/// callgrind_annotate.
fn profile_move(path: impl AsRef<Path>, bin_mod_time: SystemTime, regenerate: bool) -> Result<()> {
    let path = path.as_ref();

    let log_path = path.with_extension("log");
    let annotation_path = path.with_extension("txt");

    // Skip this program if the result is newer than the source that generated it.
    // This is done by comparing the modification times.
    let should_skip = if regenerate {
        false
    } else {
        let src_time = fs::metadata(path)?.modified()?;
        match fs::metadata(&annotation_path) {
            Ok(metadata) => {
                let annotation_time = metadata.modified()?;
                src_time < annotation_time && bin_mod_time < annotation_time
            },
            Err(err) => match err.kind() {
                io::ErrorKind::NotFound => false,
                _ => return Err(err.into()),
            },
        }
    };
    if should_skip {
        println!("Skipping {}", path.file_name().unwrap().to_string_lossy());
        return Ok(());
    } else {
        println!(
            "Profiling {}...",
            path.file_name().unwrap().to_string_lossy()
        );
    }

    crate::valgrind::profile_with_valgrind(
        [&*PATH_BIN_RUN_MOVE, path],
        &[],
        log_path,
        annotation_path,
    )?;

    Ok(())
}

/// Profile all Move programs in the `move` directory.
fn profile_move_snippets(regenerate_all: bool) -> Result<()> {
    println!("Profiling Move VM...");

    let root = Path::join(&PATH_CRATE_ROOT, "move");
    let pat = format!("{}/**/*.mvir", root.to_string_lossy());

    let bin_mod_time = fs::metadata(&*PATH_BIN_RUN_MOVE)?.modified()?;

    for entry in glob::glob(&pat)? {
        let path = entry?;

        profile_move(path, bin_mod_time, regenerate_all)?;
    }

    Ok(())
}

/// Build all binaries using the right profile.
fn build_binaries() -> Result<()> {
    let status = Command::new("cargo")
        .arg("build")
        .arg("--profile")
        .arg(BUILD_PROFILE)
        .arg("-p")
        .arg("velor-vm-profiling")
        .arg("--bin")
        .arg(RUN_MOVE)
        .status()?;

    if !status.success() {
        bail!("Failed to compile {}", RUN_MOVE);
    }

    Ok(())
}

pub(crate) fn run(regenerate_all: bool) -> Result<()> {
    build_binaries()?;
    profile_move_snippets(regenerate_all)?;

    Ok(())
}
