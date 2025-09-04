// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use velor_transaction_generator_lib::{create_prebuilt_packages_bundle, PrebuiltPackageConfig};
use clap::Parser;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Root directory where packages are stored.
    #[clap(long)]
    packages_path: Vec<PathBuf>,
    /// Root directory where packages are stored.
    #[clap(long)]
    experimental_packages_path: Vec<PathBuf>,
    /// Root directory where prebuilt packages binary will be saved.
    #[clap(long)]
    prebuilt_packages_file_dir: PathBuf,
    /// Root directory where the Rust file to access prebuilt packages will be saved.
    #[clap(long)]
    prebuilt_packages_rust_dir: PathBuf,
    /// If true, uses local velor-framework from velor-core.
    #[clap(long)]
    use_local_std: bool,
}

/// Recursively traverses a directory to extract paths of all Move packages inside it.
fn visit(
    dir: &Path,
    config: &PrebuiltPackageConfig,
    package_paths: &mut Vec<(PathBuf, PrebuiltPackageConfig)>,
) -> io::Result<()> {
    // Package found, do not recurse further.
    if dir.join("Move.toml").is_file() {
        package_paths.push((dir.to_path_buf(), config.clone()));
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            visit(&entry.path(), config, package_paths)?;
        }
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let use_local_std = args.use_local_std;

    if args.packages_path.is_empty() && args.experimental_packages_path.is_empty() {
        bail!("At least one path pointing to packages directory should be provided");
    }
    let mut all_package_paths = vec![];
    for (package_path, latest_language) in args.packages_path.into_iter().map(|p| (p, false)).chain(
        args.experimental_packages_path
            .into_iter()
            .map(|p| (p, true)),
    ) {
        let config = PrebuiltPackageConfig {
            latest_language,
            use_local_std,
        };
        visit(&package_path, &config, &mut all_package_paths)?;
    }

    let output_file = args.prebuilt_packages_rust_dir.join("prebuilt_packages.rs");
    create_prebuilt_packages_bundle(
        args.prebuilt_packages_file_dir,
        all_package_paths,
        output_file,
    )?;
    Ok(())
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
