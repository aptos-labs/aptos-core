// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, bail};
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_transaction_workloads_lib::{
    FILE_EXTENSION, MODULES_DIR, PACKAGE_METADATA_FILE, SCRIPTS_DIR, SCRIPT_FILE,
};
use clap::Parser;
use move_package::source_package::std_lib::StdVersion;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Root directory where packages are stored.
    #[clap(long)]
    packages_path: PathBuf,
    /// Root directory where prebuilt packages and their modules will be saved.
    #[clap(long)]
    prebuilt_packages_path: PathBuf,
    /// Enables dev mode when building packages. See [BuildOptions] for more details.
    #[clap(long)]
    dev: bool,
    /// If true, packages are compiled with latest (possibly unstable) version.
    #[clap(long)]
    latest_language: bool,
    /// If true, will use the local Aptos framework.
    #[clap(long)]
    pub use_local_std: bool,
}

/// Returns path to local Aptos framework.
fn get_local_framework_path() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .map(|p| p.join("aptos-move").join("framework"))
        .expect("Framework must exist")
        .to_string_lossy()
        .to_string()
}

/// Recursively traverses a directory to extract paths of all Move packages inside it.
fn visit(dir: &Path, package_paths: &mut Vec<PathBuf>) -> io::Result<()> {
    // Package found, do not recurse further.
    if dir.join("Move.toml").is_file() {
        package_paths.push(dir.to_path_buf());
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            visit(&entry.path(), package_paths)?;
        }
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let mut build_options = BuildOptions::move_2();
    if args.latest_language {
        build_options = build_options.set_latest_language();
    }
    build_options.dev = args.dev;

    if args.use_local_std {
        build_options.override_std = Some(StdVersion::Local(get_local_framework_path()));
    }

    let mut package_paths = vec![];
    visit(&args.packages_path, &mut package_paths)?;

    for package_path in package_paths {
        let package = BuiltPackage::build(package_path, build_options.clone())
            .map_err(|err| anyhow!("Failed to build a package: {err:?}"))?;

        let prebuilt_package_path = args.prebuilt_packages_path.join(package.name());
        fs::create_dir_all(&prebuilt_package_path)?;

        let package_metadata = package.extract_metadata()?;
        let metadata_bytes = bcs::to_bytes(&package_metadata)?;
        fs::write(
            prebuilt_package_path.join(PACKAGE_METADATA_FILE),
            &metadata_bytes,
        )?;

        let prebuilt_modules_path = prebuilt_package_path.join(MODULES_DIR);
        fs::create_dir_all(&prebuilt_modules_path)?;
        for (module_name, code) in package.module_code_iter() {
            fs::write(
                prebuilt_modules_path.join(format!("{module_name}.{FILE_EXTENSION}")),
                &code,
            )?;
        }

        let mut scripts = package.extract_script_code();
        if scripts.len() > 1 {
            bail!("For benchmarks, define 1 script per package to make name resolution easier")
        }

        if let Some(code) = scripts.pop() {
            let prebuilt_script_path = prebuilt_package_path.join(SCRIPTS_DIR);
            fs::create_dir_all(&prebuilt_script_path)?;
            fs::write(
                prebuilt_script_path.join(format!("{SCRIPT_FILE}.{FILE_EXTENSION}")),
                &code,
            )?;
        }
    }

    Ok(())
}
